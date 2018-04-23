extern crate nix;
extern crate peripety;
extern crate regex;

mod collector;
mod mpath;
mod data;
mod scsi;

use data::{EventType, ParserInfo};
use peripety::StorageEvent;
use std::thread::spawn;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::{thread, time};

fn handle_events_from_parsers(
    recver: &Receiver<StorageEvent>,
    parsers: &Vec<ParserInfo>,
) {
    loop {
        let event = recver.recv().unwrap();

        // Send to stdout
        println!("{}", event.to_json_string());

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
    let mut parsers: Vec<ParserInfo> = Vec::new();

    // 1. Start parser threads
    parsers.push(mpath::parser_start(notifier_send.clone()));
    parsers.push(scsi::parser_start(notifier_send.clone()));

    let parsers_clone = parsers.clone();

    // 2. Forward collector output to parsers.
    spawn(move || {
        collector_to_parsers(&collector_recv, &parsers);
    });

    // 3. Forward parsers output to parsers and notifier.
    spawn(move || {
        handle_events_from_parsers(&notifier_recv, &parsers_clone);
    });

    // 4. Start collector thread
    spawn(move || {
        collector::new(&collector_send);
    });

    loop {
        //TODO(Gris Ge): Maybe we should monitor on configuration file changes.
        //               or check threads status.
        thread::sleep(time::Duration::from_secs(600));
    }
}
