extern crate peripety;

//mod data;

use data::{EventType, ParserInfo, Sysfs};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use peripety::{StorageEvent, StorageSubSystem};
use std::thread::spawn;
use std::fs;

fn get_mpath_info_from_blk(major_minor: &str) -> (String, String) {
    // We use sysfs information to speed up things without cacheing.
    // TODO(Gris Ge): This function should return Result<>
    let sysfs_holder_dir = format!("/sys/dev/block/{}/owners", major_minor);
    let mut owners = fs::read_dir(&sysfs_holder_dir).unwrap();
    if let Some(Ok(holder)) = owners.next() {
        let dm = holder.path();
        let name_path = format!("{}/dm/name", dm.to_str().unwrap());
        let uuid_path = format!("{}/dm/uuid", dm.to_str().unwrap());
        let mut uuid = Sysfs::read(&uuid_path);
        if uuid.starts_with("mpath-") {
            uuid = uuid["mpath-".len()..].to_string();
            return (Sysfs::read(&name_path), uuid);
        }
    }

    (String::new(), String::new())
}

fn parse_event(event: &StorageEvent, sender: &Sender<StorageEvent>) {
    match event.event_type.as_ref() {
        "DM_MPATH_PATH_FAILED" | "DM_MPATH_PATH_REINSTATED" => {
            let (name, uuid) = get_mpath_info_from_blk(&event.kdev);
            if name.len() == 0 {
                return;
            }
            let mut event = event.clone();
            event.dev_path = format!("/dev/mapper/{}", name);
            event.dev_name = name;
            event.dev_wwid = uuid;
            event.extention.insert(
                "blk_name".to_string(),
                Sysfs::major_minor_to_blk_name(&event.kdev),
            );
            event
                .extention
                .insert("blk_major_minor".to_string(), event.kdev.clone());
            sender.send(event).unwrap();
        }
        _ => println!("mpath: Got unknown event type: {}", event.event_type),
    };
}

pub fn parser_start(sender: Sender<StorageEvent>) -> ParserInfo {
    let (event_in_sender, event_in_recver) = mpsc::channel();
    let name = "mpath".to_string();
    let filter_event_type = vec![EventType::Raw];
    let filter_event_subsys = vec![StorageSubSystem::Multipath];

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
