extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_timer;

use futures::prelude::*;
use futures::task::Task;
use futures::task;
use futures::sync::mpsc as f_mpsc;

use failure::Error;

use self::tokio_core::net::{TcpListener, TcpStream};
use self::tokio_core::reactor::Handle;
use self::tokio_io::io::ReadHalf;

use self::tokio_timer::{Timer, TimerError};

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::convert::From;
use std::collections::HashMap;
use std::net::{AddrParseError, SocketAddr};
use std::hash::{Hash, Hasher};
use std::io;
use std::time::Duration;

use super::tcp_session::{TcpSession, TcpSessionError, TcpSessionReader};
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
    #[fail(display = "Failed to execute a internal timer")]
    InternalTimerError,
}

impl From<AddrParseError> for NetworkError {
    fn from(_err: AddrParseError) -> NetworkError {
        NetworkError::InvalidAddress
    }
}

impl From<TimerError> for NetworkError {
    fn from(_err: TimerError) -> NetworkError {
        NetworkError::InternalTimerError
    }
}

#[derive(Debug)]
pub enum NetworkEvent {
    ClientConnected(SessionRef),
    ClientDisconnected((u32, u8)),
    ClientPacket((SessionRef, MuPacket)),
}

#[derive(Clone, Debug)]
pub struct SessionRef {
    pub id: u32,
    pub kind: u8,
    tx: f_mpsc::Sender<MuPacket>,
    addr: SocketAddr,
}

impl SessionRef {
    pub fn new(id: u32, kind: u8, tx: f_mpsc::Sender<MuPacket>, addr: SocketAddr) -> Self {
        SessionRef {
            id: id,
            kind: kind,
            tx: tx,
            addr: addr,
        }
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

impl Hash for SessionRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for SessionRef {
    fn eq(&self, other: &SessionRef) -> bool {
        self.id == other.id
    }
}
impl Eq for SessionRef {}

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

    pub fn connect_to(&mut self, listen_addr: &'a str, port: u16, kind: u8) -> Result<(), Error> {
        let handle = self.handle.clone();
        let addr = format!("{}:{}", listen_addr, port).parse()?;

        let tx = self.evt_tx.clone();
        let task = Arc::clone(&self.task);
        let clients = Arc::clone(&self.clients);
        let handle_cj = self.handle.clone();

        let ft = Server::try_connect(tx, kind, handle_cj, clients, addr, task).then(|_| Ok(()));

        handle.spawn(ft);

        Ok(())
    }

    #[async]
    fn try_connect(
        tx: Sender<NetworkEvent>,
        kind: u8,
        handle: Handle,
        clients: ClientsMap,
        addr: SocketAddr,
        task: Arc<Mutex<Option<Task>>>,
    ) -> Result<(), Error> {
        loop {
            if let Ok(stream) = await!(TcpStream::connect(&addr, &handle)) {
                break await!(Server::handle_stream(
                    stream,
                    kind,
                    tx,
                    task,
                    handle.clone(),
                    clients,
                    addr,
                    true,
                ));
            }
            println!("Failed to connect to {:?}. Retrying...", addr);
            await!(Timer::default().sleep(Duration::from_secs(1)))?;
        }
    }


    pub fn start_tcp(&mut self, listen_addr: &'a str, port: u16, kind: u8) -> Result<(), Error> {
        let handle = self.handle.clone();
        let addr = format!("{}:{}", listen_addr, port).parse()?;

        println!("Binding TCP on {:?}", addr);

        let listener = match TcpListener::bind(&addr, &handle) {
            Err(_) => return Err(NetworkError::TcpBindError)?,
            Ok(r) => r,
        };

        handle.spawn(
            Server::handle_listener(
                listener,
                self.evt_tx.clone(),
                Arc::clone(&self.task),
                self.handle.clone(),
                Arc::clone(&self.clients),
                kind,
            ).then(|_| Ok(())),
        );

        Ok(())
    }

    #[async]
    fn handle_listener(
        listener: TcpListener,
        tx: Sender<NetworkEvent>,
        task_shr: Arc<Mutex<Option<Task>>>,
        handle: Handle,
        clients: ClientsMap,
        kind: u8,
    ) -> Result<(), Error> {
        #[async]
        for (stream, peer_addr) in listener.incoming() {
            handle.spawn(
                Server::handle_stream(
                    stream,
                    kind,
                    tx.clone(),
                    Arc::clone(&task_shr),
                    handle.clone(),
                    Arc::clone(&clients),
                    peer_addr,
                    false,
                ).then(|_| Ok(())),
            );
        }


        Ok(())
    }

    #[async]
    fn handle_stream(
        stream: TcpStream,
        kind: u8,
        tx: Sender<NetworkEvent>,
        task_shr: Arc<Mutex<Option<Task>>>,
        handle: Handle,
        clients: ClientsMap,
        addr: SocketAddr,
        reconnect: bool,
    ) -> Result<(), Error> {
        let id = SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed) as u32;
        let (ssn_reader, ssn_writer) = TcpSession::new_pair(stream, id);
        let (s_tx, s_rx): (f_mpsc::Sender<MuPacket>, f_mpsc::Receiver<MuPacket>) =
            f_mpsc::channel(100);

        let s_ref = SessionRef::new(id, kind, s_tx.clone(), addr);

        {
            let mut map = clients.lock().unwrap();
            map.insert(id, s_ref.clone());
        }

        let evt = NetworkEvent::ClientConnected(s_ref.clone());

        if tx.send(evt).is_ok() {
            if let Some(ref t) = *task_shr.lock().unwrap() {
                t.notify();
            }
        } else {
            println!("Failed to send Connected event.");
            return Ok(());
        }

        let ft = ssn_writer
            .send_all(s_rx.map_err(|_| TcpSessionError::TcpStreamWrite))
            .then(|_| Ok(()));

        handle.spawn(ft);

        await!(Server::handle_tcp_session(
            ssn_reader,
            tx.clone(),
            Arc::clone(&task_shr),
            Arc::clone(&clients),
            s_ref.clone(),
            reconnect,
            handle.clone(),
        ))
    }

