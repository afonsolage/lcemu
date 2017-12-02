use mu_proto::prelude::*;
use futures::{Async, Future, Poll, Stream};
use failure::Error;

pub struct Handler<T: Stream> {
    io: T,
}

impl<T> Handler<T>
where
    T: Stream<Item = NetworkEvent, Error = Error>,
{
    pub fn new(t: T) -> Handler<T> {
        Handler { io: t }
    }

    pub fn handle_net_event(&self, evt: NetworkEvent) -> Poll<(), Error> {
        match evt {
            NetworkEvent::ClientConnected(session) => self.on_connected(session),
            NetworkEvent::ClientDisconnected((id, kind)) => self.on_disconnected(id, kind),
            NetworkEvent::ClientPacket((session, pkt)) => self.on_received(session, pkt),
        };
        Ok(Async::Ready(()))
    }

    fn on_connected(&self, mut session: SessionRef) {
        println!("Client connected {}", session.id)
    }

    fn on_disconnected(&self, id: u32, kind: u8) {
        println!("Client disconnected {}", id)
    }

    fn on_received(&self, session: SessionRef, pkt: MuPacket) {
        match pkt.code {
            _ => println!("Unhandled packet: {}", pkt),
        };
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
