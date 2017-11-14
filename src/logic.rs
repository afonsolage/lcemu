
use network::Server;
use network::Event;
use network::PacketType;
use network::C1Packet;

pub struct Handler {}

impl Handler {
    pub fn new() -> Handler {
        Handler {}
    }

    pub fn handle(&self, evt: Event, srv: &Server) {
        println!("Got: {:?}", evt);

        match evt {
            Event::ClientConnected(id) => self.on_client_connected(id, srv),
            Event::ClientDisconnected(id) => self.on_client_disconnected(id, srv),
            Event::ClientPacket(pkt) => self.on_packet_received(pkt, srv),
        };
    }

    fn on_client_connected(&self, id: u32, srv: &Server) {
        let pkt = C1Packet::result(true);

        srv.post_packet(id, pkt).ok();
    }

    fn on_client_disconnected(&self, id: u32, srv: &Server) {}

    fn on_packet_received(&self, pkt: PacketType, srv: &Server) {}
}
