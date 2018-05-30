use super::error::PeripetyError;

use serde_json;
use std::collections::HashMap;
use std::fmt;
use std::str::{self, FromStr};

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd)]
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
        match s.to_uppercase().as_ref() {
            "0" | "EMERGENCY" => Ok(LogSeverity::Emergency),
            "1" | "ALERT" => Ok(LogSeverity::Alert),
            "2" | "CRITICAL" => Ok(LogSeverity::Ctritical),
            "3" | "ERROR" => Ok(LogSeverity::Error),
            "4" | "WARNING" => Ok(LogSeverity::Warning),
            "5" | "Notice" => Ok(LogSeverity::Notice),
            "6" | "INFO" => Ok(LogSeverity::Info),
            "7" | "DEBUG" => Ok(LogSeverity::Debug),
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
    DmDirtyLog,
    LvmThin,
    Multipath,
    FsExt4,
    FsJbd2, // The generic journaling layer for block used by ext4 and ocfs2.
    FsXfs,
    Nvme,
}

impl FromStr for StorageSubSystem {
    type Err = PeripetyError;
    fn from_str(s: &str) -> Result<StorageSubSystem, PeripetyError> {
        match s.to_uppercase().as_ref() {
            "SCSI" => Ok(StorageSubSystem::Scsi),
            "DM-DIRTYLOG" => Ok(StorageSubSystem::DmDirtyLog),
            "LVM-THINPROVISIONING" => Ok(StorageSubSystem::LvmThin),
            "MULTIPATH" => Ok(StorageSubSystem::Multipath),
            "EXT4" => Ok(StorageSubSystem::FsExt4),
            "XFS" => Ok(StorageSubSystem::FsXfs),
            "NVME" => Ok(StorageSubSystem::Nvme),
            _ => Err(PeripetyError::StorageSubSystemParseError(
                format!("Invalid StorageSubSystem string {}", s),
            )),
        }
    }
}

impl fmt::Display for StorageSubSystem {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StorageSubSystem::Unknown => write!(fmt, "Unknown"),
            StorageSubSystem::Other => write!(fmt, "Other"),
            StorageSubSystem::Scsi => write!(fmt, "SCSI"),
            StorageSubSystem::DmDirtyLog => write!(fmt, "DM-DirtyLog"),
            StorageSubSystem::LvmThin => write!(fmt, "LVM-ThinProvisioning"),
            StorageSubSystem::Multipath => write!(fmt, "Multipath"),
            StorageSubSystem::FsExt4 => write!(fmt, "ext4"),
            StorageSubSystem::FsJbd2 => write!(fmt, "jbd2"),
            StorageSubSystem::FsXfs => write!(fmt, "xfs"),
            StorageSubSystem::Nvme => write!(fmt, "NVMe"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StorageEvent {
    pub hostname: String,
    pub severity: LogSeverity,
    pub sub_system: StorageSubSystem,
    pub timestamp: String,
    pub event_id: String,
    pub event_type: String,
    pub dev_wwid: String,
    pub dev_path: String,
    pub owners_wwids: Vec<String>,
    pub owners_paths: Vec<String>,
    // ^ What devices does current dev_wwid depending on.
    #[serde(skip_serializing, skip_deserializing)]
    pub kdev: String, // internal use-only: kernel device name.
    pub msg: String,
    pub raw_msg: String,
    pub extension: HashMap<String, String>,
}

impl Default for StorageEvent {
    fn default() -> StorageEvent {
        StorageEvent {
            hostname: String::new(),
            severity: LogSeverity::Unknown,
            sub_system: StorageSubSystem::Unknown,
            timestamp: String::new(),
            event_id: String::new(),
            event_type: String::new(),
            dev_wwid: String::new(),
            dev_path: String::new(),
            owners_wwids: Vec::new(),
            owners_paths: Vec::new(),
            kdev: String::new(),
            msg: String::new(),
            raw_msg: String::new(),
            extension: HashMap::new(),
        }
    }
}

impl StorageEvent {
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
    pub fn from_json_string(
        json_string: &str,
    ) -> Result<StorageEvent, PeripetyError> {
        match serde_json::from_str(json_string) {
            Ok(e) => Ok(e),
            Err(e) => Err(PeripetyError::JsonDeserializeError(format!(
                "{}",
                e
            ))),
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
            Err(e) => Err(PeripetyError::JsonDeserializeError(format!(
                "{}",
                e
            ))),
        }
    }
}
