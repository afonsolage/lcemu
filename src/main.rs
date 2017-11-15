mod network;
mod logic;
mod protocol;

fn main() {
    println!("Starting server...");
    let mut server = network::Server::new();
    server.start_tcp("0.0.0.0", 44405);
    server.start_udp("0.0.0.0", 55557);

    let handler = logic::Handler::new();
    
    for packet in &server {
        handler.handle(packet, &server);
    }

}
