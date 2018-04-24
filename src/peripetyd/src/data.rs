extern crate peripety;
extern crate regex;

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

pub const BUILD_IN_REGEX_CONFS: &[RegexConfStr] = &[
    RegexConfStr {
        starts_with: "device-mapper: multipath:",
        regex: r"(?x)
                ^device-mapper:\s
                multipath:\s
                Failing\s
                path\s
                (?P<kdev>\d+:\d+).$
                ",
        sub_system: "multipath",
        event_type: "DM_MPATH_PATH_FAILED",
    },
    RegexConfStr {
        starts_with: "device-mapper: multipath:",
        regex: r"(?x)
                ^device-mapper:\s
                multipath:\s
                Reinstating\s
                path\s
                (?P<kdev>\d+:\d+).$
                ",
        sub_system: "multipath",
        event_type: "DM_MPATH_PATH_REINSTATED",
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
        let sysfs_path = format!("/sys/class/scsi_disk/{}/device/block",
                                 scsi_id);
        let mut blks = fs::read_dir(&sysfs_path).unwrap();
        if let Some(Ok(blk)) = blks.next() {
            return blk.path().file_name().and_then(OsStr::to_str).unwrap().to_string();
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