    #[async]
    fn handle_tcp_session(
        ssn_reader: TcpSessionReader<ReadHalf<TcpStream>>,
        tx: Sender<NetworkEvent>,
        task_shr: Arc<Mutex<Option<Task>>>,
        clients: ClientsMap,
        s_ref: SessionRef,
        reconnect: bool,
        handle: Handle,
    ) -> Result<(), Error> {
        let task_shr_cs = Arc::clone(&task_shr);
        let s_ref_cj = s_ref.clone();

        #[async]
        for packet in ssn_reader {
            let task = task_shr_cs.lock().unwrap();

            let evt = NetworkEvent::ClientPacket((s_ref_cj.clone(), packet));

            if tx.send(evt).is_ok() {
                if let Some(ref t) = *task {
                    t.notify();
                }
            } else {
                println!("Failed to send Packet event.");
                return Ok(());
            }
        }

        let session_id = s_ref.id;

        {
            let mut map = clients.lock().unwrap();
            map.remove(&session_id);
        }

        let evt = NetworkEvent::ClientDisconnected((session_id, s_ref.kind));

        if tx.send(evt).is_ok() {
            if let Some(ref t) = *task_shr.lock().unwrap() {
                t.notify();
            }
        } else {
            println!("Failed to send Disconnected event.");
            return Ok(());
        }

        if reconnect {
            let handle_cj = handle.clone();
            let ft = Server::try_connect(tx, s_ref.kind, handle_cj, clients, s_ref.addr, task_shr)
                .then(|_| Ok(()));
            handle.spawn(ft);
        }

        Ok(())
    }
}

impl Stream for Server {
    type Item = NetworkEvent;
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        *self.task.lock().unwrap() = Some(task::current());

        match self.evt_rx.try_recv() {
            Err(mpsc::TryRecvError::Empty) => Ok(Async::NotReady),
            Err(mpsc::TryRecvError::Disconnected) => Ok(Async::Ready(None)),
            Ok(evt) => Ok(Async::Ready(Some(evt))),
        }
    }
}
