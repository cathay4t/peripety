use std::collections::HashMap;

#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub enum StorageSubSystem {
    Scsi,
    LvmThin,
    Multipath,
    Block,
    Fs,
    Mdraid,
    Other,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct StorageEvent {
    pub hostname: String,
    pub severity: LogSeverity,
    pub sub_system: StorageSubSystem,
    pub timestamp: u64,
    pub event_id: String,
    pub event_type: String,
    pub dev_wwid: String,
    pub dev_name: String,
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
            msg: String::new(),
            extention: HashMap::new(),
        }
    }
}
