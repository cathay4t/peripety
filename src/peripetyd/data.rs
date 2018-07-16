use peripety::{StorageEvent, StorageSubSystem};
use regex::Regex;
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
            regex: Regex::new(self.regex).unwrap_or_else(|_| {
                panic!("BUG: data.rs has invalid regex: {}", self.regex)
            }),
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

pub struct Sysfs;

impl Sysfs {
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
        if let Some(index) = scsi_id.find(':') {
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
                if contents.ends_with('\n') {
                    contents.pop();
                }
            }
            Err(e) => {
                println!("Sysfs::read(): Failed to read file {}: {}", path, e);
            }
        };
        contents
    }
}
