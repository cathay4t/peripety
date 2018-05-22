extern crate chan_signal;
extern crate nix;
extern crate peripety;
extern crate regex;
extern crate sdjournal;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate chrono;
extern crate libc;

mod collector;
mod conf;
mod data;
mod dm;
mod fs;
mod mpath;
mod scsi;

use chan_signal::Signal;
use conf::ConfMain;
use data::{EventType, ParserInfo};
use peripety::StorageEvent;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread::{sleep, Builder};
use std::time::Duration;

fn send_to_journald(event: &StorageEvent) {
    let mut logs = Vec::new();
    logs.push(("IS_PERIPETY".to_string(), "TRUE".to_string()));
    logs.push((
        "PRIORITY".to_string(),
        format!("{}", event.severity as u8),
    ));
    if event.msg.len() != 0 {
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
    for (key, value) in &event.extention {
        logs.push((
            format!("EXT_{}", key.to_uppercase()),
            value.clone(),
        ));
    }
    logs.push((
        "EVENT_TYPE".to_string(),
        event.event_type.clone(),
    ));
    logs.push(("EVENT_ID".to_string(), event.event_id.clone()));
    logs.push((
        "SUB_SYSTEM".to_string(),
        event.sub_system.to_string(),
    ));
    logs.push((
        "JSON".to_string(),
        event
            .to_json_string()
            .expect("BUG: event.to_json_string()"),
    ));
    if let Err(e) = sdjournal::send_journal_list(&logs) {
        println!("Failed to save event to journald: {}", e);
    }
}

fn handle_events_from_parsers(
    recver: &Receiver<StorageEvent>,
    parsers: &Vec<ParserInfo>,
    daemon_conf: Option<ConfMain>,
) {
    let mut skip_stdout = true;
    if let Some(c) = daemon_conf {
        if c.notify_stdout == Some(true) {
            skip_stdout = false;
        }
    }
    loop {
        let event = match recver.recv() {
            Ok(e) => e,
            Err(e) => {
                println!("Failed to receive event from parsers: {}", e);
                continue;
            }
        };

        // Send to stdout
        if !skip_stdout {
            if let Ok(s) = event.to_json_string_pretty() {
                println!("{}", s);
            }
        }

        // Send to journald.
        // TODO(Gris Ge): Invoke a thread of this in case sdjournal slows us.
        send_to_journald(&event);

        // Send to parser if parser require it.
        for parser in parsers {
            let required = match parser
                .filter_event_type
                .contains(&EventType::Synthetic)
            {
                true => match parser.filter_event_subsys {
                    None => true,
                    Some(ref syss) => syss.contains(&event.sub_system),
                },
                false => false,
            };
            if required {
                if let Err(e) = parser.sender.send(event.clone()) {
                    println!(
                        "Failed to send synthetic event to parser: {}",
                        e
                    );
                }
            }
        }
    }
}

fn collector_to_parsers(
    collector_recv: &Receiver<StorageEvent>,
    parsers: &Vec<ParserInfo>,
) {
    loop {
        match collector_recv.recv() {
            Ok(event) => {
                // Send to parser if parser require it.
                for parser in parsers {
                    let required = match parser
                        .filter_event_type
                        .contains(&EventType::Raw)
                    {
                        true => match parser.filter_event_subsys {
                            None => true,
                            Some(ref syss) => syss.contains(&event.sub_system),
                        },
                        false => false,
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
    let mut parsers: Vec<ParserInfo> = Vec::new();

    let mut daemon_conf = None;
    let mut collector_conf = None;
    if let Some(c) = conf::load_conf() {
        daemon_conf = Some(c.main);
        collector_conf = Some(c.collector);
    }

    let conf_changed_signal = chan_signal::notify(&[Signal::HUP]);

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
                daemon_conf,
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

    println!("Peripetyd: Ready!");

    loop {
        if let None = conf_changed_signal.recv() {
            println!("Failed to recv() from signal channel");
            continue;
        }
        if let Some(c) = conf::load_conf() {
            if let Err(e) = conf_send.send(c.collector) {
                println!("Failed to send config to collector: {}", e);
                continue;
            }
        }
    }
}
