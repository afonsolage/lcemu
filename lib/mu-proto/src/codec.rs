extern crate bytes;
extern crate tokio_io;
extern crate tokio_proto;

use self::tokio_io::codec::{Decoder, Encoder};
use self::bytes::BytesMut;
use super::packet::MuPacket;

use std::io;

pub struct MuCodec;

impl Decoder for MuCodec {
    type Item = MuPacket;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<MuPacket>, io::Error> {
        //3 is the minimum size to work with a packet
        if buf.len() < 3 {
            return Ok(None);
        }

        let kind = buf[0] as u8;
        let sz = match kind {
            0xC1 => buf[1] as u16,
            0xC2 => ((buf[1] as u16) << 8) | (buf[2] as u16),
            _ => panic!("Unhandled packet type: {:02X}", kind),
        } as usize;

        if buf.len() < sz {
            return Ok(None);
        }

        let pkt = MuPacket::new(&buf[0..sz]);
        buf.split_to(sz);

        Ok(Some(pkt))
    }
}

impl Encoder for MuCodec {
    type Item = MuPacket;
    type Error = io::Error;

    fn encode(&mut self, msg: MuPacket, buf: &mut BytesMut) -> io::Result<()> {
        buf.reserve(msg.len());

        msg.serialize(buf).unwrap();

        Ok(())
    }
}
