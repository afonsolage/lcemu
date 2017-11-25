extern crate util;

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
}

pub struct ServerInfo {
    pub svr_code: u16,
    pub perc: u8,
    pub usr_cnt: u16,
    pub acc_cnt: u16,
    pub pcbng_cnt: u16,
    pub mx_usr_cnt: u16,
}

impl Protocol for ServerInfo {
    fn parse(buf: &[u8]) -> Self {
        ServerInfo {
            svr_code: get_u16(&buf[0..2]),
            perc: buf[2],
            usr_cnt: get_u16(&buf[3..5]),
            acc_cnt: get_u16(&buf[5..7]),
            pcbng_cnt: get_u16(&buf[7..9]),
            mx_usr_cnt: get_u16(&buf[9..11]),
        }
    }

    fn serialize(&self, buf: &mut [u8]) {
        set_u16(&mut buf[0..2], self.svr_code);
        buf[2] = self.perc;
        set_u16(&mut buf[3..5], self.usr_cnt);
        set_u16(&mut buf[5..7], self.acc_cnt);
        set_u16(&mut buf[7..9], self.pcbng_cnt);
        set_u16(&mut buf[9..11], self.mx_usr_cnt);
    }

    fn size(&self) -> u16 {
        11
    }
}

pub struct JoinServerStat {
    pub queue_cnt: u32,
}

impl Protocol for JoinServerStat {
    fn parse(buf: &[u8]) -> Self {
        JoinServerStat {
            queue_cnt: get_u32(&buf[0..4]),
        }
    }

    fn serialize(&self, buf: &mut [u8]) {
        set_u32(&mut buf[0..2], self.queue_cnt);
    }

    fn size(&self) -> u16 {
        4
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
}
