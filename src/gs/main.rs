extern crate config;
extern crate network;
extern crate scheduled_executor;

mod consts;
mod logic;

use std::time::Duration;
use network::prelude::*;
use scheduled_executor::CoreExecutor;

fn main() {
    println!("Starting Game Server...");

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name("GameServer.toml"))
        .expect("Failed to load config file.");

    let server = setup_networking(&settings);
    let executor = setup_executor(&server, &settings);

    let handler = logic::Handler::new();

    for packet in &server {
        handler.handle(packet, &server);
    }
}

fn setup_networking(settings: &config::Config) -> Server {
    let mut server = network::Server::new();

    //Setup TCP Server
    {
        let listen_tcp_addr = match settings.get_str("network.listen_tcp_addr") {
            Ok(addr) => addr,
            Err(_) => format!("0.0.0.0"),
        };

        let listen_tcp_port = match settings.get_int("network.listen_tcp_port") {
            Ok(port) => port,
            Err(_) => 55590,
        };

        server.start_tcp(&listen_tcp_addr, listen_tcp_port as u16);
    }

    //Setup UDP Clients
    {
        let cs_addr = match settings.get_str("network.cs_addr") {
            Ok(addr) => addr,
            Err(_) => format!("127.0.0.1"),
        };

        let cs_port = match settings.get_int("network.cs_port") {
            Ok(port) => port,
            Err(_) => 55590,
        };
        server.add_udp_client(consts::CS_SERVER, &cs_addr, cs_port as u16);
    }

    server
}

fn setup_executor(server: &Server, settings: &config::Config) -> CoreExecutor {
    let udp_client = match server.get_udp_client(consts::CS_SERVER) {
            Err(why) => panic!("Failed to get udp client: {:?}", why),
            Ok(client) => client,
        };

        let max_user = match settings.get_int("general.max_user") {
            Ok(max) => max,
            Err(_) => 100,
        };
        let server_code = match settings.get_int("general.server_code") {
            Ok(code) => code,
            Err(_) => 1,
        };

        let executor = CoreExecutor::new().expect("Failed to setup Scheduled Executor");
        executor.schedule_fixed_rate(Duration::from_secs(2), Duration::from_secs(5), move |_| {
            println!("Sending keep alive!");
            let pkt = ServerInfo {
                svr_code: server_code as u16,
                perc: 0,
                usr_cnt: 0,
                acc_cnt: 0,
                pcbng_cnt: 0,
                mx_usr_cnt: max_user as u16,
            };
            udp_client.send(&pkt.to_packet()).ok();
        });
        
        executor
}