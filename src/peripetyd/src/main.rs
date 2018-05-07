extern crate chan_signal;
extern crate nix;
extern crate peripety;
extern crate regex;
extern crate sdjournal;

mod collector;
mod mpath;
mod data;
mod scsi;
mod fs;
mod dm;

use data::{EventType, ParserInfo};
use peripety::StorageEvent;
use std::thread::{sleep, spawn};
use std::time::Duration;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use chan_signal::Signal;

fn send_to_journald(event: &StorageEvent) {
    let mut logs = Vec::new();
    logs.push(("IS_PERIPETY".to_string(), "TRUE".to_string()));
    logs.push(("PRIORITY".to_string(), format!("{}", event.severity as u8)));
    logs.push(("MESSAGE".to_string(), event.msg.clone()));
    logs.push(("DEV_WWID".to_string(), event.dev_wwid.clone()));
    logs.push(("DEV_NAME".to_string(), event.dev_name.clone()));
    logs.push(("DEV_PATH".to_string(), event.dev_path.clone()));
    for owners_wwid in &event.owners_wwids {
        logs.push(("OWNERS_WWIDS".to_string(), owners_wwid.clone()));
    }
    for owners_name in &event.owners_names {
        logs.push(("OWNERS_NAMES".to_string(), owners_name.clone()));
    }
    for owners_path in &event.owners_paths {
        logs.push(("OWNERS_PATHS".to_string(), owners_path.clone()));
    }
    for (key, value) in &event.extention {
        logs.push((format!("EXT_{}", key.to_uppercase()), value.clone()));
    }
    logs.push(("EVENT_TYPE".to_string(), event.event_type.clone()));
    logs.push(("EVENT_ID".to_string(), event.event_id.clone()));
    logs.push(("SUB_SYSTEM".to_string(), event.sub_system.to_string()));
    sdjournal::send_journal_list(&logs).unwrap();
}

fn handle_events_from_parsers(
    recver: &Receiver<StorageEvent>,
    parsers: &Vec<ParserInfo>,
) {
    loop {
        let event = recver.recv().unwrap();

        // Send to stdout
        println!("{}", event.to_json_string_pretty());

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
                parser.sender.send(event.clone()).unwrap();
            }
        }
    }
}

fn collector_to_parsers(
    collector_recv: &Receiver<StorageEvent>,
    parsers: &Vec<ParserInfo>,
) {
    loop {
        let event = collector_recv.recv().unwrap();
        // Send to parser if parser require it.
        for parser in parsers {
            let required =
                match parser.filter_event_type.contains(&EventType::Raw) {
                    true => match parser.filter_event_subsys {
                        None => true,
                        Some(ref syss) => syss.contains(&event.sub_system),
                    },
                    false => false,
                };
            if required {
                parser.sender.send(event.clone()).unwrap();
            }
        }
    }
}

fn main() {
    let (collector_send, collector_recv) = mpsc::channel();
    let (notifier_send, notifier_recv) = mpsc::channel();
    let (conf_send, conf_recv) = mpsc::channel();
    let mut parsers: Vec<ParserInfo> = Vec::new();

    let conf_changed_signal = chan_signal::notify(&[Signal::HUP]);

    // 1. Start parser threads
    parsers.push(mpath::parser_start(notifier_send.clone()));
    parsers.push(scsi::parser_start(notifier_send.clone()));
    parsers.push(fs::parser_start(notifier_send.clone()));

    let parsers_clone = parsers.clone();

    // 2. Start thread for forwarding collector output to parsers.
    spawn(move || {
        collector_to_parsers(&collector_recv, &parsers);
    });

    // 3. Start thread for forwarding parsers output to parsers and notifier.
    spawn(move || {
        handle_events_from_parsers(&notifier_recv, &parsers_clone);
    });

    // TODO(Gris Ge): Need better way for waiting threads to be ready.
    sleep(Duration::from_secs(1));

    // 4. Start collector thread
    spawn(move || {
        collector::new(&collector_send, &conf_recv);
    });

    println!("Peripetyd: Ready!");

    loop {
        conf_changed_signal.recv().unwrap();
        conf_send.send(true).unwrap();
    }
}
