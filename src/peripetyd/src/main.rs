extern crate nix;
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
use std::collections::HashMap;
use nix::poll::PollFd;
use std::os::unix::io::AsRawFd;

static IPC_DIR: &'static str = "/var/run/peripety";

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

fn handle_collector_plugin_ipc(mut stream: UnixStream, sender: Sender<String>) {
    loop {
        sender.send(Ipc::ipc_recv(&mut stream)).unwrap();
    }
}

fn socket_for_collector_plugins(sender: Sender<String>) {
    if !Path::new(IPC_DIR).is_dir() {
        fs::create_dir(IPC_DIR)
            .expect(&format!("Failed to create dir '{}'", IPC_DIR));
    }
    let ipc_file = format!("{}/senders", IPC_DIR);
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
                    handle_collector_plugin_ipc(stream, new_sender);
                });
            }
            Err(_) => {
                /* connection failed */
                break;
            }
        }
    }
}

fn start_collector_plugins() {
    let cur_dir = std::env::current_exe().unwrap();
    let cur_dir = cur_dir.parent().and_then(|p| p.to_str()).unwrap();
    let kmsg_path = format!("{}/{}", cur_dir, "kmsg");
    Command::new(kmsg_path).spawn().unwrap();
}

fn collector_msg_to_parsers(
    collector_recv: Receiver<String>,
    parsers_so: &Vec<UnixStream>,
) {
    loop {
        let msg: String = collector_recv.recv().unwrap();
        for so in parsers_so {
            //TODO(Gris Ge): Plugins might wait on other slow plugin.
            //               Plugins should use queue to hold the
            //               events instead of blocking daemon.
            Ipc::ipc_send(&so, &msg);
        }
    }
}

fn parser_msg_to_daemon(
    parser_send: Sender<String>,
    parsers_so: &Vec<UnixStream>,
) {
    let mut poll_fds: Vec<PollFd> = Vec::new();
    let mut so_fd_hash = HashMap::new();
    for so in parsers_so {
        let fd = so.as_raw_fd();
        so_fd_hash.insert(fd, so);
        poll_fds.push(nix::poll::PollFd::new(
            so.as_raw_fd(),
            nix::poll::EventFlags::POLLIN,
        ));
    }
    loop {
        let fd = nix::poll::poll(&mut poll_fds, -1).unwrap();
        parser_send.send(Ipc::ipc_recv(
            *so_fd_hash.get_mut(&fd).unwrap()
                )).unwrap();
    }
}

fn start_parser_plugin(name: &str) -> UnixStream {
    let ipc_file = format!("{}/parser_{}", IPC_DIR, name);
    let cur_dir = std::env::current_exe().unwrap();
    let cur_dir = cur_dir.parent().and_then(|p| p.to_str()).unwrap();
    let plugin_path = format!("{}/{}", cur_dir, name);

    let listener = UnixListener::bind(ipc_file).unwrap();
    Command::new(plugin_path).spawn().unwrap();
    listener.incoming().next().unwrap().unwrap()
}

fn start_parser_plugins() -> Vec<UnixStream> {
    let mut rc: Vec<UnixStream> = Vec::new();
    rc.push(start_parser_plugin("mpath"));
    rc
}

fn main() {
    let (out_mc_send, out_mc_recv) = mpsc::channel();
    let (collector_send, collector_recv) = mpsc::channel();
    let (parser_send, parser_recv) = mpsc::channel();

    spawn(move || {
        multicast_for_receiver_plugins(out_mc_recv);
    });

    spawn(move || {
        socket_for_collector_plugins(collector_send);
    });

    // Wait for sender socket ready.
    collector_recv.recv().unwrap();

    start_collector_plugins();

    let parsers_so = start_parser_plugins();
    let mut parsers_so_dup = Vec::new();
    for so in &parsers_so {
        parsers_so_dup.push(so.try_clone().unwrap());
    }

    spawn(move || {
        collector_msg_to_parsers(collector_recv, &parsers_so);
    });

    spawn(move || {
        parser_msg_to_daemon(parser_send, &parsers_so_dup);
    });

    loop {
        let msg: String = parser_recv.recv().unwrap();
        out_mc_send.send(msg).unwrap();
    }
}
