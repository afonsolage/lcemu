extern crate mu_proto;
extern crate tokio_core;
extern crate futures;

use futures::stream::Stream;
use self::tokio_core::reactor::{Core, Handle};
use mu_proto::Server;
use mu_proto::NetworkEvent;
use mu_proto::MuPacket;
use mu_proto::ConnectResult;
use mu_proto::ProtoMsg;

fn main() {
    let mut reactor = Core::new().unwrap();

    let mut svr = Server::new(reactor.handle());
    svr.start_tcp("0.0.0.0", 44405).ok();

    let svr_ft = svr.for_each(|evt| {
        println!("Received: {:?}", evt);

        match evt {
            NetworkEvent::ClientConnected(mut s_ref) => {
                let proto = ConnectResult { res: 1 };
                let pkt =
                    MuPacket::from_protocol(&ProtoMsg::ConnectResult, &proto);
                s_ref.send(pkt).ok();
            }
            _ => (),
        };

        Ok(())
    });

    reactor.run(svr_ft);
}
