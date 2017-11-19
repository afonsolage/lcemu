extern crate futures;
extern crate tokio_core;

use self::futures::{Future, Stream};
use self::tokio_core::net::{TcpListener, TcpStream};
use self::tokio_core::reactor::Core;
use super::tcp_session::{Error as SessionError, TcpSession};

use std::collections::HashMap;

pub enum Error {
    InvalidAddress,
    TcpBindError,
}

pub struct Server {
    core: Core,
}

impl<'a> Server {
    pub fn new() -> Server {
        Server {
            core: Core::new().expect("Failed to initialize Mu Proto Core."),
        }
    }

    pub fn start_tcp(&mut self, listen_addr: &'a str, port: u16) -> Result<(), Error> {
        let handle = self.core.handle();
        let addr = match format!("{}:{}", listen_addr, port).parse() {
            Err(_) => return Err(Error::InvalidAddress),
            Ok(addr) => addr,
        };

        let listener = match TcpListener::bind(&addr, &handle) {
            Err(_) => return Err(Error::TcpBindError),
            Ok(r) => r,
        };

        // let mut map: HashMap<u32, TcpSession<TcpStream>> = HashMap::new();
        // let mut sessionId = 0;

        let cloure_handle = self.core.handle();
        let ft = listener
            .incoming()
            .for_each(move |(stream, _peer_addr)| {
                let session = TcpSession::new(stream);

                println!("Connected!");

                let session_ft = session
                    .for_each(move |req| {
                        println!("Received: {:?}", req);
                        Ok(())
                    })
                    .map_err(|err| println!("Session Error: {:?}", err))
                    .and_then(|_| {
                        println!("Disconnected!");
                        Ok(())
                    });

                cloure_handle.spawn(session_ft);

                Ok(())
            })
            .map_err(|err| println!("TCPSocket Listener Error: {:?}", err));

        handle.spawn(ft);

        Ok(())
    }

    pub fn start(&mut self) {
        loop {
            self.core.turn(None);
        }
    }
}
