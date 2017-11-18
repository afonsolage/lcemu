extern crate config;
extern crate network;

use network::Server;
mod logic;

fn main() {
    println!("Starting Connect Server...");

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name("config/cs.toml"))
        .expect("Failed to load config file.");

    let server = setup_networking(&settings);

    let mut handler = logic::Handler::new();
    handler.setup(&settings);

    for packet in &server {
        handler.handle(packet, &server);
    }
}

fn setup_networking(settings: &config::Config) -> Server {
    let mut server = Server::new();

    //Setup TCP Server
    {
        let listen_tcp_addr = match settings.get_str("network.listen_tcp_addr") {
            Ok(addr) => addr,
            Err(_) => format!("0.0.0.0"),
        };

        let listen_tcp_port = match settings.get_int("network.listen_tcp_port") {
            Ok(port) => port,
            Err(_) => 44405,
        };

        server.start_tcp(&listen_tcp_addr, listen_tcp_port as u16);
    }

    //Setup UDP Clients
    {
        let listen_udp_addr = match settings.get_str("network.listen_udp_addr") {
            Ok(addr) => addr,
            Err(_) => format!("0.0.0.0"),
        };

        let listen_udp_port = match settings.get_int("network.listen_udp_port") {
            Ok(port) => port,
            Err(_) => 55557,
        };

        server.start_udp(&listen_udp_addr, listen_udp_port as u16);
    }

    server
}
