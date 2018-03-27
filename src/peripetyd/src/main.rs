use std::net::{Ipv4Addr, UdpSocket};
use std::thread::{sleep, spawn};
use std::time::Duration;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;

fn out_multicast(reciever: Receiver<String>) {
    let addr: Ipv4Addr = "127.0.0.1".parse().unwrap();
    let mcast_group: Ipv4Addr = "239.0.0.1".parse().unwrap();
    let port: u16 = 6000;
    let so = UdpSocket::bind(format!("{}:{}", addr, 0)).unwrap();
    so.join_multicast_v4(&mcast_group, &addr).unwrap();
    loop {
        so.send_to(
            reciever.recv().unwrap().as_bytes(),
            format!("{}:{}", mcast_group, port),
        ).unwrap();
    }
}

fn main() {
    let (out_mc_send, out_mc_recv) = mpsc::channel();

    spawn(move || {
        out_multicast(out_mc_recv);
    });

    let mut i: u32 = 0;
    loop {
        i += 1;
        sleep(Duration::from_secs(1));
        out_mc_send.send(format!("Hello {}", i)).unwrap();
    }
}
