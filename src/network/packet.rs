pub struct Packet {
    sz: u8,
    code: u8,
    data: Vec<u8>,
}

impl Packet {
    pub fn new(buffer: &[u8]) -> Packet {
        Packet {
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
}