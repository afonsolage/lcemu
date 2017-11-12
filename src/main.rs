mod network;

fn main() {
    println!("Starting server...");
    let mut server = network::Server::new("0.0.0.0", 44405);
    server.start();
}
