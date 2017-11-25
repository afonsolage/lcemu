use std::fmt;
use protocol::Protocol;
use protocol::ProtoMsg;
use protocol::SUB_CODE_PKTS;

#[derive(Debug)]
pub enum MuPacketError {
    BufferTooSmall,
}

#[derive(Debug, Clone)]
pub struct MuPacket {
    kind: u8,
    sz: u16,
    pub code: u8,
    pub sub_code: u8,
    pub data: Vec<u8>,
}

impl MuPacket {
    fn has_sub_code(code: &u8) -> bool {
        SUB_CODE_PKTS.contains(code)
    }

    pub fn new(buffer: &[u8]) -> Option<MuPacket> {
        let kind = buffer[0];

        let mut n = 1;

        let sz = match kind {
            0xC1 => buffer[n] as u16,
            0xC2 => ((buffer[n] as u16) << 8) | buffer[n + 1] as u16,
            _ => {
                println!("Unkown header received: {:02X}", kind);
                return None;
            },
        };

        n += if kind == 0xC1 { 1 } else { 2 };

        let code = buffer[n] as u8;
        n += 1;

        let mut sub_code = 0u8;
        if MuPacket::has_sub_code(&code) {
            sub_code = buffer[n];
            n += 1;
        }

        Some(MuPacket {
            kind: kind,
            sz: sz,
            code: code,
            sub_code: sub_code,
            data: buffer[n..].to_vec(),
        })
    }

    pub fn from_protocol<T: Protocol>(msg: ProtoMsg, proto: &T) -> MuPacket {
        let (kind, code, subcode) = msg.parse();
        let len = proto.len();
        let mut v = vec![0; len as usize];
        proto.serialize(&mut v);
        MuPacket {
            kind: kind,
            sz: len + MuPacket::header_len(&kind, &code),
            code: code,
            sub_code: subcode,
            data: v,
        }
    }

    pub fn header_len(kind: &u8, code: &u8) -> u16 {
        let res = match *kind {
            0xC1 => 2, //Header and size u8
            0xC2 => 3, //Header and size u16
            _ => panic!("Unsupported!"),
        };

        if MuPacket::has_sub_code(&code) {
            res + 2
        } else {
            res + 1
        }
    }

    pub fn len(&self) -> usize {
        return MuPacket::header_len(&self.kind, &self.code) as usize + self.data.len();
    }

    pub fn serialize(&self, buf: &mut [u8]) -> Result<usize, MuPacketError> {
        if buf.len() < self.len() {
            return Err(MuPacketError::BufferTooSmall);
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

        if self.sub_code > 0 {
            buf[idx] = self.sub_code;
            idx += 1;
        }

        buf[idx..self.len()].clone_from_slice(&self.data);

        Ok(self.len())
    }
}

impl fmt::Display for MuPacket {
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

        if self.sub_code > 0 {
            write!(f, "{:02X} ", self.sub_code)?;
        }

        for b in &self.data {
            write!(f, "{:02X} ", b)?;
        }
        Ok(())
    }
}
