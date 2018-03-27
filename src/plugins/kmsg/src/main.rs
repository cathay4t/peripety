extern crate chrono;
extern crate libc;
extern crate nix;
extern crate peripety;
extern crate regex;

use nix::fcntl::OFlag;
use std::str;
use std::collections::HashMap;
use regex::Regex;
use peripety::{LogSeverity, StorageEvent, StorageSubSystem};
use std::mem;
use chrono::prelude::*;

#[repr(u8)]
enum SyslogFacility {
    Kernel = 0,
}

#[derive(Debug)]
struct Kmsg {
    severity: u8, // 3 lowest bits of syslog prefix number.
    facility: u8, // higher bits of syslog prefix number.
    sequence: u64,
    montonic_microseconds: u64,
    flag: char,
    msg: String,
    dict: HashMap<String, String>,
}

impl Default for Kmsg {
    fn default() -> Kmsg {
        Kmsg {
            severity: LogSeverity::Debug as u8,
            facility: SyslogFacility::Kernel as u8,
            sequence: 0,
            montonic_microseconds: 0,
            flag: '-',
            msg: String::new(),
            dict: HashMap::new(),
        }
    }
}

//TODO(Gris Ge): Handle the flag of kmsg
// The flags field carries '-' by default. A 'c' indicates a
// fragment of a line. All following fragments are flagged with
// '+'. Note, that these hints about continuation lines are not
// necessarily correct, and the stream could be interleaved with
// unrelated messages, but merging the lines in the output
// usually produces better human readable results. A similar
// logic is used internally when messages are printed to the
// console, /proc/kmsg or the syslog() syscall.
// In the future, the in-kernel concatenation may be removed entirely and
// /dev/kmsg users are recommended to implement fragment handling.

fn gen_kmsg(e: &str) -> Option<Kmsg> {
    let mut parsed: bool = false;
    let mut kmsg: Kmsg = Default::default();
    let re_line = Regex::new(
        r"(?x)
        ^
        (?P<prefix>[^;]+)
        # Kernel said future might add more comma separated values before the
        # terminating ';'. So, please use split on prefix in stead of do regex
        # capture here.
        ;
        (?P<msg>.+)
        $",
    ).unwrap();
    let re_subline = Regex::new(
        r"(?x)
        ^
        \s
        (?P<key>[^=]+)
        =
        (?P<value>.+)
        $",
    ).unwrap();
    for line in e.lines() {
        if !parsed {
            match re_line.captures(line) {
                Some(cap) => {
                    kmsg.msg =
                        cap.name("msg").map_or("", |m| m.as_str()).to_string();
                    let entries: Vec<&str> = cap.name("prefix")
                        .map_or("", |m| m.as_str())
                        .split(",")
                        .collect();
                    if entries.len() >= 4 {
                        let prefix: u8 = entries[0].parse().unwrap();
                        kmsg.severity = prefix & 0b111;
                        // 3 lowest bits of syslog prefix number
                        kmsg.facility = (prefix & 0b11111000) >> 3;
                        kmsg.sequence = entries[1].parse().unwrap();
                        kmsg.montonic_microseconds =
                            entries[2].parse().unwrap();
                        kmsg.flag = entries[3].parse().unwrap();
                    }
                    parsed = true;
                    continue;
                }
                None => (),
            }
        }
        match re_subline.captures(line) {
            Some(cap) => {
                kmsg.dict.insert(
                    cap.name("key").map_or("", |m| m.as_str()).to_string(),
                    cap.name("value").map_or("", |m| m.as_str()).to_string(),
                );
            }
            None => (),
        }
    }
    match parsed {
        true => {
            if kmsg.facility == SyslogFacility::Kernel as u8 {
                Some(kmsg)
            } else {
                None
            }
        }
        false => None,
    }
}

fn kmsg_to_storage_event(kmsg: Kmsg) -> Option<StorageEvent> {
    // We don't to extensive parsing here, it's other plugins' work.
    // We only set severity, sub_system, dev_name, msg.
    let mut se: StorageEvent = Default::default();
    match kmsg.dict.get("SUBSYSTEM") {
        Some(sub) => {
            match sub.as_ref() {
                "scsi" => {
                    se.sub_system = StorageSubSystem::Scsi;
                    match kmsg.dict.get("DEVICE") {
                        Some(dev) => {
                            if dev.starts_with("+scsi:") {
                                se.dev_name =
                                    dev.trim_left_matches("+scsi:").to_string();
                            }
                        }
                        None => return None,
                        // TODO(Gris Ge) Does SCSI all have DEVICE?
                    };
                }
                _ => return None,
            };
        }
        None => {
            // Do the hard work on finding sub system.
        }
    }
    if se.sub_system != StorageSubSystem::Unknown {
        se.severity = unsafe { mem::transmute(kmsg.severity) };
        se.msg = kmsg.msg;
        se.timestamp = Utc::now().timestamp() as u64;
        Some(se)
    } else {
        None
    }
}

fn send_event(se: &StorageEvent) {
    println!("{:?}", se);
}

fn main() {
    let fd = nix::fcntl::open(
        "/dev/kmsg",
        OFlag::O_RDONLY | OFlag::O_NONBLOCK,
        nix::sys::stat::Mode::empty(),
    ).unwrap();

    let mut hostname =
        [0u8; nix::unistd::SysconfVar::HOST_NAME_MAX as usize + 1];
    let hostname = nix::unistd::gethostname(&mut hostname)
        .unwrap()
        .to_str()
        .unwrap();

    nix::unistd::lseek(fd, 0, nix::unistd::Whence::SeekEnd).unwrap();
    let pool_fd = nix::poll::PollFd::new(fd, nix::poll::EventFlags::POLLIN);

    loop {
        let mut buff = [0u8; 8193];
        nix::poll::poll(&mut [pool_fd], -1).unwrap();
        match nix::unistd::read(fd, &mut buff) {
            Ok(l) => l,
            Err(e) => match e {
                nix::Error::Sys(errno) => {
                    println!("{}", errno);
                    break;
                }
                _ => 0usize,
            },
        };
        match gen_kmsg(str::from_utf8(&buff).unwrap()) {
            Some(kmsg) => match kmsg_to_storage_event(kmsg) {
                Some(mut se) => {
                    se.hostname = hostname.to_string();
                    send_event(&se);
                }
                None => continue,
            },
            None => (),
        }
    }
}
