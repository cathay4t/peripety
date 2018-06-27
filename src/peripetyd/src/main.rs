extern crate nix;
extern crate peripety;
extern crate regex;
extern crate sdjournal;
#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate libc;
extern crate toml;

mod buildin_regex;
mod collector;
mod conf;
mod data;
mod fs;
mod mpath;
mod scsi;

use conf::ConfMain;
use data::{EventType, ParserInfo};
use peripety::StorageEvent;
use std::io::{self, Write};
use std::mem;
use std::process::exit;
use std::ptr;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread::{sleep, Builder};
use std::time::Duration;

fn send_to_journald(event: &StorageEvent) {
    let mut logs = Vec::new();
    logs.push(("IS_PERIPETY".to_string(), "TRUE".to_string()));
    logs.push(("PRIORITY".to_string(), format!("{}", event.severity as u8)));
    if !event.msg.is_empty() {
        logs.push(("MESSAGE".to_string(), event.msg.clone()));
    }
    logs.push(("DEV_WWID".to_string(), event.dev_wwid.clone()));
    logs.push(("DEV_PATH".to_string(), event.dev_path.clone()));
    for owners_wwid in &event.owners_wwids {
        logs.push(("OWNERS_WWIDS".to_string(), owners_wwid.clone()));
    }
    for owners_path in &event.owners_paths {
        logs.push(("OWNERS_PATHS".to_string(), owners_path.clone()));
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

    let mut daemon_conf = None;
    let mut collector_conf = None;
    if let Some(c) = conf::load_conf() {
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
            &mut mask,
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
