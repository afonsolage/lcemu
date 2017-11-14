use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::net::TcpListener;
use std::thread;
use std::collections::HashMap;

use super::Session;
use super::PacketType;
use super::packet::C1Packet;
use super::packet::C2Packet;

#[derive(Debug)]
pub enum NetEvent {
    Connected(Session),
    Disconnected(u32),
    PacketData(Vec<u8>),
    PostPacket(u32, Vec<u8>),
}

#[derive(Debug)]
pub enum Event {
    ClientConnected(u32),
    ClientDisconnected(u32),
    ClientPacket(PacketType),
}

pub enum Error {
    EventTxFailure,
    EventTxNone,
    SendPacketSerializeFailed,
}

pub struct Server {
    evt_tx: Sender<Event>,
    evt_rx: Receiver<Event>,
    net_tx: Option<Sender<NetEvent>>,
}

impl<'a> Server {
    pub fn new() -> Server {
        let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();
        Server {
            net_tx: None,
            evt_tx: tx,
            evt_rx: rx,
        }
    }

    pub fn post_packet(&self, id: u32, pkt: C1Packet) -> Result<(), Error> {
        let mut buf = vec![0u8; pkt.len()];

        match pkt.serialize(&mut buf) {
            Err(why) => {
                println!(
                    "Failed to send packet {:?}. Error: {:?}. Buf: {:?}",
                    pkt,
                    why,
                    buf
                );
                Err(Error::SendPacketSerializeFailed)
            }
            Ok(_) => self.send_event(NetEvent::PostPacket(id, buf)),
        }
    }

    fn send_event(&self, evt: NetEvent) -> Result<(), Error> {
        let tx = match self.net_tx {
            None => return Err(Error::EventTxNone),
            Some(ref tx) => tx.clone(),
        };

        match tx.send(evt) {
            Err(why) => {
                println!("Failed to send server event: {:?}", why);
                Err(Error::EventTxFailure)
            }
            Ok(_) => Ok(()),
        }
    }

    pub fn start(&mut self, listen_addr: &'a str, port: u16) {
        let cj_listener = match TcpListener::bind(format!("{}:{}", listen_addr, port)) {
            Err(why) => panic!("{:?}", why),
            Ok(listener) => listener,
        };

        let (stx, srx): (Sender<NetEvent>, Receiver<NetEvent>) = mpsc::channel();

        self.net_tx = Some(stx.clone());

        thread::Builder::new()
            .name(format!("TCPServerListener"))
            .spawn(move || {
                let mut client_index = 1u32;
                for stream in cj_listener.incoming() {
                    let session = match Session::new(client_index, stream.unwrap(), stx.clone()) {
                        Err(_) => continue,
                        Ok(ss) => ss,
                    };

                    client_index += 1;

                    match stx.send(NetEvent::Connected(session)) {
                        Err(why) => {
                            println!("Failed to send SessionEvent: {:?}", why);
                            break;
                        }
                        _ => (),
                    }
                }
            })
            .unwrap();

        let cj_evt_tx = self.evt_tx.clone();
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
                        println!("Received event {:?}", evt);
                        Server::handle_event(&mut sessions, cj_evt_tx.clone(), evt);
                    }
                }
            }
        });
    }

    fn parse_packet(buf: &[u8]) -> Option<PacketType> {
        if buf.len() < 2 {
            println!("Failed to parse packet: Length is too smal: {}", buf.len());
            return None;
        }

        match buf[0] {
            0xC1 => Some(PacketType::C1(C1Packet::new(&buf[1..]))),
            0xC2 => Some(PacketType::C2(C2Packet::new(&buf[1..]))),
            _ => {
                println!("Unsupported code received: {}", buf[0]);
                None
            }
        }
    }

    pub fn handle_event(
        sessions: &mut HashMap<u32, Session>,
        evt_tx: Sender<Event>,
        evt: NetEvent,
    ) {
        match evt {
            NetEvent::Disconnected(id) => {
                sessions.remove(&id);
                evt_tx.send(Event::ClientDisconnected(id)).ok();
            }
            NetEvent::Connected(session) => {
                let id = session.id;
                sessions.insert(session.id, session);
                evt_tx.send(Event::ClientConnected(id)).ok();
            }
            NetEvent::PacketData(ref buf) => match Server::parse_packet(buf) {
                None => println!("Invalid packet data received: {:?}", buf),
                Some(pkt) => {
                    evt_tx.send(Event::ClientPacket(pkt)).ok();
                }
            },
            NetEvent::PostPacket(ref id, ref buf) => {
                let mut session = match sessions.get_mut(&id) {
                    None => return,
                    Some(s) => s,
                };

                session.send(&buf).ok();
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
