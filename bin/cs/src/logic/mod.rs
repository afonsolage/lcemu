mod gs;
mod client;

use std::collections::HashMap;
use mu_proto::prelude::*;
use failure::Error;
use futures::{Async, Future, Poll, Stream};

use super::consts;
use self::gs::GSInstance;

pub struct Handler<T: Stream> {
    gs_map: HashMap<u32, GSInstance>,
    clients: HashMap<u32, SessionRef>,
    io: T,
}

impl<T> Handler<T>
where
    T: Stream<Item = NetworkEvent, Error = Error>,
{
    pub fn new(t: T) -> Handler<T> {
        Handler {
            gs_map: HashMap::new(),
            clients: HashMap::new(),
            io: t,
        }
    }

    fn handle_net_event(&mut self, evt: NetworkEvent) -> Poll<(), Error> {
        match evt {
            NetworkEvent::ClientConnected(session) => self.on_connected(session),
            NetworkEvent::ClientDisconnected((id, kind)) => self.on_disconnected(id, kind),
            NetworkEvent::ClientPacket((session, pkt)) => self.on_packet_received(session, pkt),
        };

        Ok(Async::Ready(()))
    }

    fn on_packet_received(&mut self, session: SessionRef, pkt: MuPacket) {
        match session.kind {
            consts::GS_CONN => self.on_server_received(session, pkt),
            _ => self.on_client_received(session, pkt),
        }
    }

    fn on_connected(&mut self, session: SessionRef) {
        match session.kind {
            consts::GS_CONN => self.on_server_connected(session),
            _ => self.on_client_connected(session),
        }
    }

    fn on_disconnected(&mut self, id: u32, kind: u8) {
        match kind {
            consts::GS_CONN => self.on_server_disconnected(id),
            _ => self.on_client_disconnected(id),
        }
    }

    fn broadcast(&mut self, pkt: MuPacket) {
        for (_, session) in &mut self.clients {
            if session.send(pkt.clone()).is_err() {
                session.close().ok();
            }
        }
    }
}

impl<T> Future for Handler<T>
where
    T: Stream<Item = NetworkEvent, Error = Error>,
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.io.poll()? {
                Async::Ready(Some(evt)) => try_ready!(self.handle_net_event(evt)),
                Async::Ready(None) => break Ok(Async::Ready(())),
                Async::NotReady => break Ok(Async::NotReady),
            }
        }
    }
}
