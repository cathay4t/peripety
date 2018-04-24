extern crate peripety;
extern crate regex;

//mod data;

use data::{EventType, ParserInfo, Sysfs};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use peripety::{StorageEvent, StorageSubSystem};
use std::thread::spawn;
use std::path::Path;
use regex::Regex;

fn pretty_wwid(wwid: &str) -> String {
    Regex::new(r"[ \t]+").map(|r|r.replace_all(wwid, "-")).unwrap().to_string()
}

fn parse_event(event: &StorageEvent, sender: &Sender<StorageEvent>) {
    if !event.kdev.starts_with("+scsi:") {
        println!(
            "scsi: Got unexpected kdev: {} for event {:?}",
            event.kdev, event
        );
        return;
    }
    let kdev = &event.kdev["+scsi:".len()..];
    let sysfs_path = format!("/sys/class/scsi_disk/{}/device/wwid", &kdev);
    if !Path::new(&sysfs_path).exists() {
        return;
    }
    let mut event = event.clone();
    event.dev_name = Sysfs::scsi_id_to_blk_name(kdev);
    event.dev_path = format!("/dev/{}", &event.dev_name);
    event.dev_wwid = pretty_wwid(&Sysfs::read(&sysfs_path));
    event
        .extention
        .insert("scsi_id".to_string(), kdev.to_string());

    sender.send(event).unwrap();
}

pub fn parser_start(sender: Sender<StorageEvent>) -> ParserInfo {
    let (event_in_sender, event_in_recver) = mpsc::channel();
    let name = "scsi".to_string();
    let filter_event_type = vec![EventType::Raw];
    let filter_event_subsys = vec![StorageSubSystem::Scsi];

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
