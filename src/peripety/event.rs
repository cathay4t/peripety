// Copyright (C) 2018 Red Hat, Inc.
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//
// Author: Gris Ge <fge@redhat.com>

use super::blk_info::BlkInfo;
use super::error::PeripetyError;
use super::filter::{StorageEventFilter, StorageEventFilterType};

use chrono::{Datelike, Duration, Local, TimeZone};
use sdjournal::Journal;
use serde_json;
use std::collections::HashMap;
use std::fmt;
use std::str::{self, FromStr};

//TODO(Gris Ge): Add function StorageEvent::save_to_journal()

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
    Peripety, // For event generated by peripetyd itself.
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
            "JBD2" => Ok(StorageSubSystem::FsJbd2),
            "PERIPETY" => Ok(StorageSubSystem::Peripety),
            _ => Err(PeripetyError::StorageSubSystemParseError(format!(
                "Invalid StorageSubSystem string {}",
                s
            ))),
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
            StorageSubSystem::Peripety => write!(fmt, "Peripety"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageEvent {
    pub hostname: String,
    pub severity: LogSeverity,
    pub sub_system: StorageSubSystem,
    pub timestamp: String,
    pub event_id: String,
    pub event_type: String,
    pub cur_blk_info: BlkInfo,
    pub hierarchy_blk_info: BlkInfo,
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
            cur_blk_info: Default::default(),
            hierarchy_blk_info: Default::default(),
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

    pub fn query(
        filters: Option<&[StorageEventFilter]>,
    ) -> Result<StorageEventIter, PeripetyError> {
        let mut ret = StorageEventIter::new()?;
        if let Some(fts) = filters {
            ret.apply_filters(fts)?;
        }
        ret.journal.seek_head()?;
        Ok(ret)
    }

    pub fn monitor(
        filters: Option<&[StorageEventFilter]>,
    ) -> Result<StorageEventIter, PeripetyError> {
        let mut ret = StorageEventIter::new()?;
        if let Some(fts) = filters {
            ret.apply_filters(fts)?;
        }
        ret.journal.seek_tail()?;
        Ok(ret)
    }
}

pub struct StorageEventIter {
    journal: Journal,
    extra_filters: Vec<StorageEventFilter>,
}

impl Iterator for StorageEventIter {
    type Item = Result<StorageEvent, PeripetyError>;

    fn next(&mut self) -> Option<Result<StorageEvent, PeripetyError>> {
        loop {
            match self.journal.next() {
                Some(Ok(entry)) => {
                    if let Some(j) = entry.get("JSON") {
                        if let Ok(event) = StorageEvent::from_json_string(j) {
                            return Some(Ok(event));
                        }
                    }
                }
                Some(Err(e)) => return Some(Err(PeripetyError::from(e))),
                None => return None,
            }
        }
    }
}

impl StorageEventIter {
    pub fn new() -> Result<StorageEventIter, PeripetyError> {
        let mut journal = Journal::new()?;
        journal.timeout_us = 0;
        journal.add_match("IS_PERIPETY=TRUE")?;
        Ok(StorageEventIter {
            journal: journal,
            extra_filters: Vec::new(),
        })
    }

    pub fn apply_filters(
        &mut self,
        filters: &[StorageEventFilter],
    ) -> Result<(), PeripetyError> {
        for filter in filters {
            self.apply_filter(filter)?;
        }
        Ok(())
    }

    pub fn apply_filter(
        &mut self,
        filter: &StorageEventFilter,
    ) -> Result<(), PeripetyError> {
        // The sd_journal_add_match() only fail when no memory or running in
        // fork() (running in fork() is not supported by journald.
        // If the journal is running in fork(), we already get error from
        // PeripetySession::new().
        let filter_type = filter.filter_type.clone();
        let value = &filter.value;
        match filter.filter_type {
            StorageEventFilterType::Wwid => {
                self.extra_filters.push(StorageEventFilter {
                    filter_type: filter_type,
                    value: value.to_string(),
                })
            }
            StorageEventFilterType::EventType => {
                self.journal.add_match(&format!("EVENT_TYPE={}", value))?
            }
            StorageEventFilterType::Severity => {
                self.extra_filters.push(StorageEventFilter {
                    filter_type: filter_type,
                    value: value.to_string(),
                })
            }
            StorageEventFilterType::SubSystem => {
                self.journal.add_match(&format!("SUB_SYSTEM={}", value))?
            }
            StorageEventFilterType::Since => self
                .journal
                .seek_realtime_usec(since_str_to_jourald_timestamp(&value)?)?,
            StorageEventFilterType::EventId => {
                self.journal.add_match(&format!("EVENT_ID={}", value))?
            }
        }
        Ok(())
    }
}

fn time_str_to_u64(time_str: &str) -> Result<u64, PeripetyError> {
    if let Ok(t) = Local.datetime_from_str(time_str, "%F %H:%M:%S") {
        return Ok(t.timestamp() as u64 * 10u64.pow(6)
            + u64::from(t.timestamp_subsec_micros()));
    }
    Err(PeripetyError::InvalidArgument(
        "Invalid format of since time string".to_string(),
    ))
}

fn since_str_to_jourald_timestamp(since: &str) -> Result<u64, PeripetyError> {
    if since.to_uppercase() == "TODAY" {
        let dt = Local::now();
        return time_str_to_u64(&format!(
            "{}-{}-{} 00:00:00",
            dt.year(),
            dt.month(),
            dt.day()
        ));
    }

    if since.to_uppercase() == "YESTERDAY" {
        let dt = Local::now() - Duration::days(1);
        return time_str_to_u64(&format!(
            "{}-{}-{} 00:00:00",
            dt.year(),
            dt.month(),
            dt.day()
        ));
    }

    if since.contains(':') {
        return time_str_to_u64(since);
    }

    time_str_to_u64(&format!("{} 00:00:00", since))
}
