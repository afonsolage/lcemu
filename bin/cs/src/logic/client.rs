use failure::Error;
use futures::Stream;
use mu_proto::prelude::*;

use logic::Handler;

impl<T> Handler<T>
where
    T: Stream<Item = NetworkEvent, Error = Error>,
{
    pub fn on_client_connected(&mut self, mut session: SessionRef) {
        let res = ConnectResult { res: 1 };

        if session.send(res.to_packet()).is_err() {
            session.close().ok();
            return;
        }

        if self.send_server_list(&mut session).is_err() {
            session.close().ok();
            return;
        }

        self.clients.insert(session.id, session);
    }

    pub fn on_client_disconnected(&mut self, id: u32) {
        println!("Client disconnected {}", id);
        self.clients.remove(&id);
    }

    pub fn on_client_received(&mut self, session: SessionRef, pkt: MuPacket) {
        match pkt.code {
            0x01 => self.on_server_info(ServerInfo::parse(&pkt.data), session),
            0x02 => (), //JoinServerStat
            _ => println!("Unhandled packet: {}", pkt),
        };
    }
}
