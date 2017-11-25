extern crate futures;
extern crate tokio_core;
extern crate tokio_io;

use super::MuPacket;
use self::tokio_io::{AsyncRead, AsyncWrite};
use self::tokio_io::io::{ReadHalf, WriteHalf};
use self::futures::{Async, AsyncSink, Poll, Sink, StartSend, Stream};

use std::io;
use std::io::{Read, Write};
use std::marker::PhantomData;

#[derive(Debug)]
pub enum TcpSessionError {
    TcpStreamRead,
    TcpStreamWrite,
    TcpStreamFlush,
    SerializationError,
}

pub struct TcpSession<T> {
    _io: PhantomData<T>,
}

pub struct TcpSessionReader<T> {
    io: T,
    pub id: u32,
    buf: [u8; 10240]
}

pub struct TcpSessionWriter<T> {
    io: T,
    pub id: u32,
}

impl<T> TcpSession<T>
where
    T: AsyncRead + AsyncWrite,
{
    pub fn new(io: T, id: u32) -> (TcpSessionReader<ReadHalf<T>>, TcpSessionWriter<WriteHalf<T>>) {
        let (r, w) = io.split();
        (
            TcpSessionReader { io: r, id: id, buf: [0; 10240] },
            TcpSessionWriter { io: w, id: id },
        )
    }
}

impl<T> Stream for TcpSessionReader<ReadHalf<T>>
where
    T: AsyncRead,
{
    type Item = MuPacket;
    type Error = TcpSessionError;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.io.read(&mut self.buf) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    return Ok(Async::NotReady);
                } else {
                    println!("IO Read Error: {:?}", e);
                    return Err(TcpSessionError::TcpStreamRead);
                }
            }
            Ok(0) => Ok(Async::Ready(None)),
            Ok(n) => return Ok(Async::Ready(MuPacket::new(&self.buf[0..n]))),
        }
    }
}

impl<T> Sink for TcpSessionWriter<WriteHalf<T>>
where
    T: AsyncWrite,
{
    type SinkItem = MuPacket;
    type SinkError = TcpSessionError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let mut buf = vec![0; item.len()];

        match item.serialize(&mut buf) {
            Err(_) => return Err(TcpSessionError::SerializationError),
            Ok(_) => (),
        };

        match self.io.write(&buf) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    return Ok(AsyncSink::NotReady(item));
                } else {
                    return Err(TcpSessionError::TcpStreamWrite);
                }
            }
            Ok(_) => {
                return Ok(AsyncSink::Ready);
            }
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match self.io.flush() {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    return Ok(Async::NotReady);
                } else {
                    return Err(TcpSessionError::TcpStreamFlush);
                }
            }
            Ok(()) => {
                return Ok(Async::Ready(()));
            }
        }
    }
}
