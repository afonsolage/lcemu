extern crate config;
extern crate mu_proto;
extern crate scheduled_executor;
extern crate tokio_core;
extern crate failure;

#[macro_use]
extern crate futures;

mod consts;
mod logic;

// use scheduled_executor::CoreExecutor;
use tokio_core::reactor::{Core, Handle};

// use std::time::Duration;

use mu_proto::prelude::*;


fn main() {
    println!("Starting Game Server...");

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name("config/gs.toml"))
        .expect("Failed to load config file.");

    let mut reactor = Core::new().unwrap();
    let svr = setup_networking(&settings, reactor.handle());
    // let _executor = setup_executor(&server, &settings);

    reactor.run(logic::Handler::new(svr)).unwrap();
}

fn setup_networking(settings: &config::Config, handle: Handle) -> Server {
    let mut server = Server::new(handle);

    //Setup TCP Server
    {
        let listen_addr = match settings.get_str("network.listen_addr") {
            Ok(addr) => addr,
            Err(_) => format!("0.0.0.0"),
        };

        let listen_port = match settings.get_int("network.listen_port") {
            Ok(port) => port,
            Err(_) => 55590,
        };

        server.start_tcp(&listen_addr, listen_port as u16).ok();
    }

    {
        let addr = match settings.get_str("network.cs_addr") {
            Ok(addr) => addr,
            Err(_) => format!("127.0.0.1"),
        };

        let port = match settings.get_int("network.cs_port") {
            Ok(port) => port,
            Err(_) => 55557,
        };

        server.connect_to(&addr, port as u16).ok();
    }

    server
}

// fn setup_executor(server: &Server, settings: &config::Config) -> CoreExecutor {
//     let udp_client = match server.get_udp_client(consts::CS_SERVER) {
//             Err(why) => panic!("Failed to get udp client: {:?}", why),
//             Ok(client) => client,
//         };

//         let max_user = match settings.get_int("general.max_user") {
//             Ok(max) => max,
//             Err(_) => 100,
//         };
//         let server_code = match settings.get_int("general.server_code") {
//             Ok(code) => code,
//             Err(_) => 1,
//         };

//         let executor = CoreExecutor::new().expect("Failed to setup Scheduled Executor");
//         executor.schedule_fixed_rate(Duration::from_secs(2), Duration::from_secs(5), move |_| {
//             println!("Sending keep alive!");
//             let pkt = ServerInfo {
//                 svr_code: server_code as u16,
//                 perc: 0,
//                 usr_cnt: 0,
//                 acc_cnt: 0,
//                 pcbng_cnt: 0,
//                 mx_usr_cnt: max_user as u16,
//             };
//             udp_client.send(&pkt.to_packet()).ok();
//         });
        
//         executor
// }