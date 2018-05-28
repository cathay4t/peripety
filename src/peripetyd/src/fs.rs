use data::{EventType, ParserInfo};
use peripety::{BlkInfo, StorageEvent, StorageSubSystem};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread::spawn;

fn parse_event(event: &StorageEvent, sender: &Sender<StorageEvent>) {
    let mut event = event.clone();
    match BlkInfo::new(&event.kdev) {
        Ok(blk_info) => {
            let uuid = match blk_info.uuid {
                Some(u) => u,
                None => {
                    println!(
                        "fs_parser: Failed to find uuid of block {}",
                        &event.kdev
                    );
                    return;
                }
            };
            event.msg = format!(
                "{}, uuid: '{}', blk_wwid: '{}', blk_path: '{}'",
                event.raw_msg,
                uuid,
                blk_info.wwid,
                blk_info.blk_path,
            );

            if let Some(mnt_pnt) = blk_info.mount_point {
                event.msg = format!(
                    "{}, mount_point: '{}'",
                    event.msg, mnt_pnt
                );
                event
                    .extension
                    .insert("mount_point".to_string(), mnt_pnt.clone());
            }
            event.dev_path = blk_info.blk_path.clone();
            event.owners_wwids = blk_info.owners_wwids;
            event.owners_paths = blk_info.owners_paths;
            event.owners_wwids.insert(0, blk_info.wwid);
            event.owners_paths.insert(0, blk_info.blk_path);
            event
                .extension
                .insert("uuid".to_string(), uuid.clone());
            event.dev_wwid = uuid;

            if let Err(e) = sender.send(event) {
                println!("fs_parser: Failed to send event: {}", e);
            }
        }
        Err(e) => println!("fs_parser: {}", e),
    }
}

pub fn parser_start(sender: Sender<StorageEvent>) -> ParserInfo {
    let (event_in_sender, event_in_recver) = mpsc::channel();
    let name = "fs".to_string();
    let filter_event_type = vec![EventType::Raw];
    let filter_event_subsys = vec![
        StorageSubSystem::FsExt4,
        StorageSubSystem::FsXfs,
    ];

    spawn(move || loop {
        match event_in_recver.recv() {
            Ok(event) => parse_event(&event, &sender),
            Err(e) => println!("fs_parser: Failed to receive event: {}", e),
        }
    });

    ParserInfo {
        sender: event_in_sender,
        name: name,
        filter_event_type: filter_event_type,
        filter_event_subsys: Some(filter_event_subsys),
    }
}
