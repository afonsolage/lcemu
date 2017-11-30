use std::collections::HashMap;
use mu_proto::prelude::*;
use failure::Error;
use futures::{Future, Poll, Async, Stream};

struct GSInstance {
    pub svr_code: u16,
    pub ip: [u8; 16],
    pub port: u16,
    pub perc: u8,
    pub usr_cnt: u16,
    pub acc_cnt: u16,
    pub mx_usr_cnt: u16,
}

impl GSInstance {
    pub fn load(&self) -> u8 {
        match self.mx_usr_cnt {
            0 => 0,
            _ => (self.usr_cnt / self.mx_usr_cnt) as u8,
        }
    }
}

pub struct Handler<T: Stream> {
    gs_map: HashMap<u16, GSInstance>,
    clients: HashMap<u32, SessionRef>,
    io: T,
}

impl<T> Handler<T>
where
    T: Stream<Item = NetworkEvent, Error = Error>,
{
    pub fn new(t: T) -> Handler<T> {
        Handler {
            gs_map: HashMap::new(),
            clients: HashMap::new(),
            io: t,
        }
    }

    pub fn handle_net_event(&mut self, evt: NetworkEvent) {
        match evt {
            NetworkEvent::ClientConnected(session) => self.on_client_connected(session),
            NetworkEvent::ClientDisconnected(id) => self.on_client_disconnected(id),
            NetworkEvent::ClientPacket((session, pkt)) => self.on_packet_received(session, pkt),
            _ => println!("Unkown event: {:?}", evt),
        };
    }

    fn broadcast(&mut self, pkt: MuPacket) {
        for (_, session) in &mut self.clients {
            if session.send(pkt.clone()).is_err() {
                session.close().ok();
            }
        }
    }

    fn on_client_connected(&mut self, mut session: SessionRef) {
        let res = ConnectResult { res: 1 };

        if session.send(res.to_packet()).is_err() {
            session.close().ok();
            return;
        }

        if self.send_server_list(&mut session).is_err() {
            session.close().ok();
            return;
        }

        self.clients.insert(session.id, session);
    }

    fn on_client_disconnected(&mut self, id: u32) {
        println!("Client disconnected {}", id);
        self.clients.remove(&id);
    }

    fn on_packet_received(&mut self, session: SessionRef, pkt: MuPacket) {
        match pkt.code {
            0x01 => self.on_server_info(ServerInfo::parse(&pkt.data), session),
            0x02 => (), //JoinServerStat
            _ => println!("Unhandled packet: {}", pkt),
        };
    }

    fn on_server_info(&mut self, msg: ServerInfo, _session: SessionRef) {
        let code = msg.svr_code;

        let mut opt = None;

        match self.gs_map.get_mut(&code) {
            None => {
                let info = GSInstance {
                    svr_code: code,
                    ip: msg.ip,
                    port: msg.port,
                    perc: msg.perc,
                    usr_cnt: msg.usr_cnt,
                    acc_cnt: msg.acc_cnt,
                    mx_usr_cnt: msg.mx_usr_cnt,
                };
                opt = Some(info);
            }
            Some(info) => {
                info.perc = msg.perc;
                info.usr_cnt = msg.usr_cnt;
                info.acc_cnt = msg.acc_cnt;
            }
        };

        if let Some(info) = opt {
            self.gs_map.insert(code, info);
        }

        if let Some(pkt) = self.new_server_list_pkt() {
            self.broadcast(pkt);
        }
    }

    fn new_server_list_pkt(&self) -> Option<MuPacket> {
        let mut list = ServerList::new(self.gs_map.len() as u16);

        let mut cnt = 0;
        for (_, info) in self.gs_map.iter() {
            list.add(info.svr_code, info.load());
            cnt += 1;
        }

        list.cnt = cnt;

        Some(list.to_packet())
    }

    fn send_server_list(&mut self, session: &mut SessionRef) -> Result<(), Error> {
        if let Some(pkt) = self.new_server_list_pkt() {
            session.send(pkt)?
        }

        Ok(())
    }

    fn process(&mut self, evt: NetworkEvent) -> Poll<(), Error> {
        Ok(Async::Ready(()))
    }
}

impl<T> Future for Handler<T>
where
    T: Stream<Item = NetworkEvent, Error = Error>,
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.io.poll()? {
                Async::Ready(Some(evt)) => try_ready!(self.process(evt)),
                Async::Ready(None) => break Ok(Async::Ready(())),
                Async::NotReady => break Ok(Async::NotReady),
            }
        }
    }
}
