extern crate nix;
extern crate peripety;
extern crate regex;
extern crate sdjournal;
#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate libc;
extern crate toml;
extern crate uuid;

mod buildin_regex;
mod collector;
mod conf;
mod data;
mod fs;
mod mpath;
mod scsi;

use chrono::{Local, SecondsFormat};
use conf::ConfMain;
use data::{EventType, ParserInfo};
use libc::{c_char, size_t};
use peripety::{BlkInfo, LogSeverity, StorageEvent, StorageSubSystem};
use std::ffi::CStr;
use std::io::{self, Write};
use std::mem;
use std::process::exit;
use std::ptr;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{sleep, Builder};
use std::time::Duration;
use uuid::Uuid;

fn send_to_journald(event: &StorageEvent) {
    let mut logs = Vec::new();
    logs.push(("IS_PERIPETY".to_string(), "TRUE".to_string()));
    logs.push(("PRIORITY".to_string(), format!("{}", event.severity as u8)));
    if !event.msg.is_empty() {
        logs.push(("MESSAGE".to_string(), event.msg.clone()));
    }
    let blk_info = &event.blk_info;
    logs.push(("DEV_WWID".to_string(), blk_info.wwid.clone()));
    logs.push(("DEV_PATH".to_string(), blk_info.blk_path.clone()));
    for owner_blk_info in &blk_info.owners {
        logs.push(("OWNERS_WWIDS".to_string(), owner_blk_info.wwid.clone()));
        logs.push((
            "OWNERS_PATHS".to_string(),
            owner_blk_info.blk_path.clone(),
        ));
    }

    for (key, value) in &event.extension {
        logs.push((format!("EXT_{}", key.to_uppercase()), value.clone()));
    }
    logs.push(("EVENT_TYPE".to_string(), event.event_type.clone()));
    logs.push(("EVENT_ID".to_string(), event.event_id.clone()));
    logs.push(("SUB_SYSTEM".to_string(), event.sub_system.to_string()));
    logs.push((
        "JSON".to_string(),
        event.to_json_string().expect("BUG: event.to_json_string()"),
    ));
    if let Err(e) = sdjournal::send_journal_list(&logs) {
        println!("Failed to save event to journald: {}", e);
    }
}

fn handle_events_from_parsers(
    recver: &Receiver<StorageEvent>,
    parsers: &[ParserInfo],
    daemon_conf_recv: &Receiver<ConfMain>,
) {
    let mut notify_stdout = false;
    let mut save_to_journald = true;
    loop {
        if let Ok(conf) = daemon_conf_recv.try_recv() {
            if let Some(v) = conf.notify_stdout {
                notify_stdout = v;
            } else {
                // Use default value
                notify_stdout = false;
            }
            if let Some(v) = conf.save_to_journald {
                save_to_journald = v;
            } else {
                // Use default value
                save_to_journald = true;
            }
        }

        let event = match recver.recv() {
            Ok(e) => e,
            Err(e) => {
                println!("Failed to receive event from parsers: {}", e);
                continue;
            }
        };

        // Send to stdout
        if notify_stdout {
            if let Ok(s) = event.to_json_string_pretty() {
                let _ = writeln!(&mut io::stdout(), "{}", s);
            }
        }

        // Send to journald.
        // TODO(Gris Ge): Invoke a thread of this in case sdjournal slows us.
        if save_to_journald {
            send_to_journald(&event);
        }

        // Send to parser if parser require it.
        for parser in parsers {
            let required =
                if parser.filter_event_type.contains(&EventType::Synthetic) {
                    match parser.filter_event_subsys {
                        None => true,
                        Some(ref syss) => syss.contains(&event.sub_system),
                    }
                } else {
                    false
                };
            if required {
                if let Err(e) = parser.sender.send(event.clone()) {
                    println!("Failed to send synthetic event to parser: {}", e);
                }
            }
        }
    }
}

