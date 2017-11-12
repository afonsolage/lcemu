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
}

pub struct Server {
    sessions: HashMap<u32, Session>,
    session_evt_tx: Sender<NetEvent>,
    session_evt_rx: Receiver<NetEvent>,
    evt_tx: Sender<Event>,
    evt_rx: Receiver<Event>,
}


impl<'a> Server {
    pub fn new() -> Server {
        let (stx, srx): (Sender<NetEvent>, Receiver<NetEvent>) = mpsc::channel();
        let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();
        Server {
            sessions: HashMap::new(),
            session_evt_tx: stx,
            session_evt_rx: srx,
            evt_tx: tx,
            evt_rx: rx,
        }
    }

    pub fn send_event(&mut self, evt: NetEvent) -> Result<(), Error> {
        match self.session_evt_tx.send(evt) {
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

        let cj_tx = self.session_evt_tx.clone();

        thread::Builder::new()
            .name(format!("TCPServerListener"))
            .spawn(move || {
                let mut client_index = 1u32;
                for stream in cj_listener.incoming() {
                    let session = match Session::new(client_index, stream.unwrap(), cj_tx.clone()) {
                        Err(_) => continue,
                        Ok(ss) => ss,
                    };

                    client_index += 1;

                    match cj_tx.send(NetEvent::Connected(session)) {
                        Err(why) => {
                            println!("Failed to send SessionEvent: {:?}", why);
                            break;
                        }
                        _ => (),
                    }
                }
            })
            .unwrap();

        println!("Waiting for Session Events.");

        loop {
            match self.session_evt_rx.recv() {
                Err(why) => {
                    println!("Failed to read Sessions RX: {:?}", why);
                    break;
                }
                Ok(evt) => {
                    println!("Received event {:?}", evt);
                    self.handle_event(evt);
                }
            }
        }

        println!("Server has stopped.");
    }

    fn parse_packet(&self, buf: &[u8]) -> Option<PacketType> {
        if buf.len() < 2 {
            println!("Failed to parse packet: Length is too smal: {}", buf.len());
            return None;
        }

        match buf[0] {
            0xC1 => {
                Some(PacketType::C1(C1Packet::new(&buf[1..])))
            }
            0xC2 => {
                Some(PacketType::C2(C2Packet::new(&buf[1..])))
            }
            _ => {
                println!("Unsupported code received: {}", buf[0]);
                None
            }
        }
    }

    pub fn handle_event(&mut self, evt: NetEvent) {
        match evt {
            NetEvent::Disconnected(id) => {
                self.sessions.remove(&id);
                self.evt_tx.send(Event::ClientDisconnected(id)).ok();
            }
            NetEvent::Connected(session) => {
                let id = session.id;
                self.sessions.insert(session.id, session);
                self.evt_tx.send(Event::ClientConnected(id)).ok();
            }
            NetEvent::PacketData(ref buf) => match self.parse_packet(buf) {
                None => println!("Invalid packet data received: {:?}", buf),
                Some(pkt) => {
                    self.evt_tx.send(Event::ClientPacket(pkt)).ok();
                }
            },
            NetEvent::PostPacket(ref id, ref buf) => {
                let mut session = match self.sessions.get_mut(&id) {
                    None => return,
                    Some(s) => s,
                };

                session.send(&buf).ok();
            }
        };
    }
}

impl Iterator for Server {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        match self.evt_rx.recv() {
            Err(why) => None,
            Ok(evt) => Some(evt),
        }
    }
}
