extern crate peripety;
extern crate regex;

use dm;
use peripety::{StorageEvent, StorageSubSystem};
use regex::Regex;
use scsi;
use std::fs;
use std::io::Read;
use std::sync::mpsc::Sender;

#[derive(PartialEq, Clone, Debug)]
pub enum EventType {
    Raw,
    Synthetic,
}

#[derive(Clone, Debug)]
pub struct ParserInfo {
    pub sender: Sender<StorageEvent>,
    pub name: String,
    pub filter_event_type: Vec<EventType>,
    pub filter_event_subsys: Option<Vec<StorageSubSystem>>,
}

#[derive(Clone, Debug)]
pub struct RegexConf {
    pub starts_with: Option<String>,
    pub regex: Regex,
    pub sub_system: StorageSubSystem,
    pub event_type: String,
}

#[derive(Clone, Debug)]
pub struct RegexConfStr<'a> {
    pub starts_with: Option<&'a str>,
    pub regex: &'a str,
    pub sub_system: &'a str,
    pub event_type: &'a str,
}

impl<'a> RegexConfStr<'a> {
    pub fn to_regex_conf(&self) -> RegexConf {
        RegexConf {
            starts_with: self.starts_with.map(|s| s.to_string()),
            regex: Regex::new(self.regex).expect(&format!(
                "BUG: data.rs has invalid regex: {}",
                self.regex
            )),
            // ^ We panic when hard-coded regex is not valid. It's developer's
            // fault.
            sub_system: self.sub_system
                .parse()
                .expect("BUG: data.rs has invalid sub_system"),
            // ^ We panic when hard-coded sub_system is not valid. It's
            // developer's fault.
            event_type: self.event_type.to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum BlkType {
    Scsi,
    Dm,
    DmMultipath,
    DmLvm,
    Partition,
}

#[derive(Debug)]
pub struct BlkInfo {
    pub wwid: String,
    pub blk_type: BlkType,
    pub name: String,
    pub blk_path: String,
    pub owners_wwids: Vec<String>,
    pub owners_types: Vec<BlkType>,
    pub owners_names: Vec<String>,
    pub owners_paths: Vec<String>,
}

impl BlkInfo {
    pub fn new(kdev: &str) -> Option<BlkInfo> {
        // dm-0
        if kdev.starts_with("dm-") {
            return dm::blk_info_get_dm(kdev);
        }

        // sdb
        if kdev.starts_with("sd") {
            return scsi::blk_info_get_scsi(kdev);
        }

        // scsi_id: 4:0:1:1
        if let Ok(reg) = Regex::new(r"^(:?[0-9]+:){3}[0-9]+$") {
            if reg.is_match(kdev) {
                return scsi::blk_info_get_scsi(kdev);
            }
        }

        // major: minor
        if let Ok(reg) = Regex::new(r"^[0-9]+:[0-9]+$") {
            if reg.is_match(kdev) {
                if let Some(n) = Sysfs::major_minor_to_blk_name(kdev) {
                    return BlkInfo::new(&n);
                }
            }
        }
        None
    }
}

pub const BUILD_IN_REGEX_CONFS: &[RegexConfStr] = &[
    RegexConfStr {
        starts_with: Some("device-mapper: multipath:"),
        regex: r"(?x)
                ^device-mapper:\s
                multipath:\ Failing\ path\s
                (?P<kdev>\d+:\d+).$
                ",
        sub_system: "multipath",
        event_type: "DM_MPATH_PATH_FAILED",
    },
    RegexConfStr {
        starts_with: Some("device-mapper: multipath:"),
        regex: r"(?x)
                ^device-mapper:\s
                multipath:\ Reinstating\ path\s
                (?P<kdev>\d+:\d+).$
                ",
        sub_system: "multipath",
        event_type: "DM_MPATH_PATH_REINSTATED",
    },
    RegexConfStr {
        starts_with: Some("EXT4-fs "),
        regex: r"(?x)
                ^EXT4-fs\s
                \((?P<kdev>[^\s\)]+)\):\s
                mounted\ filesystem\s
                ",
        sub_system: "ext4",
        event_type: "DM_FS_MOUNTED",
    },
    RegexConfStr {
        starts_with: Some("XFS "),
        regex: r"(?x)
                ^XFS \s
                \((?P<kdev>[^\s\)]+)\):\s
                Ending\ clean\ mount",
        sub_system: "xfs",
        event_type: "DM_FS_MOUNTED",
    },
    RegexConfStr {
        starts_with: Some("XFS "),
        regex: r"(?x)
                ^XFS\s
                \((?P<kdev>[^\s\)]+)\):\s
                Unmounting\ Filesystem$",
        sub_system: "xfs",
        event_type: "DM_FS_UNMOUNTED",
    },
    RegexConfStr {
        starts_with: Some("XFS "),
        regex: r"(?x)
                ^XFS \s
                \((?P<kdev>[^\s\)]+)\):\s
                writeback\ error\ on\ sector",
        sub_system: "xfs",
        event_type: "DM_FS_IO_ERROR",
    },
    RegexConfStr {
        starts_with: Some("EXT4-fs "),
        regex: r"(?x)
                ^EXT4-fs\s
                warning\ \(device\s
                (?P<kdev>[^\s\)]+)\):\s
                ext4_end_bio:[0-9]+:\ I/O\ error
                ",
        sub_system: "ext4",
        event_type: "DM_FS_IO_ERROR",
    },
    RegexConfStr {
        starts_with: Some("JBD2: "),
        regex: r"(?x)
                ^JBD2:\s
                Detected\ IO\ errors\ while\ flushing\ file\ data\ on\s
                (?P<kdev>[^\s]+)-[0-9]+$
                ",
        sub_system: "ext4",
        event_type: "DM_FS_IO_ERROR",
    },
];

