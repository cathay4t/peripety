use super::dm;
use super::error::PeripetyError;
use super::scsi;
use super::sysfs::Sysfs;

use libc;
use regex::Regex;
use serde_json;
use std::ffi::CStr;
use std::ffi::CString;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Clone, PartialEq, Debug, Serialize)]
pub enum BlkType {
    Scsi,
    Dm,
    DmMultipath,
    DmLvm,
    Partition,
}

impl fmt::Display for BlkType {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BlkType::Scsi => write!(fmt, "SCSI"),
            BlkType::Dm => write!(fmt, "Device Mapper"),
            BlkType::DmMultipath => write!(fmt, "Device Mapper Multipath"),
            BlkType::DmLvm => write!(fmt, "Device Mapper LVM"),
            BlkType::Partition => write!(fmt, "Partition"),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct BlkInfo {
    pub wwid: String,
    pub blk_type: BlkType,
    pub blk_path: String,
    pub owners_wwids: Vec<String>,
    pub owners_types: Vec<BlkType>,
    pub owners_paths: Vec<String>,
    pub uuid: Option<String>,
    pub mount_point: Option<String>,
}

impl BlkInfo {
    pub fn new(blk: &str) -> Result<BlkInfo, PeripetyError> {
        let mut bi = BlkInfo::_new(blk, false)?;
        if let Ok(uuid) = BlkInfo::uuid(&bi.blk_path) {
            // Only search mount table when block has uuid.
            bi.uuid = Some(uuid);
            bi.mount_point = BlkInfo::get_mount_point(&bi.blk_path);
        }
        Ok(bi)
    }

    pub fn new_skip_extra(blk: &str) -> Result<BlkInfo, PeripetyError> {
        BlkInfo::_new(blk, true)
    }

    fn _new(
        blk: &str,
        skip_holder_check: bool,
    ) -> Result<BlkInfo, PeripetyError> {
        // device symbolic link or full path.
        if blk.starts_with('/') {
            if Path::new(blk).exists() {
                if let Ok(p) = Path::new(blk).canonicalize() {
                    if let Some(s) = p.file_name() {
                        if let Some(s) = s.to_str() {
                            return BlkInfo::_new(s, skip_holder_check);
                        }
                    }
                }
                return Err(PeripetyError::NoSupport(format!(
                    "Block path '{}' is not supported yet",
                    blk
                )));
            } else {
                return Err(PeripetyError::BlockNoExists(format!(
                    "Block path '{}' does not exists",
                    blk
                )));
            }
        }

        // sda
        if blk.starts_with("sd") {
            // If certain disk is used device-mapper (like multipath or LVM),
            // return block information for that mpath instead
            if !skip_holder_check {
                if let Some(d) = dm::get_holder_dm_name(blk) {
                    return dm::blk_info_get_dm(&d);
                }
            }
            return scsi::blk_info_get_scsi(blk);
        }

        // scsi_id: 4:0:1:1
        if let Ok(reg) = Regex::new(r"^(?:[0-9]+:){3}[0-9]+$") {
            if reg.is_match(blk) {
                return BlkInfo::_new(
                    &scsi::scsi_id_to_blk_name(blk)?,
                    skip_holder_check,
                );
            }
        }

        // dm-0
        if blk.starts_with("dm-") {
            return dm::blk_info_get_dm(blk);
        }

        // major: minor
        if let Ok(reg) = Regex::new(r"^[0-9]+:[0-9]+$") {
            if reg.is_match(blk) {
                return BlkInfo::_new(
                    &BlkInfo::major_minor_to_blk_name(blk)?,
                    skip_holder_check,
                );
            }
        }

        // uuid
        let uuid_dev_path = format!("/dev/disk/by-uuid/{}", blk);
        if Path::new(&uuid_dev_path).exists() {
            return BlkInfo::_new(&uuid_dev_path, skip_holder_check);
        }

        // scsi wwid
        let sysfs_folder = "/sys/class/scsi_disk";
        if let Ok(entries) = fs::read_dir(&sysfs_folder) {
            for entry in entries {
                let e = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let mut p = e.path();
                p.push("device/wwid");
                if p.exists() {
                    if let Some(p) = p.to_str() {
                        let c = Sysfs::read(p)?;
                        if blk == scsi::pretty_wwid(&c) {
                            if let Some(s) = e.file_name().to_str() {
                                return BlkInfo::new(s);
                            }
                        }
                    }
                }
            }
        }

        Err(PeripetyError::NoSupport(format!(
            "Block path '{}' is not supported yet",
            blk
        )))
    }

