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
    Closed,
}

pub struct TcpSession<T> {
    _io: PhantomData<T>,
}

pub struct TcpSessionReader<T> {
    io: T,
    pub id: u32,
    buf: [u8; 10_240]
}

pub struct TcpSessionWriter<T> {
    io: T,
    pub id: u32,
}

impl<T> TcpSession<T>
where
    T: AsyncRead + AsyncWrite,
{
    pub fn new_pair(io: T, id: u32) -> (TcpSessionReader<ReadHalf<T>>, TcpSessionWriter<WriteHalf<T>>) {
        let (r, w) = io.split();
        (
            TcpSessionReader { io: r, id: id, buf: [0; 10_240] },
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
                    Ok(Async::NotReady)
                } else {
                    println!("IO Read Error: {:?}", e);
                    Err(TcpSessionError::TcpStreamRead)
                }
            }
            Ok(0) => Ok(Async::Ready(None)),
            Ok(n) => Ok(Async::Ready(MuPacket::new(&self.buf[0..n]))),
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
        if item.is_empty() {
            return Err(TcpSessionError::Closed);
        }

        let mut buf = vec![0; item.len()];

        if item.serialize(&mut buf).is_err() {
            return Err(TcpSessionError::SerializationError);
        }

        match self.io.write(&buf) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(AsyncSink::NotReady(item))
                } else {
                    Err(TcpSessionError::TcpStreamWrite)
                }
            }
            Ok(_) => {
                Ok(AsyncSink::Ready)
            }
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match self.io.flush() {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(Async::NotReady)
                } else {
                    Err(TcpSessionError::TcpStreamFlush)
                }
            }
            Ok(()) => {
                Ok(Async::Ready(()))
            }
        }
    }
}
