use std::fmt;
use protocol::Protocol;

#[derive(Debug)]
pub enum PacketError {
    BufferTooSmall,
}

#[derive(Debug)]
pub struct Packet {
    kind: u8,
    sz: u16,
    pub code: u8,
    data: Vec<u8>,
}

impl Packet {
    pub fn new(buffer: &[u8]) -> Packet {
        match buffer[0] {
            0xC1 => Packet::new_c1(&buffer[1..]),
            0xC2 => Packet::new_c2(&buffer[1..]),
            _ => panic!("Unsupported packet type received!"),
        }
    }

    pub fn new_c1(buffer: &[u8]) -> Packet {
        Packet {
            kind: 0xC1,
            sz: buffer[0] as u16,
            code: buffer[1],
            data: buffer[2..].to_vec(),
        }
    }

    pub fn new_c2(buffer: &[u8]) -> Packet {
        Packet {
            kind: 0xC2,
            sz: ((buffer[0] as u16) << 8) | buffer[1] as u16,
            code: buffer[2],
            data: buffer[3..].to_vec(),
        }
    }

    pub fn from_protocol<T: Protocol>(kind: u8, len: u16, code: u8, proto: &T) -> Packet {
        let mut v = vec![0; len as usize];
        proto.serialize(&mut v);
        Packet {
            kind: kind,
            sz: len + if kind == 0xC1 { 3 } else { 4 },
            code: code,
            data: v,
        }
    }

    pub fn header_len(&self) -> usize {
        match self.kind {
            0xC1 => 3,
            0xC2 => 4,
            _ => panic!("Unsupported!"),
        }
    }

    pub fn len(&self) -> usize {
        return self.header_len() + self.data.len();
    }

    pub fn serialize(&self, buf: &mut [u8]) -> Result<usize, PacketError> {
        if buf.len() < self.len() {
            return Err(PacketError::BufferTooSmall);
        }

        let mut idx = 0;

        buf[idx] = self.kind;
        idx += 1;

        match self.kind {
            0xC1 => {
                buf[idx] = self.sz as u8;
                idx += 1;
            }
            0xC2 => {
                buf[idx] = (self.sz >> 8) as u8;
                buf[idx + 1] = (self.sz) as u8;
                idx += 2;
            }
            _ => panic!("Unsupported!"),
        };

        buf[idx] = self.code;
        idx += 1;
        
        buf[idx..self.len()].clone_from_slice(&self.data);

        Ok(self.len())
    }
}

impl fmt::Display for Packet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02X} ", self.kind)?;

        match self.kind {
            0xC1 => {
                write!(f, "{:02X} ", self.sz as u8)?;
            }
            0xC2 => {
                write!(f, "{:02X} ", (self.sz >> 8) as u8)?;
                write!(f, "{:02X} ", (self.sz) as u8)?;
            }
            _ => panic!("Unsupported!"),
        };

        write!(f, "{:02X} ", self.code)?;

        for b in &self.data {
            write!(f, "{:02X} ", b)?;
        }
        Ok(())
    }
}
