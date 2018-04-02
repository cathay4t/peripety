extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::io::{Read, Write};
use std::str;

const IPC_HDR_LEN: usize = 10;
static SENDER_SOCKET_FILE: &'static str = "/var/run/peripety/senders";
static PARSER_SOCKET_FILE_PREFIX: &'static str = "/var/run/peripety/parser_";

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
// https://tools.ietf.org/html/rfc5424#section-6.2.1
pub enum LogSeverity {
    Emergency = 0,
    Alert = 1,
    Ctritical = 2,
    Error = 3,
    Warning = 4,
    Notice = 5,
    Info = 6,
    Debug = 7,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum StorageSubSystem {
    Scsi,
    LvmThin,
    Multipath,
    Block,
    Fs,
    Mdraid,
    Other,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageEvent {
    pub hostname: String,
    pub severity: LogSeverity,
    pub sub_system: StorageSubSystem,
    pub timestamp: u64,
    pub event_id: String,
    pub event_type: String,
    pub dev_wwid: String,
    pub dev_name: String,
    pub msg: String,
    pub extention: HashMap<String, String>,
}

impl Default for StorageEvent {
    fn default() -> StorageEvent {
        StorageEvent {
            hostname: String::new(),
            severity: LogSeverity::Debug,
            sub_system: StorageSubSystem::Unknown,
            timestamp: 0,
            event_id: String::new(),
            event_type: String::new(),
            dev_wwid: String::new(),
            dev_name: String::new(),
            msg: String::new(),
            extention: HashMap::new(),
        }
    }
}

impl StorageEvent {
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
    pub fn from_json_string(json_string: &str) -> StorageEvent {
        serde_json::from_str(json_string).unwrap()
    }
    pub fn from_slice(buff: &[u8]) -> StorageEvent {
        // We cannot use serde_json::from_slice, as buff might have trailing \0
        // where serde_json will raise error.
        let tmp_s = str::from_utf8(buff).unwrap().trim_right_matches('\0');
        serde_json::from_str(tmp_s).unwrap()
    }
}

pub struct Ipc {}

impl Ipc {
    pub fn ipc_send(mut stream: &UnixStream, msg: &str) {
        let msg =
            format!("{:0padding$}{}", msg.len(), msg, padding = IPC_HDR_LEN);
        stream.write_all(msg.as_bytes()).unwrap();
    }

    pub fn ipc_recv(mut stream: &UnixStream) -> String {
        let mut msg_buff = [0u8; IPC_HDR_LEN];
        stream.read_exact(&mut msg_buff).unwrap();
        let msg_len =
            str::from_utf8(&msg_buff).unwrap().parse::<usize>().unwrap();
        let mut msg = vec![0u8; msg_len];
        stream.read_exact(msg.as_mut_slice()).unwrap();
        String::from_utf8(msg).unwrap()
    }

    pub fn sender_ipc() -> UnixStream {
        UnixStream::connect(SENDER_SOCKET_FILE).unwrap()
    }

    pub fn parser_ipc(name: &str) -> UnixStream {
        UnixStream::connect(format!("{}{}", PARSER_SOCKET_FILE_PREFIX, name))
            .unwrap()
    }
}
