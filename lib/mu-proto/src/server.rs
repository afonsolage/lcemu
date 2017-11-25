extern crate tokio_core;
extern crate tokio_io;

use futures::prelude::*;
use futures::task::Task;
use futures::task;
use futures::sync::mpsc as f_mpsc;

use self::tokio_core::net::{TcpListener, TcpStream};
use self::tokio_core::reactor::Handle;
use self::tokio_io::io::ReadHalf;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::io;
use std::convert::From;
use std::collections::HashMap;

use super::tcp_session::{TcpSessionError, TcpSession, TcpSessionReader};
use super::packet::MuPacket;

pub enum NetworkError {
    InvalidAddress,
    TcpBindError,
    Disconnected,
    TxFailed,
    IoErrror,
    SessionError,
    SessionNotFound,
    SessionSendError,
    SessionDisconnected,
}

impl From<io::Error> for NetworkError {
    fn from(err: io::Error) -> Self {
        match err {
            _ => NetworkError::IoErrror,
        }
    }
}


impl From<TcpSessionError> for NetworkError {
    fn from(err: TcpSessionError) -> Self {
        match err {
            _ => NetworkError::SessionError,
        }
    }
}

type ClientsMap = Arc<Mutex<HashMap<u32, f_mpsc::Sender<MuPacket>>>>;

pub struct Server {
    handle: Handle,
    evt_rx: Receiver<NetworkEvent>,
    evt_tx: Sender<NetworkEvent>,
    task: Arc<Mutex<Option<Task>>>,
    clients: ClientsMap,
}

#[derive(Debug)]
pub enum NetworkEvent {
    ClientConnected(u32),
    ClientDisconnected(u32),
    ClientPacket((u32, MuPacket)),
}

static SESSION_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

impl<'a> Server {
    pub fn new(handle: Handle) -> Server {
        let (tx, rx): (Sender<NetworkEvent>, Receiver<NetworkEvent>) = mpsc::channel();

        Server {
            handle: handle,
            evt_tx: tx,
            evt_rx: rx,
            task: Arc::new(Mutex::new(None)),
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start_tcp(&mut self, listen_addr: &'a str, port: u16) -> Result<(), NetworkError> {
        let handle = self.handle.clone();
        let addr = match format!("{}:{}", listen_addr, port).parse() {
            Err(_) => return Err(NetworkError::InvalidAddress),
            Ok(addr) => addr,
        };

        let listener = match TcpListener::bind(&addr, &handle) {
            Err(_) => return Err(NetworkError::TcpBindError),
            Ok(r) => r,
        };

        handle.spawn(
            Server::handle_tcp_connections(
                listener,
                self.evt_tx.clone(),
                self.task.clone(),
                self.handle.clone(),
                self.clients.clone(),
            ).then(|_| Ok(())),
        );

        Ok(())
    }

    pub fn send(&self, id: u32, pkt: MuPacket) -> Result<(), NetworkError> {
        let mut map = self.clients.lock().unwrap();

        if let Some(tx) = map.get_mut(&id) {
            match tx.try_send(pkt) {
                Ok(_) => Ok(()),
                Err(err) => {
                    if err.is_disconnected() {
                        Err(NetworkError::SessionDisconnected)
                    } else {
                        Err(NetworkError::SessionSendError)
                    }
                }
            }
        } else {
            Err(NetworkError::SessionNotFound)
        }
    }

    #[async]
    fn handle_tcp_connections(
        listener: TcpListener,
        tx: Sender<NetworkEvent>,
        task_shr: Arc<Mutex<Option<Task>>>,
        handle: Handle,
        clients: ClientsMap,
    ) -> Result<(), NetworkError> {

        #[async]
        for (stream, _peer_addr) in listener.incoming() {
            let id = SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed) as u32;
            let (ssn_reader, ssn_writer) = TcpSession::new(stream, id);

            if let Ok(_) = tx.send(NetworkEvent::ClientConnected(id)) {
                if let Some(ref t) = *task_shr.lock().unwrap() {
                    t.notify();
                }
            } else {
                println!("Failed to send ClientConnected event.");
                return Ok(());
            }

            {
                let (tx, rx): (f_mpsc::Sender<MuPacket>,
                               f_mpsc::Receiver<MuPacket>) = f_mpsc::channel(100);

                let mut map = clients.lock().unwrap();

                map.insert(id, tx);

                let ft = ssn_writer
                    .send_all(rx.map_err(|_| TcpSessionError::TcpStreamWrite))
                    .then(|_| Ok(()));

                handle.spawn(ft);
            }

            handle.spawn(
                Server::handle_tcp_session(
                    ssn_reader,
                    tx.clone(),
                    task_shr.clone(),
                    clients.clone(),
                ).then(|_| Ok(())),
            )
        }

        Ok(())
    }

    #[async]
    fn handle_tcp_session(
        ssn_reader: TcpSessionReader<ReadHalf<TcpStream>>,
        tx: Sender<NetworkEvent>,
        task_shr: Arc<Mutex<Option<Task>>>,
        clients: ClientsMap,
    ) -> Result<(), NetworkError> {

        let task_shr_cs = task_shr.clone();
        let session_id = ssn_reader.id;

        #[async]
        for packet in ssn_reader {
            let task = task_shr_cs.lock().unwrap();
            if let Ok(_) = tx.send(NetworkEvent::ClientPacket((session_id, packet))) {
                if let Some(ref t) = *task {
                    t.notify();
                }
            } else {
                println!("Failed to send ClientDisconnected event.");
                return Ok(());
            }
        }

        {
            let mut map = clients.lock().unwrap();
            map.remove(&session_id);
        }

        let task = task_shr.lock().unwrap();
        if let Ok(_) = tx.send(NetworkEvent::ClientDisconnected(session_id)) {
            if let Some(ref t) = *task {
                t.notify();
            }
        } else {
            println!("Failed to send ClientDisconnected event.");
            return Ok(());
        }

        Ok(())
    }
}

impl Stream for Server {
    type Item = NetworkEvent;
    type Error = NetworkError;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        {
            let mut task = self.task.lock().unwrap();
            *task = Some(task::current());
        }

        match self.evt_rx.try_recv() {
            Err(mpsc::TryRecvError::Empty) => Ok(Async::NotReady),
            Err(mpsc::TryRecvError::Disconnected) => Ok(Async::Ready(None)),
            Ok(evt) => Ok(Async::Ready(Some(evt))),
        }
    }
}
