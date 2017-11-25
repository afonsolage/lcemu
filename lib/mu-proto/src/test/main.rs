extern crate mu_proto;
extern crate tokio_core;
extern crate futures;

use futures::stream::Stream;
use self::tokio_core::reactor::{Core, Handle};
use mu_proto::Server;

fn main() {
    let mut reactor = Core::new().unwrap();

    let mut svr = Server::new(reactor.handle());
    svr.start_tcp("0.0.0.0", 44405).ok();

    let svr_ft = svr.for_each(|evt| {
        println!("Received: {:?}", evt);
        Ok(())
    });

    reactor.run(svr_ft);
}