use std::collections::{HashMap, HashSet};
use std::time::{Instant, Duration};
use config;
use network::prelude::*;

struct GSInstance {
    pub svr_code: u16,
    pub ip: [u8; 16],
    pub port: u16,
    pub perc: u8,
    pub usr_cnt: u16,
    pub acc_cnt: u16,
    pub pcbng_cnt: u16,
    pub mx_usr_cnt: u16,
    pub last_info: Instant,
}

impl GSInstance {
    pub fn load(&self) -> u8 {
        match self.mx_usr_cnt {
            0 => 0,
            _ => (self.usr_cnt / self.mx_usr_cnt) as u8,
        }
    }

    pub fn alive(&self) -> bool {
        self.last_info.elapsed().as_secs() < 10
    }
}

pub struct Handler {
    gs_map: HashMap<u16, GSInstance>,
    clients: HashSet<u32>,
}

impl Handler {
    pub fn new() -> Handler {
        Handler {
            gs_map: HashMap::new(),
            clients: HashSet::new(),
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
                Some(str_addr) => {
                    if str_addr.len() >= 16 {
                        panic!("Invalid addr on section: {}", key);
                    }

                    let mut addr = [0u8; 16];
                    addr[..str_addr.len()].copy_from_slice(&str_addr.as_bytes());
                    addr
                }
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
                ip: addr,
                port: port,
                perc: 0,
                usr_cnt: 0,
                acc_cnt: 0,
                pcbng_cnt: 0,
                mx_usr_cnt: 0,
                last_info: Instant::now() - Duration::new(10, 0), //Creates a instance 10 seconds ago, which is the timeout of keep alive, so the instance is inited as "dead"
            };

            self.gs_map.insert(gs.svr_code, gs);
        }

        if self.gs_map.len() == 0 {
            panic!("No GS settings found on config file. Please add at least one.");
        } else {
            println!("Loaded {} gs config(s).", self.gs_map.len());
        }
    }

    pub fn handle(&mut self, evt: Event, svr: &Server) {
        match evt {
            Event::ClientConnected(id) => self.on_client_connected(id, svr),
            Event::ClientDisconnected(id) => self.on_client_disconnected(id, svr),
            Event::ClientPacket(pkt) => self.on_packet_received(pkt, svr),
        };
    }

    fn broadcast(&self, pkt: Packet, svr: &Server) -> Vec<u32> {
        let mut errs: Vec<u32> = vec![];
        for &id in self.clients.iter() {
            match svr.post_packet(id, pkt.clone()) {
                Err(_) => errs.push(id),
                Ok(_) => (),
            };
        }

        errs
    }

    fn on_client_connected(&mut self, id: u32, svr: &Server) {
        let res = ConnectResult { res: 1 };

        match svr.post_packet(id, res.to_packet()) {
            Err(_) => svr.disconnect(id),
            _ => match self.send_server_list(id, svr) {
                Err(_) => svr.disconnect(id),
                _ => Ok(()),
            },
        }.ok();

        self.clients.insert(id);
    }

    fn on_client_disconnected(&mut self, id: u32, _: &Server) {
        println!("Client disconnected {}", id);
        self.clients.remove(&id);
    }

    fn on_packet_received(&mut self, pkt: Packet, svr: &Server) {
        match pkt.code {
            0x01 => self.on_server_info(ServerInfo::parse(&pkt.data), svr),
            0x02 => (), //JoinServerStat
            _ => println!("Unhandled packet: {}", pkt),
        };
    }

    fn on_server_info(&mut self, msg: ServerInfo, svr: &Server) {
        let code = msg.svr_code;

        {
            let info = match self.gs_map.get_mut(&code) {
                None => {
                    println!(
                        "Received info from gs {}, but there is config for it!.",
                        code
                    );
                    return;
                }
                Some(r) => r,
            };

            info.perc = msg.perc;
            info.usr_cnt = msg.usr_cnt;
            info.acc_cnt = msg.acc_cnt;
            info.pcbng_cnt = msg.pcbng_cnt;
            info.mx_usr_cnt = msg.mx_usr_cnt;
            info.last_info = Instant::now();
        }

        match self.new_server_list_pkt() {
            None => (),
            Some(pkt) => {
                for id in self.broadcast(pkt, svr) {
                    svr.disconnect(id).ok();
                }
            }
        }
    }

    fn new_server_list_pkt(&self) -> Option<Packet> {
        let mut list = ServerList::new(self.gs_map.len() as u16);

        let mut at_least_one = false;
        for (_, info) in self.gs_map.iter() {
            if info.alive() {
                list.add(info.svr_code, info.load());
                at_least_one = true;
            }
        }

        if at_least_one {
            Some(list.to_packet())
        } else {
            None
        }
    }

    fn send_server_list(&mut self, id: u32, svr: &Server) -> Result<(), Error> {
        match self.new_server_list_pkt() {
            None => Ok(()),
            Some(pkt) => svr.post_packet(id, pkt),
        }
    }
}
