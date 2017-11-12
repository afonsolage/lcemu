use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::net::TcpListener;
use std::thread;

use super::Session;
use super::SessionEvent;

pub struct Server {
    sessions: Vec<Session>,
    session_evt_tx: Sender<SessionEvent>,
    session_evt_rx: Receiver<SessionEvent>,
}

impl<'a> Server {
    pub fn new() -> Server {
        let (tx, rx): (Sender<SessionEvent>, Receiver<SessionEvent>) = mpsc::channel();
        Server {
            sessions: vec![],
            session_evt_tx: tx,
            session_evt_rx: rx,
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

                    match cj_tx.send(SessionEvent::Connected(session)) {
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

    pub fn handle_event(&mut self, evt: SessionEvent) {
        match evt {
            SessionEvent::Disconnected(id) => self.sessions.retain(|e| e.id != id),
            SessionEvent::Connected(mut session) => {

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

                self.sessions.push(session);
            }
            SessionEvent::Data(buf) => println!("Received: {:?}", buf),
        };
    }
}
