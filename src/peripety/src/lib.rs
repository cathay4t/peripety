extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
use std::collections::HashMap;
use std::str;
use std::str::FromStr;

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
// https://tools.ietf.org/html/rfc5424#section-6.2.1
pub enum LogSeverity {
    Emergency = 0,
    Alert = 1,
    Ctritical = 2,
    Error = 3,
    Warning = 4,
    Notice = 5,
    Info = 6,
    Debug = 7,
}

impl FromStr for LogSeverity {
    type Err = ();
    fn from_str(s: &str) -> Result<LogSeverity, ()> {
        match s.as_ref() {
            "0" => Ok(LogSeverity::Emergency),
            "1" => Ok(LogSeverity::Alert),
            "2" => Ok(LogSeverity::Ctritical),
            "3" => Ok(LogSeverity::Error),
            "4" => Ok(LogSeverity::Warning),
            "5" => Ok(LogSeverity::Notice),
            "6" => Ok(LogSeverity::Info),
            "7" => Ok(LogSeverity::Debug),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum StorageSubSystem {
    Unknown,
    Other,
    Scsi,
    LvmThin,
    Multipath,
    FsExt4,
    Nvme,
}

impl FromStr for StorageSubSystem {
    type Err = ();
    fn from_str(s: &str) -> Result<StorageSubSystem, ()> {
        match s.to_uppercase().as_ref() {
            "SCSI" => Ok(StorageSubSystem::Scsi),
            "LVMTHIN" => Ok(StorageSubSystem::LvmThin),
            "MULTIPATH" => Ok(StorageSubSystem::Multipath),
            "EXT4" => Ok(StorageSubSystem::FsExt4),
            "NVME" => Ok(StorageSubSystem::Nvme),
            _ => Ok(StorageSubSystem::Unknown),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StorageEvent {
    pub hostname: String,
    pub severity: LogSeverity,
    pub sub_system: StorageSubSystem,
    pub timestamp: u64,
    pub event_id: String,
    pub event_type: String,
    pub dev_wwid: String,
    pub dev_name: String,
    pub dev_path: String,
    pub holders_wwids: Vec<String>,
    pub holders_names: Vec<String>,
    pub holders_paths: Vec<String>,
    // ^ What devices does current dev_wwid depending on.
    pub kdev: String,       // internal use-only: kernel device name.
    pub msg: String,
    pub extention: HashMap<String, String>,
}

impl Default for StorageEvent {
    fn default() -> StorageEvent {
        StorageEvent {
            hostname: String::new(),
            severity: LogSeverity::Debug,
            sub_system: StorageSubSystem::Unknown,
            timestamp: 0,
            event_id: String::new(),
            event_type: String::new(),
            dev_wwid: String::new(),
            dev_name: String::new(),
            dev_path: String::new(),
            holders_wwids: Vec::new(),
            holders_names: Vec::new(),
            holders_paths: Vec::new(),
            kdev: String::new(),
            msg: String::new(),
            extention: HashMap::new(),
        }
    }
}

impl StorageEvent {
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
    pub fn from_json_string(json_string: &str) -> StorageEvent {
        serde_json::from_str(json_string).unwrap()
    }
    pub fn from_slice(buff: &[u8]) -> StorageEvent {
        // We cannot use serde_json::from_slice, as buff might have trailing \0
        // where serde_json will raise error.
        let tmp_s = str::from_utf8(buff).unwrap().trim_right_matches('\0');
        serde_json::from_str(tmp_s).unwrap()
    }
}
