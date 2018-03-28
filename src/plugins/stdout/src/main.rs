extern crate net2;

use std::net::SocketAddr;
use std::str;
use net2::UdpBuilder;
use net2::unix::UnixUdpBuilderExt;

fn main() {
    let mut buff = [0u8; 4096];
    let addr = SocketAddr::from(([239, 0, 0, 1], 6000));
    let so = UdpBuilder::new_v4().unwrap();
    so.reuse_port(true).unwrap();
    let so = so.bind(addr).unwrap();
    loop {
        so.recv_from(&mut buff).unwrap();
        println!("got: {}", str::from_utf8(&buff).unwrap());
    }
}
