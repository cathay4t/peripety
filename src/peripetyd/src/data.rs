extern crate peripety;
extern crate regex;

use regex::Regex;
use std::sync::mpsc::Sender;
use peripety::{StorageEvent, StorageSubSystem};

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
    pub sub_system: StorageSubSystem,
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
