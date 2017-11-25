#![feature(proc_macro, conservative_impl_trait, generators)]
#![feature(const_atomic_usize_new)]
extern crate futures_await as futures;

mod server;
mod codec;
mod protocol;
mod packet;
mod tcp_session;

pub use server::{Server, NetworkError};
pub use codec::MuCodec;
pub use protocol::*;
pub use packet::{MuPacket, MuPacketError};