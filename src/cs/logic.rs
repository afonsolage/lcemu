
use network::prelude::*;

pub struct Handler {}

impl Handler {
    pub fn new() -> Handler {
        Handler {}
    }

    pub fn handle(&self, evt: Event, srv: &Server) {
        match evt {
            Event::ClientConnected(id) => self.on_client_connected(id, srv),
            Event::ClientDisconnected(id) => self.on_client_disconnected(id, srv),
            Event::ClientPacket(pkt) => self.on_packet_received(pkt, srv),
        };
    }

    fn on_client_connected(&self, id: u32, srv: &Server) {
        let res = ConnectResult { res: 1 };
        srv.post_packet(id, res.to_packet()).ok();
    }

    fn on_client_disconnected(&self, id: u32, _: &Server) {
        println!("Client disconnected {}", id)
    }

    fn on_packet_received(&self, pkt: Packet, _: &Server) {
        match pkt.code {
            0x02 => (), //JoinServerStat
            _ => println!("Unhandled packet: {}", pkt),
        };
    }
}
