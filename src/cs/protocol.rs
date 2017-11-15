use network::Packet;
use network::Protocol;
use network::*;

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

    fn to_packet(&self) -> Packet {
        Packet::from_protocol(0xC1, 11, 0x01, self)
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

    fn to_packet(&self) -> Packet {
        Packet::from_protocol(0xC1, 4, 0x02, self)
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

    fn to_packet(&self) -> Packet {
        Packet::from_protocol(0xC1, 1, 0x00, self)
    }
}
