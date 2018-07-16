use super::dm;
use super::error::PeripetyError;
use super::scsi;
use super::sysfs::Sysfs;

use libmount::mountinfo;
use regex::Regex;
use serde_json;
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum BlkType {
    Unknown,
    Other,
    Scsi,
    Dm,
    DmMultipath,
    DmLvm,
    Partition,
}

impl fmt::Display for BlkType {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BlkType::Unknown => write!(fmt, "Unknown"),
            BlkType::Other => write!(fmt, "Other"),
            BlkType::Scsi => write!(fmt, "SCSI"),
            BlkType::Dm => write!(fmt, "Device Mapper"),
            BlkType::DmMultipath => write!(fmt, "Device Mapper Multipath"),
            BlkType::DmLvm => write!(fmt, "Device Mapper LVM"),
            BlkType::Partition => write!(fmt, "Partition"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlkInfo {
    pub wwid: String,
    pub blk_type: BlkType,
    pub blk_path: String,
    pub preferred_blk_path: String, // preferred block path
    pub uuid: Option<String>,
    pub mount_point: Option<String>,
    pub transport_id: String,
    pub owners: Vec<BlkInfo>,
}

impl Default for BlkInfo {
    fn default() -> BlkInfo {
        BlkInfo {
            wwid: String::new(),
            blk_type: BlkType::Unknown,
            blk_path: String::new(),
            preferred_blk_path: String::new(),
            uuid: None,
            mount_point: None,
            transport_id: String::new(),
            owners: Vec::new(),
        }
    }
}

fn flat_blk_info_owners(owners: Vec<BlkInfo>) -> Vec<BlkInfo> {
    let mut ret = Vec::new();
    for owner in owners {
        ret.push(owner.clone());
        ret.append(&mut flat_blk_info_owners(owner.owners));
    }
    ret
}

impl BlkInfo {
    pub fn new(blk: &str) -> Result<BlkInfo, PeripetyError> {
        let mut bi = BlkInfo::_new(blk, false)?;
        if bi.uuid.is_none() {
            if let Ok(uuid) = BlkInfo::uuid(&bi.blk_path) {
                // Only search mount table when block has uuid.
                bi.uuid = Some(uuid);
            }
        }
        if bi.uuid.is_some() {
            bi.mount_point = BlkInfo::get_mount_point(&bi.blk_path);
        }

        // flat the owners array.
        bi.owners = flat_blk_info_owners(bi.owners);

        Ok(bi)
    }

    pub fn list() -> Result<Vec<BlkInfo>, PeripetyError> {
        // Steps:
        //  1. Enumerate /sys/class/block/ folder.
        //  2. Query dm-[0-9]+
        //  3. Query sd[a-z]+, if already included by above, skip.
        //  4. Query nvme[0-9]+n[0-9]+. TODO
        // Try dm first as multipath might contain many slaves.
        let dir_entries = match fs::read_dir("/sys/class/block") {
            Ok(d) => d,
            Err(e) => {
                return Err(PeripetyError::InternalBug(format!(
                    "Failed to read dir /sys/class/block: {}",
                    e
                )))
            }
        };
        let mut dm_devs = Vec::new();
        let mut other_devs = Vec::new();

        for entry in dir_entries {
            if let Ok(e) = entry {
                if let Ok(name) = e.file_name().into_string() {
                    if name.starts_with("dm-") {
                        dm_devs.push(name);
                    } else {
                        other_devs.push(name);
                    }
                }
            }
        }

        let mut ret = Vec::new();

        for dm_dev in &dm_devs {
            let info = BlkInfo::new(&dm_dev)?;
            if info.blk_type == BlkType::DmMultipath {
                for owner_info in info.owners {
                    let blk_name = match owner_info.blk_path.rfind('/') {
                        Some(i) => &owner_info.blk_path[i + 1..],
                        None => owner_info.blk_path.as_str(),
                    };
                    other_devs.retain(|x| x != blk_name);
                }
            }
            ret.push(BlkInfo::new(&dm_dev)?);
        }

        for other_dev in &other_devs {
            ret.push(BlkInfo::new(&other_dev)?);
        }

        Ok(ret)
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
            Err(e) => Err(PeripetyError::JsonSerializeError(format!("{}", e))),
        }
    }

    pub fn to_json_string_pretty(&self) -> Result<String, PeripetyError> {
        match serde_json::to_string_pretty(&self) {
            Ok(s) => Ok(s),
            Err(e) => Err(PeripetyError::JsonSerializeError(format!("{}", e))),
        }
    }

    pub fn get_mount_point(blk_path: &str) -> Option<String> {
        let mut fd = fs::File::open("/proc/self/mountinfo").unwrap();
        let mut data = Vec::new();
        fd.read_to_end(&mut data).unwrap();

        for e in mountinfo::Parser::new(&data) {
            if let Ok(m) = e {
                // TODO(Gris Ge): we should use read_link() to compare blk_path
                // and mount_source.
                if let Some(mount_source) = m.mount_source.into_owned().to_str()
                {
                    if let Some(mount_point) =
                        m.mount_point.into_owned().to_str()
                    {
                        if mount_source == blk_path {
                            return Some(mount_point.to_string());
                        }
                    }
                }
            }
        }
        None
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