pub struct Sysfs;

impl Sysfs {
    pub fn major_minor_to_blk_name(major_minor: &str) -> Option<String> {
        let sysfs_path = format!("/sys/dev/block/{}", major_minor);
        match fs::read_link(&sysfs_path) {
            // We don't do unicode check there as this are used internally
            // where no such non-utf8 concern.
            Ok(p) => {
                if let Some(p) = p.file_name() {
                    if let Some(s) = p.to_str() {
                        return Some(s.to_string());
                    }
                }
            }
            Err(e) => {
                println!(
                    "Sysfs::major_minor_to_blk_name(): \
                     Failed to read link {}: {}",
                    sysfs_path, e
                );
                return None;
            }
        };
        None
    }

    pub fn scsi_id_to_blk_name(scsi_id: &str) -> String {
        let sysfs_path =
            format!("/sys/class/scsi_disk/{}/device/block", scsi_id);
        let mut blks = match fs::read_dir(&sysfs_path) {
            Ok(b) => b,
            Err(e) => {
                println!(
                    "Sysfs::scsi_id_to_blk_name(): Failed to read_dir {}: {}",
                    sysfs_path, e
                );
                return String::new();
            }
        };
        if let Some(Ok(blk)) = blks.next() {
            // Assuming sysfs are all utf8 filenames for block layer.
            if let Some(n) = blk.path().file_name() {
                if let Some(s) = n.to_str() {
                    return s.to_string();
                }
            }
        }

        String::new()
    }

    pub fn scsi_id_of_disk(name: &str) -> Option<String> {
        let sysfs_path = format!("/sys/block/{}/device", name);
        match fs::read_link(&sysfs_path) {
            Ok(p) => {
                if let Some(p) = p.file_name() {
                    if let Some(s) = p.to_str() {
                        return Some(s.to_string());
                    }
                }
            }
            Err(e) => {
                println!(
                    "Sysfs::scsi_host_id_of_disk(): Failed to read link {}: {}",
                    sysfs_path, e
                );
                return None;
            }
        };
        None
    }

    pub fn scsi_host_id_of_scsi_id(scsi_id: &str) -> Option<String> {
        if let Some(index) = scsi_id.find(":") {
            return Some(scsi_id[..index].to_string());
        }

        None
    }

    pub fn read(path: &str) -> String {
        let mut contents = String::new();
        match fs::File::open(path) {
            Ok(mut fd) => {
                if let Err(e) = fd.read_to_string(&mut contents) {
                    println!(
                        "Sysfs::read(): Failed to read file {}: {}",
                        path, e
                    );
                }
                if contents.ends_with("\n") {
                    contents.pop();
                }
            }
            Err(e) => {
                println!(
                    "Sysfs::read(): Failed to read file {}: {}",
                    path, e
                );
            }
        };
        contents
    }
}
