extern crate libc;
extern crate peripety;
extern crate regex;

use data::{BlkInfo, EventType, ParserInfo};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use peripety::{StorageEvent, StorageSubSystem};
use std::thread::spawn;
use std::fs;
use std::path::Path;
use std::ffi::CString;
use std::ffi::CStr;

fn uuid_of_blk(blk_path: &str) -> String {
    let blk_path = Path::new(&blk_path);
    if !blk_path.exists() {
        return String::new();
    }

    let blk_path = blk_path.canonicalize().unwrap();
    for entry in fs::read_dir("/dev/disk/by-uuid").unwrap() {
        let path = entry.unwrap().path();
        if let Ok(p) = fs::read_link(&path) {
            let link_path =
                format!("/dev/disk/by-uuid/{}", p.to_str().unwrap());
            let cur_path = Path::new(&link_path).canonicalize().unwrap();
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
        event.owners_wwids = blk_info.owners_wwids;
        event.owners_names = blk_info.owners_names;
        event.owners_paths = blk_info.owners_paths;
        event.owners_wwids.insert(0, blk_info.wwid);
        event.owners_names.insert(0, blk_info.name);
        let mnt_pnt = get_mount_point(&blk_info.blk_path);
        if mnt_pnt.len() != 0 {
            event.extention.insert("mount_point".to_string(), mnt_pnt);
        }
        event.owners_paths.insert(0, blk_info.blk_path);

        sender.send(event).unwrap();
    }
}

pub fn parser_start(sender: Sender<StorageEvent>) -> ParserInfo {
    let (event_in_sender, event_in_recver) = mpsc::channel();
    let name = "fs".to_string();
    let filter_event_type = vec![EventType::Raw];
    let filter_event_subsys =
        vec![StorageSubSystem::FsExt4, StorageSubSystem::FsXfs];

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

fn get_mount_point(blk_path: &str) -> String {
    let mut ret = String::new();
    let fd = unsafe {
        libc::setmntent(
            CStr::from_bytes_with_nul(b"/proc/mounts\0")
                .unwrap()
                .as_ptr(),
            CStr::from_bytes_with_nul(b"r\0").unwrap().as_ptr(),
        )
    };
    if fd.is_null() {
        return ret;
    }
    let mut entry = unsafe { libc::getmntent(fd) };
    while !entry.is_null() {
        let table: libc::mntent = unsafe { *entry };
        let mnt_fsname =
            unsafe { CStr::from_ptr(table.mnt_fsname).to_str().unwrap() };
        if mnt_fsname == blk_path {
            ret = unsafe {
                CString::from_raw(table.mnt_dir).into_string().unwrap()
            };
            break;
        }
        entry = unsafe { libc::getmntent(fd) };
    }
    unsafe { libc::endmntent(fd) };
    return ret;
}
