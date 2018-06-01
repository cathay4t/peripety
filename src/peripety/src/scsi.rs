use super::blk_info::{BlkInfo, BlkType};
use super::error::PeripetyError;
use super::sysfs::Sysfs;

use regex::Regex;
use std::fs;
use std::path::Path;

pub(crate) fn scsi_id_to_blk_name(
    scsi_id: &str,
) -> Result<String, PeripetyError> {
    let sysfs_path = format!("/sys/class/scsi_disk/{}/device/block", scsi_id);
    if !Path::new(&sysfs_path).exists() {
        return Err(PeripetyError::BlockNoExists(format!(
            "SCSI ID {} does not exists",
            scsi_id
        )));
    }
    let mut blks = match fs::read_dir(&sysfs_path) {
        Ok(b) => b,
        Err(e) => {
            return Err(PeripetyError::InternalBug(format!(
                "scsi_id_to_blk_name(): Failed to read_dir {}: {}",
                sysfs_path, e
            )))
        }
    };

    if let Some(Ok(blk)) = blks.next() {
        if let Some(n) = blk.path().file_name() {
            if let Some(s) = n.to_str() {
                return Ok(s.to_string());
            }
        }
    }

    Err(PeripetyError::InternalBug(format!(
        "scsi::scsi_id_to_blk_name(): Folder {} is empty",
        sysfs_path,
    )))
}

pub(crate) fn pretty_wwid(wwid: &str) -> String {
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

// Support query on these formats:
//  * 4:0:0:1
//  * sda
//  * sda1
pub(crate) fn blk_info_get_scsi(blk: &str) -> Result<BlkInfo, PeripetyError> {
    let name;

    // Check if partition
    if let Ok(reg) = Regex::new("^(sd[a-z]+)([0-9]+)$") {
        if let Some(cap) = reg.captures(blk) {
            let name = cap.get(1)
                .expect("BUG: blk_info_get_scsi()")
                // ^ We never panic as above regex is valid.
                .as_str();
            let part = cap.get(2)
                .expect("BUG: blk_info_get_scsi()")
                // ^ We never panic as above regex is valid.
                .as_str();
            let blk_info = BlkInfo::new(&name)?;
            let mut blk_path = format!("/dev/{}", &blk);
            let uuid = match BlkInfo::uuid(&blk_path) {
                Ok(u) => Some(u),
                Err(_) => None,
            };
            // udev create mpatha-part1 while kpartx create mpatha1
            if blk_info.blk_type == BlkType::DmMultipath {
                blk_path = format!("{}-part{}", blk_info.blk_path, part);
                if !Path::new(&blk_path).exists() {
                    blk_path = format!("{}{}", blk_info.blk_path, part);
                    if !Path::new(&blk_path).exists() {
                        return Err(PeripetyError::BlockNoExists(format!(
                            "Multipath partition file of {} partition {} \
                             is missing, please use kpartx to create them",
                            blk_info.blk_path, part
                        )));
                    }
                }
            }

            let preferred_blk_path = if let Some(u) = &uuid {
                format!("/dev/disk/by-uuid/{}", u)
            } else {
                get_prefered_blk_path(&blk_path)
            };
            let mut ret = BlkInfo {
                wwid: format!("{}-part{}", blk_info.wwid, part),
                blk_type: BlkType::Partition,
                blk_path,
                preferred_blk_path,
                uuid,
                owners_wwids: blk_info.owners_wwids,
                owners_types: blk_info.owners_types,
                owners_paths: blk_info.owners_paths,
                mount_point: None,
            };
            ret.owners_wwids.insert(0, blk_info.wwid);
            ret.owners_types.insert(0, blk_info.blk_type);
            ret.owners_paths.insert(0, blk_info.blk_path);
            return Ok(ret);
        }
    }

    // Try 4:0:0:1 format
    let mut sysfs_path = format!("/sys/class/scsi_disk/{}/device/wwid", &blk);
    if Path::new(&sysfs_path).exists() {
        name = scsi_id_to_blk_name(blk)?;
    } else {
        // Try sda format
        sysfs_path = format!("/sys/block/{}/device/wwid", &blk);
        name = blk.to_string();
    }

    if Path::new(&sysfs_path).exists() {
        let blk_path = format!("/dev/{}", &name);
        return Ok(BlkInfo {
            wwid: pretty_wwid(&Sysfs::read(&sysfs_path)?),
            blk_type: BlkType::Scsi,
            preferred_blk_path: get_prefered_blk_path(&blk_path),
            blk_path,
            owners_wwids: Vec::new(),
            owners_types: Vec::new(),
            owners_paths: Vec::new(),
            uuid: None,
            mount_point: None,
        });
    }

    Err(PeripetyError::InternalBug(format!(
        "scsi::blk_info_get_scsi(): Got invalid scsi blk {}",
        blk
    )))
}

fn get_prefered_blk_path(raw_blk_path: &str) -> String {
    let dev_folder = "/dev/disk/by-id";
    let raw_path = Path::new(raw_blk_path);
    let mut matches = Vec::new();
    if let Ok(entries) = fs::read_dir(dev_folder) {
        for entry in entries {
            let e = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if let Some(s) = e.file_name().to_str() {
                if let Ok(p) = Path::new(dev_folder).join(s).canonicalize() {
                    if p == raw_path {
                        matches.push(s.to_string())
                    }
                }
            }
        }
    }
    for s in &matches {
        if s.starts_with("wwn-") {
            return format!("{}/{}", dev_folder, s);
        }
    }

    match matches.get(0) {
        Some(s) => format!("{}/{}", dev_folder, s),
        None => raw_blk_path.to_string(),
    }
}
