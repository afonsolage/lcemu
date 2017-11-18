use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::net::TcpListener;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::thread;
use std::collections::HashMap;

mod session;

use self::session::TcpSession;
use super::packet::Packet;

#[derive(Debug)]
pub enum SessionEvent {
    Connected(TcpSession),
    Disconnected(u32),
    PacketData(Vec<u8>),
    PostPacket(u32, Vec<u8>),
    RequestDisconnect(u32),
}

#[derive(Debug)]
pub enum Event {
    ClientConnected(u32),
    ClientDisconnected(u32),
    ClientPacket(Packet),
}

#[derive(Debug)]
pub enum Error {
    EventTxFailure,
    EventTxNone,
    SendPacketSerializeFailed,
    UdpClientNotFound,
    UdpSendError,
    UdpClientCloneFail,
}

pub struct Server {
    evt_rx: Receiver<Event>,
    net_tx: Sender<SessionEvent>,
    udp_clients: HashMap<u32, UdpClient>,
}

#[derive(Debug)]
pub struct UdpClient {
    id: u32,
    addr: SocketAddr,
    socket: UdpSocket,
}

impl<'a> UdpClient {
    pub fn new(id: u32, address: &'a str, port: u16) -> UdpClient {
        let addr = format!("{}:{}", address, port);
        let addr: SocketAddr = addr.parse().expect("Invalid addr supplied: ");
        let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind UDP socket.");

        UdpClient {
            id: id,
            addr: addr,
            socket: socket,
        }
    }

    pub fn try_clone(&self) -> Result<Self, Error> {
        match self.socket.try_clone() {
            Err(_) => Err(Error::UdpClientCloneFail),
            Ok(socket) => Ok(UdpClient {
                id: self.id,
                addr: self.addr.clone(),
                socket: socket,
            }),
        }
    }

    pub fn send(&self, pkt: &Packet) -> Result<(), Error> {
        let mut buf = vec![0; pkt.len()];
        match pkt.serialize(&mut buf) {
            Err(why) => {
                println!("Failed to send udp packet: {:?}", why);
                return Err(Error::UdpSendError);
            }
            Ok(_) => match self.socket.send_to(&buf, &self.addr) {
                Err(why) => {
                    println!("Failed to send udp packet: {:?}", why);
                    return Err(Error::UdpSendError);
                }
                Ok(_) => {
                    println!("=>[{:?}]: {}", &self.addr, pkt);
                    Ok(())
                }
            },
        }
    }
}

