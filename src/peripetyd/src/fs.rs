extern crate peripety;
extern crate regex;

use data::{EventType, ParserInfo, BlkInfo};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use peripety::{StorageEvent, StorageSubSystem};
use std::thread::spawn;
use std::fs;
use std::path::Path;

fn uuid_of_blk(blk_path: &str) -> String {
    for entry in fs::read_dir("/dev/disk/by-uuid").unwrap() {
        let path = entry.unwrap().path();
        if let Ok(p) = fs::read_link(&path) {
            let link_path =
                format!("/dev/disk/by-uuid/{}", p.to_str().unwrap());
            let link_path = Path::new(&link_path).canonicalize().unwrap();
            let cur_path = link_path.to_str().unwrap();
            if cur_path == blk_path {
                return path.file_name().unwrap().to_str().unwrap().to_string();
            }
        }
    }
    String::new()
}

fn parse_event(event: &StorageEvent, sender: &Sender<StorageEvent>) {
    let mut event = event.clone();
    if let Some(blk_info) = BlkInfo::new(&event.kdev) {
        event.dev_wwid = uuid_of_blk(&blk_info.blk_path);
        if event.dev_wwid.len() == 0 {
            return;
        }
        event.dev_name = blk_info.name.clone();
        event.dev_path = blk_info.blk_path.clone();
        event.holders_wwids = blk_info.holders_wwids;
        event.holders_names = blk_info.holders_names;
        event.holders_paths = blk_info.holders_paths;
        if event.holders_wwids.len() == 0 {
            event.holders_wwids.push(blk_info.wwid);
            event.holders_names.push(blk_info.name);
            event.holders_paths.push(blk_info.blk_path);
        }

        sender.send(event).unwrap();
    }
}

pub fn parser_start(sender: Sender<StorageEvent>) -> ParserInfo {
    let (event_in_sender, event_in_recver) = mpsc::channel();
    let name = "fs".to_string();
    let filter_event_type = vec![EventType::Raw];
    let filter_event_subsys = vec![StorageSubSystem::FsExt4];

    spawn(move || loop {
        let event = event_in_recver.recv().unwrap();
        parse_event(&event, &sender);
    });

    ParserInfo {
        sender: event_in_sender,
        name: name,
        filter_event_type: filter_event_type,
        filter_event_subsys: Some(filter_event_subsys),
    }
}
