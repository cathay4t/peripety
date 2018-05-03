extern crate peripety;
extern crate regex;

use scsi;
use dm;
use regex::Regex;
use std::sync::mpsc::Sender;
use peripety::{StorageEvent, StorageSubSystem};
use std::fs;
use std::io::Read;
use std::ffi::OsStr;

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
pub struct RegexConf<'a> {
    pub starts_with: &'a str,
    pub regex: Regex,
    pub sub_system: StorageSubSystem,
    pub event_type: &'a str,
}

#[derive(Clone, Debug)]
pub struct RegexConfStr<'a> {
    pub starts_with: &'a str,
    pub regex: &'a str,
    pub sub_system: &'a str,
    pub event_type: &'a str,
}

impl<'a> RegexConfStr<'a> {
    pub fn to_regex_conf(&self) -> RegexConf {
        RegexConf {
            starts_with: self.starts_with,
            regex: Regex::new(self.regex).unwrap(),
            sub_system: self.sub_system.parse().unwrap(),
            event_type: self.event_type,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum BlkType {
    Scsi,
    Nvme,
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
        if kdev.starts_with("dm") {
            return dm::blk_info_get_dm(kdev);
        }

        if kdev.starts_with("sd") {
            return scsi::blk_info_get_scsi(kdev);
        }

        if let Ok(reg) = Regex::new(r"^(:?[0-9]+:){3}[0-9]+$") {
            if reg.is_match(kdev) {
                return scsi::blk_info_get_scsi(kdev);
            }
        }
        None
    }
}

pub const BUILD_IN_REGEX_CONFS: &[RegexConfStr] = &[
    RegexConfStr {
        starts_with: "device-mapper: multipath:",
        regex: r"(?x)
                ^device-mapper:\s
                multipath:\ Failing\ path\s
                (?P<kdev>\d+:\d+).$
                ",
        sub_system: "multipath",
        event_type: "DM_MPATH_PATH_FAILED",
    },
    RegexConfStr {
        starts_with: "device-mapper: multipath:",
        regex: r"(?x)
                ^device-mapper:\s
                multipath:\ Reinstating\ path\s
                (?P<kdev>\d+:\d+).$
                ",
        sub_system: "multipath",
        event_type: "DM_MPATH_PATH_REINSTATED",
    },
    RegexConfStr {
        starts_with: "EXT4-fs ",
        regex: r"(?x)
                ^EXT4-fs\s
                \((?P<kdev>[^\s\)]+)\):\s
                mounted\ filesystem\s
                ",
        sub_system: "ext4",
        event_type: "DM_FS_MOUNTED",
    },
    RegexConfStr {
        starts_with: "XFS ",
        regex: r"(?x)
                ^XFS \s
                \((?P<kdev>[^\s\)]+)\):\s
                Ending\ clean\ mount",
        sub_system: "xfs",
        event_type: "DM_FS_MOUNTED",
    },
    RegexConfStr {
        starts_with: "EXT4-fs ",
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
        starts_with: "JBD2: ",
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
    pub fn major_minor_to_blk_name(major_minor: &str) -> String {
        let sysfs_path = format!("/sys/dev/block/{}", major_minor);
        if let Ok(p) = fs::read_link(sysfs_path) {
            return p.file_name()
                .map(|p| p.to_str().unwrap())
                .unwrap()
                .to_string();
        } else {
            panic!("Sysfs::major_minor_to_blk_name(): Failed to read link");
        }
    }

    pub fn scsi_id_to_blk_name(scsi_id: &str) -> String {
        let sysfs_path =
            format!("/sys/class/scsi_disk/{}/device/block", scsi_id);
        let mut blks = fs::read_dir(&sysfs_path).unwrap();
        if let Some(Ok(blk)) = blks.next() {
            return blk.path()
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap()
                .to_string();
        }

        String::new()
    }

    pub fn read(path: &str) -> String {
        let mut fd = fs::File::open(path).unwrap();
        let mut contents = String::new();
        fd.read_to_string(&mut contents).unwrap();
        if contents.ends_with("\n") {
            contents.pop();
        }
        contents
    }
}
