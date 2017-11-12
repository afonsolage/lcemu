use std::fmt;

#[derive(Debug)]
pub enum PacketType {
    C1(C1Packet),
    C2(C2Packet),
}

#[derive(Debug)]
pub enum PacketError {
    BufferTooSmall,
}

#[derive(Debug)]
pub struct C1Packet {
    sz: u8,
    code: u8,
    data: Vec<u8>,
}

impl C1Packet {
    pub fn new(buffer: &[u8]) -> C1Packet {
        C1Packet {
            sz: buffer[0],
            code: buffer[1],
            data: buffer[2..].to_vec(),
        }
    }

    pub fn code(&self) -> u8 {
        return self.code;
    }

    pub fn sub_code(&self) -> u8 {
        return self.data[0];
    }

    pub fn data(&self) -> &[u8] {
        return &self.data;
    }

    pub fn len(&self) -> usize {
        return self.data.len() + 2;
    }

    pub fn serialize(&self, buf: &mut [u8]) -> Result<usize, PacketError> {
        if buf.len() < self.len() {
            return Err(PacketError::BufferTooSmall);
        }

        buf[0] = 0xC1;
        buf[1] = self.sz;
        buf[2] = self.code;
        buf[3..].clone_from_slice(&self.data);

        Ok(self.len())
    }
}

impl fmt::Display for C1Packet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "C2")?;
        for b in &self.data {
            write!(f, "{:X}", b)?;
        }
        Ok(())
    }
}




#[derive(Debug)]
pub struct C2Packet {
    sz: u16,
    code: u8,
    data: Vec<u8>,
}

impl C2Packet {
    pub fn new(buffer: &[u8]) -> C2Packet {
        C2Packet {
            sz: (((buffer[0] as u16) << 8) | buffer[1] as u16),
            code: buffer[1],
            data: buffer[2..].to_vec(),
        }
    }

    pub fn code(&self) -> u8 {
        return self.code;
    }

    pub fn data(&self) -> &[u8] {
        return &self.data;
    }

    pub fn len(&self) -> usize {
        return self.data.len() + 3;
    }

    pub fn serialize(&self, buf: &mut [u8]) -> Result<usize, PacketError> {
        if buf.len() < self.len() {
            return Err(PacketError::BufferTooSmall);
        }

        buf[0] = 0xC2;
        buf[1] = (self.sz >> 8) as u8;
        buf[1] = (self.sz & 0xFF) as u8;
        buf[3] = self.code;
        buf[4..].clone_from_slice(&self.data);

        Ok(self.len())
    }
}

impl fmt::Display for C2Packet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "C2")?;
        for b in &self.data {
            write!(f, "{:X}", b)?;
        }
        Ok(())
    }
}
