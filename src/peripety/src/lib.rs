extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
use std::collections::HashMap;
use std::str::{self, FromStr};
use std::fmt;

#[derive(Debug)]
pub enum PeripetyError {
    LogSeverityParseError(String),
    ConfError(String),
    StorageSubSystemParseError(String),
    JsonSerializeError(String),
    JsonDeserializeError(String),
}

impl fmt::Display for PeripetyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                PeripetyError::LogSeverityParseError(ref x)
                | PeripetyError::ConfError(ref x)
                | PeripetyError::JsonSerializeError(ref x)
                | PeripetyError::JsonDeserializeError(ref x)
                | PeripetyError::StorageSubSystemParseError(ref x) => x,
            }
        )
    }
}

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
    Unknown = 255,
}

impl FromStr for LogSeverity {
    type Err = PeripetyError;
    fn from_str(s: &str) -> Result<LogSeverity, PeripetyError> {
        match s.as_ref() {
            "0" => Ok(LogSeverity::Emergency),
            "1" => Ok(LogSeverity::Alert),
            "2" => Ok(LogSeverity::Ctritical),
            "3" => Ok(LogSeverity::Error),
            "4" => Ok(LogSeverity::Warning),
            "5" => Ok(LogSeverity::Notice),
            "6" => Ok(LogSeverity::Info),
            "7" => Ok(LogSeverity::Debug),
            _ => Err(PeripetyError::LogSeverityParseError(format!(
                "Invalid severity string {}",
                s
            ))),
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
    FsXfs,
    Nvme,
}

impl FromStr for StorageSubSystem {
    type Err = PeripetyError;
    fn from_str(s: &str) -> Result<StorageSubSystem, PeripetyError> {
        match s.to_uppercase().as_ref() {
            "SCSI" => Ok(StorageSubSystem::Scsi),
            "LVMTHIN" => Ok(StorageSubSystem::LvmThin),
            "MULTIPATH" => Ok(StorageSubSystem::Multipath),
            "EXT4" => Ok(StorageSubSystem::FsExt4),
            "XFS" => Ok(StorageSubSystem::FsXfs),
            "NVME" => Ok(StorageSubSystem::Nvme),
            _ => Err(PeripetyError::StorageSubSystemParseError(format!(
                "Invalid StorageSubSystem string {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for StorageSubSystem {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            StorageSubSystem::Unknown => write!(fmt, "Unknown"),
            StorageSubSystem::Other => write!(fmt, "Other"),
            StorageSubSystem::Scsi => write!(fmt, "SCSI"),
            StorageSubSystem::LvmThin => write!(fmt, "LvmThin"),
            StorageSubSystem::Multipath => write!(fmt, "Multipath"),
            StorageSubSystem::FsExt4 => write!(fmt, "FsExt4"),
            StorageSubSystem::FsXfs => write!(fmt, "FsXfs"),
            StorageSubSystem::Nvme => write!(fmt, "NVMe"),
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
    pub owners_wwids: Vec<String>,
    pub owners_names: Vec<String>,
    pub owners_paths: Vec<String>,
    // ^ What devices does current dev_wwid depending on.
    pub kdev: String, // internal use-only: kernel device name.
    pub msg: String,
    pub extention: HashMap<String, String>,
}

impl Default for StorageEvent {
    fn default() -> StorageEvent {
        StorageEvent {
            hostname: String::new(),
            severity: LogSeverity::Unknown,
            sub_system: StorageSubSystem::Unknown,
            timestamp: 0,
            event_id: String::new(),
            event_type: String::new(),
            dev_wwid: String::new(),
            dev_name: String::new(),
            dev_path: String::new(),
            owners_wwids: Vec::new(),
            owners_names: Vec::new(),
            owners_paths: Vec::new(),
            kdev: String::new(),
            msg: String::new(),
            extention: HashMap::new(),
        }
    }
}

impl StorageEvent {
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
    pub fn from_json_string(
        json_string: &str,
    ) -> Result<StorageEvent, PeripetyError> {
        match serde_json::from_str(json_string) {
            Ok(e) => Ok(e),
            Err(e) => {
                Err(PeripetyError::JsonDeserializeError(format!("{}", e)))
            }
        }
    }
    pub fn from_slice(buff: &[u8]) -> Result<StorageEvent, PeripetyError> {
        // We cannot use serde_json::from_slice, as buff might have trailing \0
        // where serde_json will raise error.
        let tmp_s = match str::from_utf8(buff) {
            Ok(s) => s.trim_right_matches('\0'),
            Err(e) => {
                return Err(PeripetyError::JsonDeserializeError(format!(
                    "{}",
                    e
                )))
            }
        };
        match serde_json::from_str(tmp_s) {
            Ok(e) => Ok(e),
            Err(e) => {
                Err(PeripetyError::JsonDeserializeError(format!("{}", e)))
            }
        }
    }
}
