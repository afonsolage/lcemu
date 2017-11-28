extern crate util;

use super::MuPacket;
use self::util::*;

#[derive(Debug)]
pub enum ProtoMsg {
    ServerInfo,
    JoinServerStat,
    ConnectResult,
    ServerList,
}

pub const SUB_CODE_PKTS: [u8; 1] = [0xF4];

impl ProtoMsg {
    #[allow(unreachable_patterns)]
    pub fn parse(&self) -> (u8, u8, u8) {
        match *self {
            //Type (C1, C2, C3, C4)
            //Data Lenght (excluding header)
            //Code
            //Sub code (0x00 if none)
            ProtoMsg::ServerInfo => (0xC1, 0x01, 0x00),
            ProtoMsg::JoinServerStat => (0xC1, 0x02, 0x00),
            ProtoMsg::ConnectResult => (0xC1, 0x00, 0x00),
            ProtoMsg::ServerList => (0xC2, 0xF4, 0x06),
            _ => panic!("Unimplemented ProtoMsg: {:?}", *self),
        }
    }
}

pub trait Protocol {
    fn parse(&[u8]) -> Self;
    fn serialize(&self, &mut [u8]);
    fn size(&self) -> u16;
    fn to_packet(&self) -> MuPacket;
}

pub struct ServerInfo {
    pub svr_code: u16,
    pub ip: [u8; 16],
    pub port: u16,
    pub perc: u8,
    pub usr_cnt: u16,
    pub acc_cnt: u16,
    pub mx_usr_cnt: u16,
}

impl Protocol for ServerInfo {
    fn parse(buf: &[u8]) -> Self {
        ServerInfo {
            svr_code: get_u16(&buf[0..2]),
            ip: {
                let mut b = [0; 16];
                b.copy_from_slice(&buf[2..18]);
                b
            },
            port: get_u16(&buf[18..20]),
            perc: buf[20],
            usr_cnt: get_u16(&buf[21..23]),
            acc_cnt: get_u16(&buf[23..25]),
            mx_usr_cnt: get_u16(&buf[27..29]),
        }
    }

    fn serialize(&self, buf: &mut [u8]) {
        set_u16(&mut buf[0..2], self.svr_code);
        buf[2..18].copy_from_slice(&self.ip);
        set_u16(&mut buf[18..20], self.port);
        buf[20] = self.perc;
        set_u16(&mut buf[21..23], self.usr_cnt);
        set_u16(&mut buf[23..25], self.acc_cnt);
        set_u16(&mut buf[25..27], self.mx_usr_cnt);
    }

    fn size(&self) -> u16 {
        27
    }

    fn to_packet(&self) -> MuPacket {
        MuPacket::from_protocol(&ProtoMsg::ServerInfo, self)
    }
}

pub struct JoinServerStat {
    pub queue_cnt: u32,
}

impl Protocol for JoinServerStat {
    fn parse(buf: &[u8]) -> Self {
        JoinServerStat { queue_cnt: get_u32(&buf[0..4]) }
    }

    fn serialize(&self, buf: &mut [u8]) {
        set_u32(&mut buf[0..2], self.queue_cnt);
    }

    fn size(&self) -> u16 {
        4
    }

    fn to_packet(&self) -> MuPacket {
        MuPacket::from_protocol(&ProtoMsg::JoinServerStat, self)
    }
}

pub struct ConnectResult {
    pub res: u8,
}

impl Protocol for ConnectResult {
    fn parse(buf: &[u8]) -> Self {
        ConnectResult { res: buf[0] }
    }

    fn serialize(&self, buf: &mut [u8]) {
        buf[0] = self.res;
    }

    fn size(&self) -> u16 {
        1
    }

    fn to_packet(&self) -> MuPacket {
        MuPacket::from_protocol(&ProtoMsg::ConnectResult, self)
    }
}

pub struct ServerList {
    pub cnt: u16,
    data: Vec<(u16, u8)>,
}

impl ServerList {
    pub fn new(cnt: u16) -> Self {
        ServerList {
            cnt: cnt,
            data: vec![],
        }
    }

    pub fn add(&mut self, idx: u16, load: u8) {
        self.data.push((idx, load));
    }
}

impl Protocol for ServerList {
    fn parse(_: &[u8]) -> Self {
        panic!("not used");
    }

    fn serialize(&self, buf: &mut [u8]) {
        set_u16(&mut buf[0..2], self.cnt);

        let mut i = 2;
        for &(idx, load) in &self.data {
            set_u16(&mut buf[i..i + 2], idx); //The range end is exclusive.
            buf[i + 2] = load;
            buf[i + 3] = 0xFF; //Unkown data.

            i += 4;
        }
    }

    fn size(&self) -> u16 {
        2 + (self.data.len() * 4) as u16 //The size of each item is 4, because there is a fixed byte (0xCC)
    }

    fn to_packet(&self) -> MuPacket {
        MuPacket::from_protocol(&ProtoMsg::ServerList, self)
    }
}
