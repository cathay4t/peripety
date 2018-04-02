extern crate net2;
extern crate peripety;

use std::net::SocketAddr;
use net2::UdpBuilder;
use net2::unix::UnixUdpBuilderExt;
use peripety::StorageEvent;

fn main() {
    let mut buff = [0u8; 4096];
    let addr = SocketAddr::from(([239, 0, 0, 1], 6000));
    let so = UdpBuilder::new_v4().unwrap();
    so.reuse_port(true).unwrap();
    let so = so.bind(addr).unwrap();
    loop {
        so.recv_from(&mut buff).unwrap();
        let se = StorageEvent::from_slice(&buff);
        println!("got: {:?}", se);
        buff = [0u8; 4096];
    }
}
