extern crate tokio_core;

use futures::prelude::*;
use futures::task::Task;
use futures::task;

use self::tokio_core::net::{TcpListener, TcpStream};
use self::tokio_core::reactor::{Core, Handle};
use super::tcp_session::{TcpSessionError, TcpSession};

use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::io;
use std::convert::From;
use std::sync::{Arc, Mutex};
use std::thread;

use super::packet::MuPacket;

pub enum NetworkError {
    InvalidAddress,
    TcpBindError,
    Disconnected,
    TxFailed,
    IoErrror,
    SessionError,
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

pub struct Server {
    handle: Handle,
    evt_rx: Receiver<NetworkEvent>,
    evt_tx: Sender<NetworkEvent>,
    task: Arc<Mutex<Option<Task>>>,
}

#[derive(Debug)]
pub enum NetworkEvent {
    ClientConnected(u32),
    ClientDisconnected(u32),
    ClientPacket(MuPacket),
}

impl<'a> Server {
    pub fn new(handle: Handle) -> Server {
        let (tx, rx): (Sender<NetworkEvent>, Receiver<NetworkEvent>) = mpsc::channel();

        Server {
            handle: handle,
            evt_tx: tx,
            evt_rx: rx,
            task: Arc::new(Mutex::new(None)),
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
            Server::handle_connections(
                listener,
                self.evt_tx.clone(),
                self.task.clone(),
                self.handle.clone(),
            ).then(|_| Ok(())),
        );

        Ok(())
    }

    #[async]
    fn handle_connections(
        listener: TcpListener,
        tx: Sender<NetworkEvent>,
        task_shr: Arc<Mutex<Option<Task>>>,
        handle: Handle,
    ) -> Result<(), NetworkError> {

        #[async]
        for (stream, _peer_addr) in listener.incoming() {
            let session = TcpSession::new(stream);

            if let Ok(_) = tx.send(NetworkEvent::ClientConnected(1)) {
                if let Some(ref t) = *task_shr.lock().unwrap() {
                    t.notify();
                }
            } else {
                println!("Failed to send ClientConnected event.");
                return Ok(());
            }

            handle.spawn(
                Server::handle_session(session, tx.clone(), task_shr.clone()).then(|_| Ok(())),
            )
        }

        Ok(())
    }

    #[async]
    fn handle_session(
        session: TcpSession<TcpStream>,
        tx: Sender<NetworkEvent>,
        task_shr: Arc<Mutex<Option<Task>>>,
    ) -> Result<(), NetworkError> {
        #[async]
        for packet in session {
            println!("Received packet: {}", packet);
        }

        let task = task_shr.lock().unwrap();

        if let Ok(_) = tx.send(NetworkEvent::ClientDisconnected(1)) {
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
