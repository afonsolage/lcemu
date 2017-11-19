// extern crate futures;
extern crate mu_proto;
// extern crate tokio_core;
// extern crate tokio_io;
// extern crate tokio_service;
// extern crate tokio_proto;

// use futures::{future, Future, Sink, Stream};
use mu_proto::Server;
// use mu_proto::MuPacket;
// use mu_proto::MuCodec;
// use tokio_proto::multiplex::ServerProto;
// use tokio_service::{NewService, Service};
// use tokio_core::reactor::Core;
// use tokio_core::net::TcpListener;
// use tokio_io::codec::Framed;
// use tokio_io::{AsyncRead, AsyncWrite};

// use std::io;

// struct MyServer;

// impl Service for MyServer {
//     type Request = MuPacket;
//     type Response = MuPacket;
//     type Error = io::Error;
//     type Future = Box<Future<Item = MuPacket, Error = io::Error>>;
//     fn call(&self, input: MuPacket) -> Self::Future {
//         println!("Received pkt: {}", input);
//         Box::new(future::empty())
//     }
// }

// fn serve<S>(s: S) -> io::Result<()>
// where
//     S: NewService<Request = MuPacket, Response = MuPacket, Error = io::Error> + 'static,
// {
//     let mut core = Core::new()?;
//     let handle = core.handle();

//     let address = "0.0.0.0:44405".parse().unwrap();
//     let listener = TcpListener::bind(&address, &handle)?;

//     let connections = listener.incoming();
//     let server = connections.for_each(move |(socket, _peer_addr)| {
//         let (writer, reader) = socket.framed(MuCodec).split();
//         let service = s.new_service()?;

//         writer.and_then(|)
//         let responses = reader.and_then(move |req| service.call(req));
//         let server = writer.send_all(responses).then(|_| Ok(()));
//         handle.spawn(server);

//         Ok(())
//     });

//     core.run(server)
// }

fn main() {
    // if let Err(e) = serve(|| Ok(MyServer)) {
    //     println!("Server failed with {}", e);
    // }

    let mut svr = Server::new();
    svr.start_tcp("0.0.0.0", 44405).ok();
    svr.start();
}
