mod network;
mod logic;

fn main() {
    println!("Starting server...");
    let mut server = network::Server::new();
    server.start("0.0.0.0", 44405);

    let handler = logic::Handler::new();

    for packet in &server {
        handler.handle(packet, &server);
    }
}
