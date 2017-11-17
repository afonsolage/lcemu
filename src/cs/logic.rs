use std::collections::HashMap;
use config;
use network::prelude::*;
use std::net::SocketAddr;

struct GSInstance {
    pub svr_code: u16,
    pub ip: [u8; 16],
    pub port: u16,
    pub perc: u8,
    pub usr_cnt: u16,
    pub acc_cnt: u16,
    pub pcbng_cnt: u16,
    pub mx_usr_cnt: u16,
}

pub struct Handler {
    gs_map: HashMap<u16, GSInstance>,
}

impl Handler {
    pub fn new() -> Handler {
        Handler {
            gs_map: HashMap::new(),
        }
    }

    pub fn setup(&mut self, settings: &config::Config) {
        let map = settings
            .clone()
            .try_into::<HashMap<String, HashMap<String, String>>>()
            .unwrap();

        for (key, val) in map.iter() {
            if !key.starts_with("gs-") {
                continue;
            }

            if !val.contains_key("addr") {
                panic!(
                    "Failed to parse config. Section {} doesnt have addr config.",
                    key
                );
            }

            if !val.contains_key("port") {
                panic!(
                    "Failed to parse config. Section {} doesnt have port config.",
                    key
                );
            }

            let code = key.split("-").collect::<Vec<_>>();

            if code.len() != 2 {
                panic!("Failed to parse config, invalid section name: {}", key);
            }

            let code = code[1];
            let code = match code.parse::<u16>() {
                Err(_) => panic!("Failed to parse config, invalid section name: {}", key),
                Ok(r) => r,
            };

            let addr = match val.get("addr") {
                None => panic!(
                    "Failed to parse config. Section {} doesnt have addr config.",
                    key
                ),
                Some(r) => r,
            };

            let port = match val.get("port") {
                None => panic!(
                    "Failed to parse config. Section {} doesnt have port config.",
                    key
                ),
                Some(r) => match r.parse::<u16>() {
                    Err(_) => panic!("Failed to parse config, invalid port on section: {}", key),
                    Ok(r) => r,
                },
            };

            let gs = GSInstance {
                svr_code: code,
                ip: [0; 16],
                port: port,
                perc: 0,
                usr_cnt: 0,
                acc_cnt: 0,
                pcbng_cnt: 0,
                mx_usr_cnt: 0,
            };
        }
    }

    pub fn handle(&self, evt: Event, srv: &Server) {
        match evt {
            Event::ClientConnected(id) => self.on_client_connected(id, srv),
            Event::ClientDisconnected(id) => self.on_client_disconnected(id, srv),
            Event::ClientPacket(pkt) => self.on_packet_received(pkt, srv),
        };
    }

    fn on_client_connected(&self, id: u32, srv: &Server) {
        let res = ConnectResult { res: 1 };
        srv.post_packet(id, res.to_packet()).ok();
    }

    fn on_client_disconnected(&self, id: u32, _: &Server) {
        println!("Client disconnected {}", id)
    }

    fn on_packet_received(&self, pkt: Packet, _: &Server) {
        match pkt.code {
            0x02 => (), //JoinServerStat
            _ => println!("Unhandled packet: {}", pkt),
        };
    }
}
