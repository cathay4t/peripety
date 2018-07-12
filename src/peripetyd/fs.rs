use data::{EventType, ParserInfo};
use peripety::{BlkInfo, StorageEvent, StorageSubSystem};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread::spawn;

fn parse_event(event: &StorageEvent, sender: &Sender<StorageEvent>) {
    let mut event = event.clone();
    match BlkInfo::new(&event.kdev) {
        Ok(blk_info) => {
            let uuid = match &blk_info.uuid {
                Some(u) => u.clone(),
                None => {
                    println!(
                        "fs_parser: Failed to find uuid of block {}",
                        &event.kdev
                    );
                    return;
                }
            };
            event.msg = format!(
                "{}, blk_wwid: '{}', blk_path: '{}'",
                event.raw_msg, blk_info.wwid, blk_info.blk_path,
            );

            if let Some(ref mnt_pnt) = blk_info.mount_point {
                event
                    .extension
                    .insert("mount_point".to_string(), mnt_pnt.clone());
            }
            event.blk_info = blk_info;
            event.extension.insert("uuid".to_string(), uuid.clone());

            if event.sub_system == StorageSubSystem::FsExt4
                && event.event_type == "FS_MOUNTED"
            {
                let data_mode = match event
                    .extension
                    .get("data_mode")
                    .expect("BUG: Invalid build_regex for ext4 FS_MOUNTED.")
                    .as_str()
                {
                    " journalled data mode" => "journalled".to_string(),
                    " ordered data mode" => "ordered".to_string(),
                    " writeback data mode" => "writeback".to_string(),
                    "out journal" => "no_journal".to_string(),
                    _ => "unknown".to_string(),
                };
                event.extension.insert("data_mode".to_string(), data_mode);
            }
            for (key, value) in &event.extension {
                event.msg = format!("{}, {}: '{}'", event.msg, key, value);
            }

            if let Err(e) = sender.send(event) {
                println!("fs_parser: Failed to send event: {}", e);
            }
        }
        Err(e) => println!("fs_parser: {}", e),
    }
}

pub fn parser_start(sender: Sender<StorageEvent>) -> ParserInfo {
    let (event_in_sender, event_in_recver) = mpsc::channel();

    spawn(move || loop {
        match event_in_recver.recv() {
            Ok(event) => parse_event(&event, &sender),
            Err(e) => println!("fs_parser: Failed to receive event: {}", e),
        }
    });

    ParserInfo {
        sender: event_in_sender,
        name: "fs".to_string(),
        filter_event_type: vec![EventType::Raw],
        filter_event_subsys: Some(vec![
            StorageSubSystem::FsExt4,
            StorageSubSystem::FsXfs,
            StorageSubSystem::FsJbd2,
        ]),
    }
}