    pub fn uuid(blk_path: &str) -> Result<String, PeripetyError> {
        let path = Path::new(&blk_path);
        if !path.exists() {
            return Err(PeripetyError::BlockNoExists(format!(
                "Block {} does not exists",
                blk_path
            )));
        }

        let blk_real_path = match path.canonicalize() {
            Ok(b) => b,
            Err(e) => {
                return Err(PeripetyError::InternalBug(format!(
                    "Failed to find canonicalize path of {}: {}",
                    blk_path, e
                )));
            }
        };
        let entries = match fs::read_dir("/dev/disk/by-uuid") {
            Ok(es) => es,
            Err(e) => {
                return Err(PeripetyError::InternalBug(format!(
                    "Failed to read_dir {}: {}",
                    "/dev/disk/by-uuid", e
                )));
            }
        };
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(p) = fs::read_link(&entry.path()) {
                    let link_path = Path::new("/dev/disk/by-uuid/").join(p);
                    if let Ok(cur_path) = link_path.canonicalize() {
                        if cur_path == blk_real_path {
                            if let Some(s) = entry.file_name().to_str() {
                                return Ok(s.to_string());
                            }
                        }
                    }
                }
            }
        }
        Err(PeripetyError::BlockNoExists(format!(
            "No block device is not holding any uuid {}",
            blk_path
        )))
    }

    pub fn to_json_string(&self) -> Result<String, PeripetyError> {
        match serde_json::to_string(&self) {
            Ok(s) => Ok(s),
            Err(e) => Err(PeripetyError::JsonSerializeError(format!(
                "{}",
                e
            ))),
        }
    }

    pub fn to_json_string_pretty(&self) -> Result<String, PeripetyError> {
        match serde_json::to_string_pretty(&self) {
            Ok(s) => Ok(s),
            Err(e) => Err(PeripetyError::JsonSerializeError(format!(
                "{}",
                e
            ))),
        }
    }

    pub fn get_mount_point(blk_path: &str) -> Option<String> {
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
            return None;
        }
        let mut entry = unsafe { libc::getmntent(fd) };
        while !entry.is_null() {
            let table: libc::mntent = unsafe { *entry };
            if let Ok(mnt_fsname) =
                unsafe { CStr::from_ptr(table.mnt_fsname).to_str() }
            {
                if mnt_fsname == blk_path {
                    if let Ok(s) = unsafe {
                        CString::from_raw(table.mnt_dir).into_string()
                    } {
                        ret = s;
                        break;
                    }
                }
                entry = unsafe { libc::getmntent(fd) };
            }
        }
        unsafe { libc::endmntent(fd) };
        if ret.is_empty() {
            return None;
        }
        Some(ret)
    }

    pub fn major_minor_to_blk_name(
        major_minor: &str,
    ) -> Result<String, PeripetyError> {
        let sysfs_path = format!("/sys/dev/block/{}", major_minor);
        match fs::read_link(&sysfs_path) {
            Ok(p) => {
                if let Some(p) = p.file_name() {
                    if let Some(s) = p.to_str() {
                        return Ok(s.to_string());
                    }
                }
            }
            Err(e) => {
                return Err(PeripetyError::InternalBug(format!(
                    "major_minor_to_blk_name(): \
                     Failed to read link {}: {}",
                    sysfs_path, e
                )));
            }
        };

        Err(PeripetyError::InternalBug(format!(
            "major_minor_to_blk_name():  Got non-utf8 path {}",
            sysfs_path
        )))
    }
}
