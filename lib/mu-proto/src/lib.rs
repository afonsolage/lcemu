#![feature(proc_macro, conservative_impl_trait, generators)]
#![feature(const_atomic_usize_new)]

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate failure;
#[macro_use] extern crate failure_derive;

extern crate futures_await as futures;

mod server;
mod protocol;
mod packet;
mod tcp_session;

pub use server::{Server, NetworkError, NetworkEvent};
pub use protocol::*;
pub use packet::{MuPacket, MuPacketError};