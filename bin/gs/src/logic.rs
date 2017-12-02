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

    pub fn handle_net_event(&self, evt: NetworkEvent) {
        match evt {
            NetworkEvent::ClientConnected(session) => self.on_client_connected(session),
            NetworkEvent::ClientDisconnected(id) => self.on_client_disconnected(id),
            NetworkEvent::ClientPacket((session, pkt)) => self.on_packet_received(session, pkt),
            _ => println!("Unkown event: {:?}", evt),
        };
    }

    fn on_client_connected(&self, mut session: SessionRef) {
        println!("Client connected {}", session.id)
    }

    fn on_client_disconnected(&self, id: u32) {
        println!("Client disconnected {}", id)
    }

    fn on_packet_received(&self, session: SessionRef, pkt: MuPacket) {
        match pkt.code {
            _ => println!("Unhandled packet: {}", pkt),
        };
    }

    fn process(&mut self, evt: NetworkEvent) -> Poll<(), Error> {
        self.handle_net_event(evt);
        Ok(Async::Ready(()))
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
                Async::Ready(Some(evt)) => try_ready!(self.process(evt)),
                Async::Ready(None) => break Ok(Async::Ready(())),
                Async::NotReady => break Ok(Async::NotReady),
            }
        }
    }
}
