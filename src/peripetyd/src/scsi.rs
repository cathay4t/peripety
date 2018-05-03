extern crate peripety;
extern crate regex;

//mod data;

use data::{BlkInfo, BlkType, EventType, ParserInfo, Sysfs};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use peripety::{StorageEvent, StorageSubSystem};
use std::thread::spawn;
use std::path::Path;
use regex::Regex;

// Support query on these formats:
//  * 4:0:0:1
//  * sda
//  * sda1
pub fn blk_info_get_scsi(kdev: &str) -> Option<BlkInfo> {
    let name;

    // Check if partition
    if let Ok(reg) = Regex::new("^(sd[a-z]+)([0-9]+)$") {
        if let Some(cap) = reg.captures(kdev) {
            let name = cap.get(1).unwrap().as_str();
            let part = cap.get(2).unwrap().as_str();
            if let Some(blk_info) = blk_info_get_scsi(&name) {
                return Some(BlkInfo {
                    wwid: format!("{}-part{}", blk_info.wwid, part),
                    blk_type: BlkType::Partition,
                    blk_path: format!("/dev/{}", &kdev),
                    name: kdev.to_string(),
                    owners_wwids: vec![blk_info.wwid],
                    owners_types: vec![BlkType::Scsi],
                    owners_names: vec![blk_info.name],
                    owners_paths: vec![blk_info.blk_path],
                });
            } else {
                return None;
            }
        }
    }

    // Try 4:0:0:1 format
    let mut sysfs_path = format!("/sys/class/scsi_disk/{}/device/wwid", &kdev);
    if Path::new(&sysfs_path).exists() {
        name = Sysfs::scsi_id_to_blk_name(kdev);
    } else {
        // Try sda format
        sysfs_path = format!("/sys/block/{}/device/wwid", &kdev);
        name = kdev.to_string();
    }

    if Path::new(&sysfs_path).exists() {
        return Some(BlkInfo {
            wwid: pretty_wwid(&Sysfs::read(&sysfs_path)),
            blk_type: BlkType::Scsi,
            blk_path: format!("/dev/{}", &name),
            name: name,
            owners_wwids: Vec::new(),
            owners_types: Vec::new(),
            owners_names: Vec::new(),
            owners_paths: Vec::new(),
        });
    }

    // TODO(Gris Ge): Handle partition

    None
}

fn pretty_wwid(wwid: &str) -> String {
    Regex::new(r"[ \t]+")
        .map(|r| r.replace_all(wwid.trim(), "-"))
        .unwrap()
        .to_string()
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
    if let Some(b) = blk_info_get_scsi(kdev) {
        let mut event = event.clone();
        event.dev_name = b.name;
        event.dev_path = b.blk_path;
        event.dev_wwid = b.wwid;
        event
            .extention
            .insert("scsi_id".to_string(), kdev.to_string());

        sender.send(event).unwrap();
    }
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
