extern crate mu_proto;
extern crate config;
extern crate tokio_core;

#[macro_use]
extern crate futures;

extern crate failure;

use tokio_core::reactor::{Core, Handle};
use mu_proto::prelude::*;

mod logic;
mod consts;

fn main() {
    println!("Starting Connect Server...");

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name("config/cs.toml"))
        .expect("Failed to load config file.");

    let mut reactor = Core::new().unwrap();
    let svr = setup_networking(&settings, reactor.handle());

    reactor.run(logic::Handler::new(svr)).unwrap();
}

fn setup_networking(settings: &config::Config, handle: Handle) -> Server {
    let mut server = Server::new(handle);

    //Setup external TCP Server
    {
        let external_addr = match settings.get_str("network.external_addr") {
            Ok(addr) => addr,
            Err(_) => format!("0.0.0.0"),
        };

        let external_port = match settings.get_int("network.external_port") {
            Ok(port) => port,
            Err(_) => 44405,
        };

        server.start_tcp(&external_addr, external_port as u16, consts::CLIENT_CONN).ok();
    }

    //Setup internal TCP Server
    {
        let internal_addr = match settings.get_str("network.internal_addr") {
            Ok(addr) => addr,
            Err(_) => format!("0.0.0.0"),
        };

        let internal_port = match settings.get_int("network.internal_port") {
            Ok(port) => port,
            Err(_) => 55557,
        };

        server.start_tcp(&internal_addr, internal_port as u16, consts::GS_CONN).ok();
    }

    server
}
