extern crate peripety;

//mod data;

use data::{EventType, ParserInfo};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use peripety::{StorageEvent, StorageSubSystem};
use std::thread::spawn;

fn parse_event(event: &StorageEvent, sender: &Sender<StorageEvent>) {
    println!("mpath:::parse_event: {:?}", &event);
}

pub fn parser_start(sender: Sender<StorageEvent>) -> ParserInfo {
    let (event_in_sender, event_in_recver) = mpsc::channel();
    let name = "mpath".to_string();
    let filter_event_type = vec![EventType::Raw];
    let filter_event_subsys = vec![StorageSubSystem::Multipath];

    spawn(move || {
        loop {
            let event = event_in_recver.recv().unwrap();
            parse_event(&event, &sender);
        }
    });

    ParserInfo {
        sender: event_in_sender,
        name: name,
        filter_event_type: filter_event_type,
        filter_event_subsys: Some(filter_event_subsys),
    }
}
