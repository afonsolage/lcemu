extern crate tokio_core;
extern crate tokio_io;

use futures::prelude::*;
use futures::task::Task;
use futures::task;
use futures::sync::mpsc as f_mpsc;

use failure::Error;

use self::tokio_core::net::{TcpListener, TcpStream};
use self::tokio_core::reactor::Handle;
use self::tokio_io::io::ReadHalf;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::convert::From;
use std::collections::HashMap;
use std::net::AddrParseError;

use super::tcp_session::{TcpSessionError, TcpSession, TcpSessionReader};
use super::packet::MuPacket;

static SESSION_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Fail)]
pub enum NetworkError {
    #[fail(display = "You shouldn't see this.")]
    None,
    #[fail(display = "Invalid network address provided.")]
    InvalidAddress,
    #[fail(display = "Failed to bind on TCP address and port")]
    TcpBindError,
    #[fail(display = "Endpoint was disconnected")]
    Disconnected,
    #[fail(display = "Failed to write on TX channel")]
    TxFailed,
    #[fail(display = "General IO Error")]
    IoErrror,
    #[fail(display = "Given session was not found")]
    SessionNotFound,
    #[fail(display = "Failed to send packet to given session")]
    SessionSendError,
    #[fail(display = "Session was disconnected")]
    SessionDisconnected,
}

impl From<AddrParseError> for NetworkError {
    fn from(err: AddrParseError) -> NetworkError {
        NetworkError::InvalidAddress
    }
}

#[derive(Debug)]
pub enum NetworkEvent {
    ClientConnected(SessionRef),
    ClientPacket((SessionRef, MuPacket)),
    ClientDisconnected(u32),
}

#[derive(Clone, Debug)]
pub struct SessionRef {
    pub id: u32,
    tx: f_mpsc::Sender<MuPacket>,
}

impl SessionRef {
    pub fn new(id: u32, tx: f_mpsc::Sender<MuPacket>) -> Self {
        SessionRef { id: id, tx: tx }
    }

    pub fn close(&mut self) -> Result<(), NetworkError> {
        self.send(MuPacket::empty())
    }

    pub fn send(&mut self, pkt: MuPacket) -> Result<(), NetworkError> {
        match self.tx.try_send(pkt) {
            Ok(_) => Ok(()),
            Err(err) => {
                if err.is_disconnected() {
                    Err(NetworkError::SessionDisconnected)
                } else {
                    Err(NetworkError::SessionSendError)
                }
            }
        }
    }
}

type ClientsMap = Arc<Mutex<HashMap<u32, SessionRef>>>;

pub struct Server {
    handle: Handle,
    evt_rx: Receiver<NetworkEvent>,
    evt_tx: Sender<NetworkEvent>,
    task: Arc<Mutex<Option<Task>>>,
    clients: ClientsMap,
}

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

    pub fn send(&self, id: u32, pkt: MuPacket) -> Result<(), NetworkError> {
        let mut map = self.clients.lock().unwrap();

        if let Some(s_ref) = map.get_mut(&id) {
            s_ref.send(pkt)
        } else {
            Err(NetworkError::SessionNotFound)
        }
    }

    pub fn start_tcp(&mut self, listen_addr: &'a str, port: u16) -> Result<(), NetworkError> {
        let handle = self.handle.clone();
        let addr = format!("{}:{}", listen_addr, port).parse()?;

        println!("Binding TCP on {:?}", addr);

        let listener = match TcpListener::bind(&addr, &handle) {
            Err(_) => return Err(NetworkError::TcpBindError),
            Ok(r) => r,
        };

        handle.spawn(
            Server::handle_tcp_connections(
                listener,
                self.evt_tx.clone(),
                Arc::clone(&self.task),
                self.handle.clone(),
                Arc::clone(&self.clients),
            ).then(|_| Ok(())),
        );

        Ok(())
    }


    #[async]
    fn handle_tcp_connections(
        listener: TcpListener,
        tx: Sender<NetworkEvent>,
        task_shr: Arc<Mutex<Option<Task>>>,
        handle: Handle,
        clients: ClientsMap,
    ) -> Result<(), Error> {

        #[async]
        for (stream, _peer_addr) in listener.incoming() {
            let id = SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed) as u32;
            let (ssn_reader, ssn_writer) = TcpSession::new_pair(stream, id);
            let (s_tx, s_rx): (f_mpsc::Sender<MuPacket>, f_mpsc::Receiver<MuPacket>) =
                f_mpsc::channel(100);

            let s_ref = SessionRef::new(id, s_tx.clone());

            {
                let mut map = clients.lock().unwrap();
                map.insert(id, s_ref.clone());
            }

            if tx.send(NetworkEvent::ClientConnected(s_ref.clone()))
                .is_ok()
            {
                if let Some(ref t) = *task_shr.lock().unwrap() {
                    t.notify();
                }
            } else {
                println!("Failed to send ClientConnected event.");
                return Ok(());
            }



            let ft = ssn_writer
                .send_all(s_rx.map_err(|_| TcpSessionError::TcpStreamWrite))
                .then(|_| Ok(()));

            handle.spawn(ft);

            handle.spawn(
                Server::handle_tcp_session(
                    ssn_reader,
                    tx.clone(),
                    Arc::clone(&task_shr),
                    Arc::clone(&clients),
                    s_ref.clone(),
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
        s_ref: SessionRef,
    ) -> Result<(), Error> {

        let task_shr_cs = Arc::clone(&task_shr);
        let s_ref_cj = s_ref.clone();

        #[async]
        for packet in ssn_reader {
            let task = task_shr_cs.lock().unwrap();
            if tx.send(NetworkEvent::ClientPacket((s_ref_cj.clone(), packet)))
                .is_ok()
            {
                if let Some(ref t) = *task {
                    t.notify();
                }
            } else {
                println!("Failed to send ClientDisconnected event.");
                return Ok(());
            }
        }

        let session_id = s_ref.id;

        {
            let mut map = clients.lock().unwrap();
            map.remove(&session_id);
        }

        let task = task_shr.lock().unwrap();
        if tx.send(NetworkEvent::ClientDisconnected(session_id))
            .is_ok()
        {
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
    type Error = Error;
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
