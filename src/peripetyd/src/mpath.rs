extern crate peripety;

//mod data;

use data::{EventType, ParserInfo, Sysfs};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use peripety::{StorageEvent, StorageSubSystem};
use std::thread::Builder;
use std::fs;

fn get_mpath_info_from_blk(major_minor: &str) -> Option<(String, String)> {
    // We use sysfs information to speed up things without cacheing.
    // TODO(Gris Ge): This function should return Result<>
    let sysfs_holder_dir = format!("/sys/dev/block/{}/holders", major_minor);
    let mut holders = match fs::read_dir(&sysfs_holder_dir) {
        Ok(o) => o,
        Err(e) => {
            println!(
                "mpath_parser: Failed to read_dir {}: {}",
                sysfs_holder_dir, e
            );
            return None;
        }
    };
    match holders.next() {
        Some(Ok(holder)) => {
            let dm = holder.path();
            let dm = match dm.to_str() {
                Some(p) => p,
                None => {
                    println!(
                        "mpath_parser: Path {:?} is not valid unicode",
                        holder
                    );
                    return None;
                }
            };
            let name_path = format!("{}/dm/name", dm);
            let uuid_path = format!("{}/dm/uuid", dm);
            let mut uuid = Sysfs::read(&uuid_path);
            if uuid.starts_with("mpath-") {
                uuid = uuid["mpath-".len()..].to_string();
                return Some((Sysfs::read(&name_path), uuid));
            }
        }
        Some(Err(e)) => println!(
            "mpath_parser: Failed to read_dir {}: {}",
            sysfs_holder_dir, e
        ),
        None => println!("mpath_parser: {} is empty", sysfs_holder_dir),
    };
    None
}

fn parse_event(event: &StorageEvent, sender: &Sender<StorageEvent>) {
    match event.event_type.as_ref() {
        "DM_MPATH_PATH_FAILED" | "DM_MPATH_PATH_REINSTATED" => {
            let (name, uuid) = match get_mpath_info_from_blk(&event.kdev) {
                Some(t) => t,
                None => return,
            };
            let mut event = event.clone();
            event.dev_path = format!("/dev/mapper/{}", name);
            event.dev_name = name;
            event.dev_wwid = uuid;
            if let Some(n) = Sysfs::major_minor_to_blk_name(&event.kdev) {
                event.extention.insert("blk_name".to_string(), n);
            }
            event
                .extention
                .insert("blk_major_minor".to_string(), event.kdev.clone());
            if let Err(e) = sender.send(event) {
                println!("mpath_parser: Failed to send event: {}", e);
            }
        }
        _ => println!("mpath: Got unknown event type: {}", event.event_type),
    };
}

pub fn parser_start(sender: Sender<StorageEvent>) -> ParserInfo {
    let (event_in_sender, event_in_recver) = mpsc::channel();
    let name = "mpath".to_string();
    let filter_event_type = vec![EventType::Raw];
    let filter_event_subsys = vec![StorageSubSystem::Multipath];

    if let Err(e) = Builder::new().name("mpath_parser".into()).spawn(
        move || loop {
            match event_in_recver.recv() {
                Ok(event) => parse_event(&event, &sender),
                Err(e) => {
                    println!("mpath_parser: Failed to retrieve event: {}", e)
                }
            };
        },
    ) {
        panic!("mpath_parser: Failed to create parser thread: {}", e);
    }

    println!("mpath_parser: Ready");
    ParserInfo {
        sender: event_in_sender,
        name: name,
        filter_event_type: filter_event_type,
        filter_event_subsys: Some(filter_event_subsys),
    }
}
