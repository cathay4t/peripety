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
            let blk_info = blk_info_get_scsi(&name)?;
            return Ok(BlkInfo {
                wwid: format!("{}-part{}", blk_info.wwid, part),
                blk_type: BlkType::Partition,
                blk_path: format!("/dev/{}", &blk),
                owners_wwids: vec![blk_info.wwid],
                owners_types: vec![BlkType::Scsi],
                owners_paths: vec![blk_info.blk_path],
                uuid: None,
                mount_point: None,
            });
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
        return Ok(BlkInfo {
            wwid: pretty_wwid(&Sysfs::read(&sysfs_path)?),
            blk_type: BlkType::Scsi,
            blk_path: format!("/dev/{}", &name),
            owners_wwids: Vec::new(),
            owners_types: Vec::new(),
            owners_paths: Vec::new(),
            uuid: None,
            mount_point: None,
        });
    }

    return Err(PeripetyError::InternalBug(format!(
        "scsi::blk_info_get_scsi(): Got invalid scsi blk {}",
        blk
    )));
}
