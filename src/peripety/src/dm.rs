use super::blk_info::{BlkInfo, BlkType};
use super::error::PeripetyError;
use super::sysfs::Sysfs;

use std::ffi::OsStr;
use std::fs;
use std::path::Path;

pub(crate) fn get_holder_dm_name(blk: &str) -> Option<String> {
    let holders = format!("/sys/block/{}/holders", blk);
    if let Ok(mut entries) = fs::read_dir(&holders) {
        if let Some(Ok(holder)) = entries.next() {
            if let Some(n) = holder.path().file_name() {
                if let Some(s) = n.to_str() {
                    if s.starts_with("dm-") {
                        return Some(s.to_string());
                    }
                }
            }
        }
    }
    None
}

// Support query on these formats:
//  * dm-0
pub(crate) fn blk_info_get_dm(blk: &str) -> Result<BlkInfo, PeripetyError> {
    let sysfs_uuid = format!("/sys/block/{}/dm/uuid", &blk);

    if Path::new(&sysfs_uuid).exists() {
        let sysfs_name = format!("/sys/block/{}/dm/name", &blk);
        let name = Sysfs::read(&sysfs_name)?;
        let mut ret = BlkInfo {
            wwid: Sysfs::read(&sysfs_uuid)?,
            blk_type: BlkType::Dm,
            preferred_blk_path: format!("/dev/mapper/{}", &name),
            blk_path: format!("/dev/mapper/{}", &name),
            owners_wwids: Vec::new(),
            owners_types: Vec::new(),
            owners_paths: Vec::new(),
            uuid: None,
            mount_point: None,
        };
        if ret.wwid.starts_with("LVM-") {
            ret.blk_type = BlkType::DmLvm;
        } else if ret.wwid.starts_with("mpath-") {
            ret.blk_type = BlkType::DmMultipath;
        } else if ret.wwid.starts_with("part") {
            ret.blk_type = BlkType::Partition;
        }
        let slave_dir = format!("/sys/block/{}/slaves", &blk);
        let entries = match fs::read_dir(&slave_dir) {
            Ok(e) => e,
            Err(e) => {
                return Err(PeripetyError::InternalBug(format!(
                    "dm::blk_info_get_dm(): Failed to read_dir {}: {}",
                    slave_dir, e
                )));
            }
        };
        for entry in entries {
            let f = match entry {
                Ok(e) => e.file_name(),
                Err(_) => continue,
            };
            let slave_blk = match f.to_str() {
                Some(k) => k,
                None => continue,
            };
            if let Ok(slave_info) = BlkInfo::new_skip_extra(slave_blk) {
                if !ret.owners_wwids.contains(&slave_info.wwid) {
                    ret.owners_wwids.push(slave_info.wwid.clone());
                    ret.owners_types
                        .push(slave_info.blk_type.clone());
                }
                if !ret.owners_paths.contains(&slave_info.blk_path) {
                    ret.owners_paths
                        .push(slave_info.blk_path.clone());
                }
                if slave_info.blk_type == BlkType::DmLvm
                    || slave_info.blk_type == BlkType::Dm
                    || slave_info.blk_type == BlkType::DmMultipath
                {
                    for sub_slave_blk_path in slave_info.owners_paths {
                        let sub_slave_blk;
                        if let Ok(p) =
                            Path::new(&sub_slave_blk_path).canonicalize()
                        {
                            sub_slave_blk =
                                match p.file_name().and_then(OsStr::to_str) {
                                    Some(n) => n.to_string(),
                                    None => continue,
                                };
                        } else {
                            continue;
                        }

                        if let Ok(sub_slave_info) =
                            BlkInfo::new_skip_extra(&sub_slave_blk)
                        {
                            if !ret.owners_wwids.contains(&sub_slave_info.wwid)
                            {
                                ret.owners_wwids.push(sub_slave_info.wwid);
                                ret.owners_types.push(sub_slave_info.blk_type);
                            }
                            if !ret.owners_paths
                                .contains(&sub_slave_info.blk_path)
                            {
                                ret.owners_paths.push(sub_slave_info.blk_path);
                            }
                        }
                    }
                }
            }
        }
        if ret.owners_wwids.is_empty() {
            return Err(PeripetyError::InternalBug(format!(
                "dm::blk_info_get_dm() not supported blk {}",
                blk
            )));
        }
        return Ok(ret);
    }

    Err(PeripetyError::InternalBug(format!(
        "dm::blk_info_get_dm() \
         not supported blk {} as path {} not exists",
        blk, sysfs_uuid
    )))
}
