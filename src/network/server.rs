use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::net::TcpListener;
use std::thread;

use super::Session;
use super::SessionEvent;

type SessionList = Arc<Mutex<Vec<Session>>>;

pub struct Server {
    listener: TcpListener,
    running: bool,
    sessions: SessionList,
    sessions_tx: Sender<SessionEvent>,
}

impl<'a> Server {
    pub fn new(listen_addr: &'a str, port: u16) -> Server {
        let (tx, rx): (Sender<SessionEvent>, Receiver<SessionEvent>) = mpsc::channel();

        match TcpListener::bind(format!("{}:{}", listen_addr, port)) {
            Err(why) => panic!("{:?}", why),
            Ok(listener) => {
                let sessions: SessionList = Arc::new(Mutex::new(vec![]));

                let clojure_sessions = sessions.clone();
                thread::Builder::new()
                    .name(format!("TcpServerListener"))
                    .spawn(move || {
                        loop {
                            match rx.recv() {
                                Err(why) => panic!("Failed to read Sessions RX: {:?}", why),
                                Ok(evt) => {
                                    println!("Removing session {:?}", evt);
                                    // let mut list = clojure_sessions.lock().unwrap();
                                    // list.retain(|ref s| s.id != uid);
                                }
                            }
                        }
                    })
                    .unwrap();

                Server {
                    listener: listener,
                    running: false,
                    sessions: sessions,
                    sessions_tx: tx,
                }
            }
        }
    }

    pub fn start(&mut self) {
        let mut client_index = 1u32;
        for stream in self.listener.incoming() {
            let session =
                match Session::new(client_index, stream.unwrap(), self.sessions_tx.clone()) {
                    Err(_) => continue,
                    Ok(ss) => ss,
                };

            client_index += 1;

            let mut list = self.sessions.lock().unwrap();
            list.push(session);
        }
    }
}
