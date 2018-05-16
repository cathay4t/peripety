extern crate libc;
extern crate peripety;
extern crate regex;

use data::{BlkInfo, EventType, ParserInfo};
use peripety::{StorageEvent, StorageSubSystem};
use std::ffi::CStr;
use std::ffi::CString;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread::spawn;

fn uuid_of_blk(blk_path: &str) -> String {
    let path = Path::new(&blk_path);
    if !path.exists() {
        println!("fs_parser: Path {} does not exists", blk_path);
        return String::new();
    }

    let blk_path = match path.canonicalize() {
        Ok(b) => b,
        Err(e) => {
            println!(
                "fs_parser: Failed to find canonicalize path of {}: {}",
                blk_path, e
            );
            return String::new();
        }
    };
    let entries = match fs::read_dir("/dev/disk/by-uuid") {
        Ok(es) => es,
        Err(e) => {
            println!(
                "fs_parser: Failed to read_dir {}: {}",
                "/dev/disk/by-uuid", e
            );
            return String::new();
        }
    };
    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(p) = fs::read_link(&entry.path()) {
                let link_path = Path::new("/dev/disk/by-uuid/").join(p);
                if let Ok(cur_path) = link_path.canonicalize() {
                    if cur_path == blk_path {
                        if let Some(s) = entry.file_name().to_str() {
                            return s.to_string();
                        }
                    }
                }
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
        event.dev_path = blk_info.blk_path.clone();
        event.owners_wwids = blk_info.owners_wwids;
        event.owners_paths = blk_info.owners_paths;
        event.owners_wwids.insert(0, blk_info.wwid);
        let mnt_pnt = get_mount_point(&blk_info.blk_path);
        if mnt_pnt.len() != 0 {
            event
                .extention
                .insert("mount_point".to_string(), mnt_pnt);
        }
        event.owners_paths.insert(0, blk_info.blk_path);

        if let Err(e) = sender.send(event) {
            println!("fs_parser: Failed to send event: {}", e);
        }
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

fn get_mount_point(blk_path: &str) -> String {
    let mut ret = String::new();
    let fd = unsafe {
        libc::setmntent(
            CStr::from_bytes_with_nul(b"/proc/mounts\0")
                .expect("BUG: get_mount_point()")
                // ^We never panic as it is null terminated.
                .as_ptr(),
            CStr::from_bytes_with_nul(b"r\0")
                .expect("BUG")
                .as_ptr(),
            // ^We never panic as it is null terminated.
        )
    };
    if fd.is_null() {
        return ret;
    }
    let mut entry = unsafe { libc::getmntent(fd) };
    while !entry.is_null() {
        let table: libc::mntent = unsafe { *entry };
        if let Ok(mnt_fsname) =
            unsafe { CStr::from_ptr(table.mnt_fsname).to_str() }
        {
            if mnt_fsname == blk_path {
                if let Ok(s) =
                    unsafe { CString::from_raw(table.mnt_dir).into_string() }
                {
                    ret = s;
                    break;
                }
            }
            entry = unsafe { libc::getmntent(fd) };
        }
    }
    unsafe { libc::endmntent(fd) };
    return ret;
}