impl<'a> Server {
    pub fn new() -> Server {
        let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();
        let (stx, srx): (Sender<SessionEvent>, Receiver<SessionEvent>) = mpsc::channel();

        let cj_evt_tx = tx.clone();
        thread::spawn(move || {
            let mut sessions = HashMap::new();
            println!("Waiting for Session Events.");
            loop {
                match srx.recv() {
                    Err(why) => {
                        println!("Failed to read Sessions RX: {:?}", why);
                        break;
                    }
                    Ok(evt) => {
                        Server::handle_event(&mut sessions, cj_evt_tx.clone(), evt);
                    }
                }
            }
        });

        Server {
            net_tx: stx,
            evt_rx: rx,
            udp_clients: HashMap::new(),
        }
    }

    pub fn add_udp_client(&mut self, id: u32, address: &'a str, port: u16) {
        let client = UdpClient::new(id, address, port);
        self.udp_clients.insert(client.id, client);
    }

    pub fn get_udp_client(&self, id: u32) -> Result<UdpClient, Error> {
        match self.udp_clients.get(&id) {
            None => Err(Error::UdpClientNotFound),
            Some(client) => client.try_clone(),
        }
    }

    pub fn post_packet(&self, id: u32, pkt: Packet) -> Result<(), Error> {
        let mut buf = vec![0u8; pkt.len()];
        
        match pkt.serialize(&mut buf) {
            Err(why) => {
                println!(
                    "Failed to send packet {}. Error: {:?}. Buf: {:?}",
                    pkt,
                    why,
                    buf
                );
                Err(Error::SendPacketSerializeFailed)
            }
            Ok(_) => self.send_event(SessionEvent::PostPacket(id, buf)),
        }
    }

    pub fn disconnect(&self, id: u32) -> Result<(), Error> {
        self.send_event(SessionEvent::RequestDisconnect(id))
    }

    fn send_event(&self, evt: SessionEvent) -> Result<(), Error> {
        let tx = self.net_tx.clone();

        match tx.send(evt) {
            Err(why) => {
                println!("Failed to send server event: {:?}", why);
                Err(Error::EventTxFailure)
            }
            Ok(_) => Ok(()),
        }
    }

    pub fn start_udp(&mut self, listen_addr: &'a str, port: u16) {
        let bind = format!("{}:{}", listen_addr, port);
        println!("Binding UDP on {}", bind);
        let socket = match UdpSocket::bind(bind) {
            Err(why) => panic!("{:?}", why),
            Ok(s) => s,
        };

        let tx = self.net_tx.clone();
        thread::Builder::new()
            .name(format!("UDPServerListener"))
            .spawn(move || {
                let mut buf = [0; 0xFFFF];
                loop {
                    match socket.recv_from(&mut buf) {
                        Err(_) => {
                            println!("Failed to read from socket.");
                            break;
                        }
                        Ok((rc, _)) => {
                            if rc == 0 {
                                println!("Read 0 bytes.");
                                break;
                            }

                            match tx.send(SessionEvent::PacketData(buf[0..rc].to_vec())) {
                                Err(why) => {
                                    println!("Failed to send SessionEvent: {:?}", why);
                                    break;
                                }
                                _ => (),
                            }
                        }
                    };
                }

                println!("Quitting UDPServerListener");
            })
            .unwrap();
    }

    pub fn start_tcp(&mut self, listen_addr: &'a str, port: u16) {
        let bind = format!("{}:{}", listen_addr, port);
        println!("Binding TCP on {}", bind);
        let cj_listener = match TcpListener::bind(bind) {
            Err(why) => panic!("{:?}", why),
            Ok(listener) => listener,
        };

        let stx = self.net_tx.clone();

        thread::Builder::new()
            .name(format!("TCPServerListener"))
            .spawn(move || {
                let mut client_index = 1u32;
                for stream in cj_listener.incoming() {
                    let session = match TcpSession::new(client_index, stream.unwrap(), stx.clone())
                    {
                        Err(_) => continue,
                        Ok(ss) => ss,
                    };

                    client_index += 1;

                    match stx.send(SessionEvent::Connected(session)) {
                        Err(why) => {
                            println!("Failed to send SessionEvent: {:?}", why);
                            break;
                        }
                        _ => (),
                    }
                }
            })
            .unwrap();
    }

    fn parse_packet(buf: &[u8]) -> Option<Packet> {
        if buf.len() < 2 {
            println!("Failed to parse packet: Length is too smal: {}", buf.len());
            return None;
        }

        Some(Packet::new(buf))
    }

    pub fn handle_event(
        sessions: &mut HashMap<u32, TcpSession>,
        evt_tx: Sender<Event>,
        evt: SessionEvent,
    ) {
        match evt {
            SessionEvent::Disconnected(id) => {
                sessions.remove(&id);
                evt_tx.send(Event::ClientDisconnected(id)).ok();
            }
            SessionEvent::Connected(session) => {
                let id = session.id;
                sessions.insert(session.id, session);
                evt_tx.send(Event::ClientConnected(id)).ok();
            }
            SessionEvent::PacketData(ref buf) => match Server::parse_packet(buf) {
                None => println!("Invalid packet data received: {:?}", buf),
                Some(pkt) => {
                    evt_tx.send(Event::ClientPacket(pkt)).ok();
                }
            },
            SessionEvent::PostPacket(ref id, ref buf) => {
                let mut session = match sessions.get_mut(&id) {
                    None => return,
                    Some(s) => s,
                };

                session.send(&buf).ok();
            },
            SessionEvent::RequestDisconnect(ref id) => {
                
            }
        };
    }
}

impl<'a> Iterator for &'a Server {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        match self.evt_rx.recv() {
            Err(_) => None,
            Ok(evt) => Some(evt),
        }
    }
}
