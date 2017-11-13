mod network;

fn main() {
    println!("Starting server...");
    let mut server = network::Server::new();
    server.start("0.0.0.0", 44405);

    for packet in server {
        println!("Got: {:?}", packet);
    }
}
