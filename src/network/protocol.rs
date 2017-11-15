use super::Packet;

pub fn get_u32(buf: &[u8]) -> u32 {
    (buf[0] as u32) << 24 | (buf[1] as u32) << 16 | (buf[2] as u32) << 8 | buf[3] as u32
}

pub fn set_u32(buf: &mut [u8], val: u32) {
    buf[0] = ((val >> 24) & 0xFF) as u8;
    buf[1] = ((val >> 16) & 0xFF) as u8;
    buf[2] = ((val >> 8) & 0xFF) as u8;
    buf[3] = (val & 0xFF) as u8;
}

pub fn get_u16(buf: &[u8]) -> u16 {
    ((buf[0] as u16) << 8) | buf[1] as u16
}

pub fn set_u16(buf: &mut [u8], val: u16) {
    buf[0] = ((val >> 8) & 0xFF) as u8;
    buf[1] = (val & 0xFF) as u8;
}


pub trait Protocol {
    fn parse(&[u8]) -> Self;
    fn serialize(&self, &mut [u8]);
    fn to_packet(&self) -> Packet;
}
