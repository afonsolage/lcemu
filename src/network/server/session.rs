use std::io::prelude::*;
use std::sync::mpsc::Sender;
use std::net::TcpStream;
use std::thread;

use super::SessionEvent;

#[derive(Debug)]
pub enum SessionError {
    BufferTooSmall,
    FailedToReadNBytes,
    EosReached,
    C1ErrorGeneralFailure,
    C1ErrorEosReached,
    StreamCopyFailure,
    ThreadCreationFailure,
    StreamWriteFailure,
}

#[derive(Debug)]
pub struct TcpSession {
    pub id: u32,
    stream: TcpStream,
    rx_handler: thread::JoinHandle<()>,
}


impl TcpSession {
    pub fn read_n_bytes(
        out: &mut [u8],
        cnt: usize,
        stream: &mut TcpStream,
    ) -> Result<usize, SessionError> {
        if out.len() < cnt {
            return Err(SessionError::BufferTooSmall);
        }

        let mut total_read = 0;
        let mut tmp = [0];
        loop {
            match stream.read(&mut tmp) {
                Err(_) => return Err(SessionError::FailedToReadNBytes),
                Ok(read_cnt) => {
                    if read_cnt == 0 {
                        return Err(SessionError::EosReached);
                    }

                    out[total_read] = tmp[0];
                    total_read += read_cnt;
                    if total_read == cnt {
                        break;
                    }
                }
            };
        }

        Ok(total_read)
    }

    pub fn read_c1_data(out: &mut [u8], stream: &mut TcpStream) -> Result<usize, SessionError> {
        let mut sz_buf = [0];
        match stream.read(&mut sz_buf) {
            Err(_) => Err(SessionError::C1ErrorGeneralFailure),
            Ok(read_cnt) => {
                let c1_sz = sz_buf[0] as usize;
                if read_cnt == 0 {
                    Err(SessionError::C1ErrorEosReached)
                } else {
                    out[0] = sz_buf[0];
                    TcpSession::read_n_bytes(&mut out[1..], c1_sz, stream)
                }
            }
        }
    }

    fn close_client(srv_tx: &mut Sender<SessionEvent>, id: u32) {
        match srv_tx.send(SessionEvent::Disconnected(id)) {
            Err(why) => panic!("{:?}", why),
            _ => (),
        }
    }

    pub fn parse_packet(
        header: u8,
        out: &mut [u8],
        stream: &mut TcpStream,
    ) -> Result<usize, SessionError> {
        out[0] = header;
        match TcpSession::read_c1_data(&mut out[1..], stream) {
            Err(why) => Err(why),
            Ok(read_cnt) => {
                let len = out[1] as usize;
                let slice = &out[0..len];
                print!("Received: ");
                for byte in slice.into_iter() {
                    print!("{:02X} ", byte);
                }
                println!("");
                Ok(read_cnt)
            }
        }
    }

    fn main_loop(srv_tx: Sender<SessionEvent>, mut stream: TcpStream, id: u32) {
        let mut tx = srv_tx;
        let mut packet_buf = [0; 65536]; //64k
        let mut buf = [0];

        println!("Starting to read rx of session {}", id);

        loop {
            match stream.read(&mut buf) {
                Err(why) => {
                    println!("Failed to read stream: {:?}", why);
                    break;
                }
                Ok(0) => {
                    println!("EOF reached on session: {}", id);
                    break;
                }
                Ok(_) => match TcpSession::parse_packet(buf[0], &mut packet_buf, &mut stream) {
                    Err(why) => {
                        println!("Failed to read packet data: {:?}", why);
                        break;
                    }
                    Ok(cnt) => {
                        match tx.send(SessionEvent::PacketData(packet_buf[0..cnt].to_vec())) {
                            Err(why) => {
                                println!("Failed to send SessionEvent: {:?}", why);
                                break;
                            }
                            _ => (),
                        }
                    }
                },
            }
        }

        TcpSession::close_client(&mut tx, id);
    }

    pub fn new(
        id: u32,
        stream: TcpStream,
        srv_tx: Sender<SessionEvent>,
    ) -> Result<TcpSession, SessionError> {
        let rx_stream = match stream.try_clone() {
            Err(why) => {
                println!("Failed to clone stream: {:?}", why);
                return Err(SessionError::StreamCopyFailure);
            }
            Ok(stream) => stream,
        };

        let rx_handler = match thread::Builder::new()
            .name(format!("Session {} RX", id))
            .spawn(move || TcpSession::main_loop(srv_tx, rx_stream, id))
        {
            Err(why) => {
                println!("Failed to create session loop thread: {:?}", why);
                return Err(SessionError::ThreadCreationFailure);
            }
            Ok(handler) => handler,
        };

        Ok(TcpSession {
            id: id,
            stream: stream,
            rx_handler: rx_handler,
        })
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<(), SessionError> {
        match self.stream.write(buf) {
            Err(why) => {
                println!("Failed to write on stream: {:?}", why);
                Err(SessionError::StreamWriteFailure)
            }
            Ok(_) => Ok(()),
        }
    }
}