fn collector_to_parsers(
    collector_recv: &Receiver<StorageEvent>,
    parsers: &[ParserInfo],
) {
    loop {
        match collector_recv.recv() {
            Ok(event) => {
                // Send to parser if parser require it.
                for parser in parsers {
                    let required = if parser
                        .filter_event_type
                        .contains(&EventType::Raw)
                    {
                        match parser.filter_event_subsys {
                            None => true,
                            Some(ref syss) => syss.contains(&event.sub_system),
                        }
                    } else {
                        false
                    };
                    if required {
                        if let Err(e) = parser.sender.send(event.clone()) {
                            println!(
                                "Failed to send event to parser {}: {}",
                                parser.name, e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to retrieve event from collector: {}", e);
                return;
            }
        }
    }
}

fn main() {
    let (collector_send, collector_recv) = mpsc::channel();
    let (notifier_send, notifier_recv) = mpsc::channel();
    let (conf_send, conf_recv) = mpsc::channel();
    let (daemon_conf_send, daemon_conf_recv) = mpsc::channel();
    let mut parsers: Vec<ParserInfo> = Vec::new();
    let mut dump_blk_info = true;

    let mut daemon_conf = None;
    let mut collector_conf = None;
    if let Some(c) = conf::load_conf() {
        if c.main.dump_blk_info_at_start == Some(false) {
            dump_blk_info = false;
        }
        daemon_conf = Some(c.main);
        collector_conf = Some(c.collector);
    }

    let sig_fd = unsafe {
        let mut mask: libc::sigset_t = mem::uninitialized();
        libc::sigemptyset(&mut mask);
        libc::sigaddset(&mut mask, libc::SIGHUP);
        libc::sigprocmask(libc::SIG_BLOCK, &mask, ptr::null_mut());
        libc::signalfd(
            -1, /* create new fd */
            &mask,
            0, /* no flag */
        )
    };
    if sig_fd < 0 {
        println!("BUG: Failed to handle SIGHUP signal");
        exit(1);
    }

    // 1. Start parser threads
    parsers.push(mpath::parser_start(notifier_send.clone()));
    parsers.push(scsi::parser_start(notifier_send.clone()));
    parsers.push(fs::parser_start(notifier_send.clone()));

    let parsers_clone = parsers.clone();

    // 2. Start thread for forwarding collector output to parsers.
    Builder::new()
        .name("collector_to_parser".into())
        .spawn(move || {
            collector_to_parsers(&collector_recv, &parsers);
        })
        .expect("Failed to start 'collector_to_parser' thread");

    // 3. Start thread for forwarding parsers output to parsers and notifier.
    Builder::new()
        .name("handle_events_from_parsers".into())
        .spawn(move || {
            handle_events_from_parsers(
                &notifier_recv,
                &parsers_clone,
                &daemon_conf_recv,
            );
        })
        .expect("Failed to start 'handle_events_from_parsers' thread");

    // TODO(Gris Ge): Need better way for waiting threads to be ready.
    sleep(Duration::from_secs(1));

    // 4. Start collector thread
    Builder::new()
        .name("collector".into())
        .spawn(move || {
            collector::new(&collector_send, &conf_recv);
        })
        .expect("Failed to start 'collector' thread");

    if let Some(c) = collector_conf {
        conf_send
            .send(c)
            .expect("Failed to send config to collector");
    }

    if let Some(c) = daemon_conf {
        daemon_conf_send
            .send(c)
            .expect("Failed to reload daemon config");
    }

    println!("Peripetyd: Ready!");

    if dump_blk_info {
        dump_blk_infos(&notifier_send);
    }

    let mut sig: libc::signalfd_siginfo = unsafe { mem::uninitialized() };
    let sig_size = std::mem::size_of::<libc::signalfd_siginfo>();

    loop {
        let mut fds = nix::sys::select::FdSet::new();
        fds.insert(sig_fd);
        if let Err(e) =
            nix::sys::select::select(None, Some(&mut fds), None, None, None)
        {
            println!("collector: Failed select against signal fd: {}", e);
            continue;
        }
        if fds.contains(sig_fd) {
            unsafe {
                if libc::read(
                    sig_fd,
                    &mut sig as *mut _ as *mut libc::c_void,
                    sig_size,
                ) != sig_size as isize
                    || sig.ssi_signo != libc::SIGHUP as u32
                {
                    continue;
                }
            }
        }

        if let Some(c) = conf::load_conf() {
            println!("Config reloaded");
            if let Err(e) = conf_send.send(c.collector) {
                println!("failed to send config to collector: {}", e);
                continue;
            }
            if let Err(e) = daemon_conf_send.send(c.main) {
                println!("Failed to reload daemon config: {}", e);
                continue;
            }
        }
    }
}

const HOST_NAME_MAX: usize = 64;

fn gethostname() -> String {
    let mut buf = [0u8; HOST_NAME_MAX];
    let rc = unsafe {
        libc::gethostname(
            buf.as_mut_ptr() as *mut c_char,
            HOST_NAME_MAX as size_t,
        )
    };

    if rc != 0 {
        panic!("Failed to gethostname(): error {}", rc);
    }

    unsafe {
        CStr::from_bytes_with_nul_unchecked(&buf)
            .to_str()
            .expect("Got invalid utf8 from gethostname()")
            .trim_right_matches('\0')
            .to_string()
    }
}

fn dump_blk_infos(notifier_send: &Sender<StorageEvent>) {
    let blk_infos = BlkInfo::list().expect("Failed to query existing blocks");
    for blk_info in blk_infos {
        let msg = match blk_info.mount_point {
            Some(ref mount_point) => format!(
                "Found block '{}' '{}' mounted at '{}'",
                &blk_info.blk_path, &blk_info.wwid, &mount_point
            ),
            None => format!("Found block '{}' '{}'", &blk_info.blk_path,
                            &blk_info.wwid),
        };
        let mut se: StorageEvent = Default::default();
        se.hostname = gethostname();
        se.severity = LogSeverity::Info;
        se.sub_system = StorageSubSystem::Peripety;
        se.timestamp =
            Local::now().to_rfc3339_opts(SecondsFormat::Micros, false);
        se.event_id = Uuid::new_v4().hyphenated().to_string();
        se.event_type = "PERIPETY_BLK_INFO".to_string();
        se.blk_info = blk_info;
        se.msg = msg.clone();
        se.raw_msg = msg;

        notifier_send
            .send(se)
            .expect("Failed to send block information event");
    }
}
