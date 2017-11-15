extern crate network;

use network::prelude::*;

fn main() {
    println!("Starting Game Server...");
    let mut server = network::Server::new();

    server.add_udp_client(1, "127.0.0.1", 55557);

    server.start_tcp("0.0.0.0", 44405);

    let info = ServerInfo {
        svr_code: 1,
        perc: 0,
        usr_cnt: 0,
        acc_cnt: 0,
        pcbng_cnt: 0,
        mx_usr_cnt: 100,
    };

    server.send_udp_packet(1, info.to_packet()).ok();
}
