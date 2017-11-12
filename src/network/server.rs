use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::net::TcpListener;
use std::thread;
use std::collections::HashMap;

use super::Session;

#[derive(Debug)]
pub enum Event {
    Connected(Session),
    Disconnected(u32),
    PacketData(Vec<u8>),
    PostPacket(u32, Vec<u8>),
}

pub enum Error {
    EventTxFailure,
}

pub struct Server {
    sessions: HashMap<u32, Session>,
    session_evt_tx: Sender<Event>,
    session_evt_rx: Receiver<Event>,
}


impl<'a> Server {
    pub fn new() -> Server {
        let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();
        Server {
            sessions: HashMap::new(),
            session_evt_tx: tx,
            session_evt_rx: rx,
        }
    }

    pub fn send_event(&mut self, evt: Event) -> Result<(), Error> {
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

                    match cj_tx.send(Event::Connected(session)) {
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
                Err(why) => panic!("Failed to read Sessions RX: {:?}", why),
                Ok(evt) => {
                    println!("Received event {:?}", evt);
                    self.handle_event(evt);
                }
            }
        }
    }

    pub fn handle_event(&mut self, evt: Event) {
        match evt {
            Event::Disconnected(id) => {
                self.sessions.remove(&id);
            }
            Event::Connected(mut session) => {
                let welcome = [0xC1, 0x04, 0x00, 0x01];
                let srv_list = [
                    0xC2,
                    0x00,
                    0x0B,
                    0xF4,
                    0x06,
                    0x00,
                    0x01,
                    0x00,
                    0x00,
                    0x00,
                    0x77,
                ];

                session.send(&welcome).unwrap();

                session.send(&srv_list).unwrap();

                self.sessions.insert(session.id, session);
            }
            Event::PacketData(buf) => println!("Received: {:?}", buf),
            Event::PostPacket(id, buf) => {
                let mut session = match self.sessions.get_mut(&id) {
                    None => return,
                    Some(s) => s,
                };

                session.send(&buf).ok();
            }
        };
    }
}
