extern crate peripety;
extern crate regex;

//mod data;

use data::{BlkInfo, BlkType, EventType, ParserInfo, Sysfs};
use peripety::{StorageEvent, StorageSubSystem};
use regex::Regex;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread::spawn;

// Support query on these formats:
//  * 4:0:0:1
//  * sda
//  * sda1
pub fn blk_info_get_scsi(kdev: &str) -> Option<BlkInfo> {
    let name;

    // Check if partition
    if let Ok(reg) = Regex::new("^(sd[a-z]+)([0-9]+)$") {
        if let Some(cap) = reg.captures(kdev) {
            let name = cap.get(1)
                .expect("BUG: blk_info_get_scsi()")
                .as_str();
            // We never panic as above regex is valid.
            let part = cap.get(2)
                .expect("BUG: blk_info_get_scsi()")
                .as_str();
            // We never panic as above regex is valid.
            if let Some(blk_info) = blk_info_get_scsi(&name) {
                return Some(BlkInfo {
                    wwid: format!("{}-part{}", blk_info.wwid, part),
                    blk_type: BlkType::Partition,
                    blk_path: format!("/dev/{}", &kdev),
                    owners_wwids: vec![blk_info.wwid],
                    owners_types: vec![BlkType::Scsi],
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
            owners_wwids: Vec::new(),
            owners_types: Vec::new(),
            owners_paths: Vec::new(),
        });
    }

    None
}

fn pretty_wwid(wwid: &str) -> String {
    let s = Regex::new(r"[ \t]+")
        .map(|r| r.replace_all(wwid.trim(), "-"))
        .expect("BUG: pretty_wwid()");
    // we never panic as above regex string is valid.
    Regex::new(r"(\\0)+$")
        .map(|r| r.replace_all(&s, ""))
        .expect("BUG: pretty_wwid()")
        // we never panic as above regex string is valid.
        .to_string()
}

fn parse_event(event: &StorageEvent, sender: &Sender<StorageEvent>) {
    let mut kdev: &str = &event.kdev;
    if event.kdev.starts_with("+scsi:") {
        kdev = &event.kdev["+scsi:".len()..];
    }
    if let Some(b) = blk_info_get_scsi(kdev) {
        let mut event = event.clone();
        event.dev_path = b.blk_path;
        event.dev_wwid = b.wwid;
        event
            .extension
            .insert("scsi_id".to_string(), kdev.to_string());

        if event.event_type == "SCSI_SENSE_KEY" {
            if let Some(sense_key) = event.extension.get("sense_key") {
                match sense_key.as_ref() {
                    // Find a way to use follow up CBD event to extract
                    // sector number of medium error.
                    "Medium Error" => {
                        event.event_type = "SCSI_MEDIUM_ERROR".to_string()
                    }
                    "Hardware Error" => {
                        event.event_type = "SCSI_HARDWARE_ERROR".to_string()
                    }
                    _ => {}
                }
            }
        }

        if let Err(e) = sender.send(event) {
            println!("scsi_parser: Failed to send event: {}", e);
        }
    }
}

pub fn parser_start(sender: Sender<StorageEvent>) -> ParserInfo {
    let (event_in_sender, event_in_recver) = mpsc::channel();
    let name = "scsi".to_string();
    let filter_event_type = vec![EventType::Raw];
    let filter_event_subsys = vec![StorageSubSystem::Scsi];

    spawn(move || loop {
        match event_in_recver.recv() {
            Ok(event) => parse_event(&event, &sender),
            Err(e) => println!("scsi_parser: Failed to receive event: {}", e),
        }
    });

    ParserInfo {
        sender: event_in_sender,
        name: name,
        filter_event_type: filter_event_type,
        filter_event_subsys: Some(filter_event_subsys),
    }
}
