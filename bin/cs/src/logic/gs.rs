use failure::Error;
use futures::Stream;
use mu_proto::prelude::*;

use logic::Handler;

pub struct GSInstance {
    pub s_ref: SessionRef,
    pub svr_code: u16,
    pub ip: [u8; 16],
    pub port: u16,
    pub perc: u8,
    pub usr_cnt: u16,
    pub acc_cnt: u16,
    pub mx_usr_cnt: u16,
}

impl GSInstance {
    pub fn load(&self) -> u8 {
        match self.mx_usr_cnt {
            0 => 0,
            _ => (self.usr_cnt / self.mx_usr_cnt) as u8,
        }
    }
}

impl<T> Handler<T>
where
    T: Stream<Item = NetworkEvent, Error = Error>,
{
    pub fn on_server_info(&mut self, msg: ServerInfo, session: SessionRef) {
        let id = session.id;
        let mut opt = None;

        match self.gs_map.get_mut(&id) {
            None => {
                let info = GSInstance {
                    s_ref: session,
                    svr_code: msg.svr_code,
                    ip: msg.ip,
                    port: msg.port,
                    perc: msg.perc,
                    usr_cnt: msg.usr_cnt,
                    acc_cnt: msg.acc_cnt,
                    mx_usr_cnt: msg.mx_usr_cnt,
                };
                opt = Some(info);
            }
            Some(info) => {
                info.perc = msg.perc;
                info.usr_cnt = msg.usr_cnt;
                info.acc_cnt = msg.acc_cnt;
            }
        };

        if let Some(info) = opt {
            self.gs_map.insert(id, info);
        }

        self.broadcast_server_list_upd();
    }

    pub fn broadcast_server_list_upd(&mut self) {
        if let Some(pkt) = self.new_server_list_pkt() {
            self.broadcast(pkt);
        }
    }

    pub fn new_server_list_pkt(&self) -> Option<MuPacket> {
        let mut list = ServerList::new(self.gs_map.len() as u16);

        let mut cnt = 0;
        for (_, info) in self.gs_map.iter() {
            list.add(info.svr_code, info.load());
            cnt += 1;
        }

        list.cnt = cnt;

        Some(list.to_packet())
    }

    pub fn send_server_list(&mut self, session: &mut SessionRef) -> Result<(), Error> {
        if let Some(pkt) = self.new_server_list_pkt() {
            session.send(pkt)?
        }

        Ok(())
    }

    pub fn on_server_disconnected(&mut self, id: u32) {
        self.gs_map.remove(&id);
        self.broadcast_server_list_upd();
    }

    pub fn on_server_connected(&mut self, session: SessionRef) {
        //
    }

    pub fn on_server_received(&mut self, session: SessionRef, pkt: MuPacket) {
        match pkt.code {
            0x01 => self.on_server_info(ServerInfo::parse(&pkt.data), session),
            0x02 => (), //JoinServerStat
            _ => println!("Unhandled packet: {}", pkt),
        };
    }
}
