extern crate peripety;

use std::net::{Ipv4Addr, UdpSocket};
use std::thread::spawn;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::fs;
use peripety::Ipc;
use std::thread;
use std::process::Command;

fn multicast_for_receiver_plugins(reciever: Receiver<String>) {
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

fn handle_sender_plugin_ipc(mut stream: UnixStream, sender: Sender<String>) {
    loop {
        sender.send(Ipc::ipc_recv(&mut stream)).unwrap();
    }
}

fn socket_for_sender_plugins(sender: Sender<String>) {
    let ipc_dir = "/var/run/peripety".to_string();
    if !Path::new(&ipc_dir).is_dir() {
        fs::create_dir(&ipc_dir)
            .expect(&format!("Failed to create dir '{}'", ipc_dir));
    }
    let ipc_file = format!("{}/senders", ipc_dir);
    if Path::new(&ipc_file).exists() {
        fs::remove_file(&ipc_file).unwrap();
    }
    let listener = UnixListener::bind(ipc_file).unwrap();
    sender.send("socket ready".to_string()).unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let new_sender = sender.clone();
                /* connection succeeded */
                thread::spawn(move || {
                    handle_sender_plugin_ipc(stream, new_sender);
                });
            }
            Err(_) => {
                /* connection failed */
                break;
            }
        }
    }
}

fn start_sender_plugins() {
    let cur_dir = std::env::current_exe().unwrap();
    let cur_dir = cur_dir.parent()
        .and_then(|p| p.to_str())
        .unwrap();
    let kmsg_path = format!("{}/{}", cur_dir, "kmsg");
    Command::new(kmsg_path).spawn().unwrap();
}

fn main() {
    let (out_mc_send, out_mc_recv) = mpsc::channel();
    let (sender_send, sender_recv) = mpsc::channel();

    spawn(move || {
        multicast_for_receiver_plugins(out_mc_recv);
    });

    spawn(move || {
        socket_for_sender_plugins(sender_send);
    });

    // Wait for sender socket ready.
    sender_recv.recv().unwrap();

    start_sender_plugins();

    loop {
        let msg: String = sender_recv.recv().unwrap();
        out_mc_send.send(msg).unwrap();
    }
}
