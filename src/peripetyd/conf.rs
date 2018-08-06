use data::RegexConf;
use peripety::{PeripetyError, StorageSubSystem};
use regex::Regex;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml;

use std::io::Write;

static CONFIG_PATH: &'static str = "/etc/peripetyd.conf";

#[derive(Deserialize, Debug)]
pub struct ConfMain {
    pub save_to_journald: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct ConfRegex {
    pub regex: String,
    pub event_type: String,
    pub starts_with: Option<String>,
    pub sub_system: String,
}

impl ConfRegex {
    pub fn to_regex_conf(&self) -> Result<RegexConf, PeripetyError> {
        let regex = match Regex::new(&self.regex) {
            Ok(r) => r,
            Err(e) => {
                return Err(PeripetyError::ConfError(format!(
                    "Invalid regex: {}",
                    e
                )))
            }
        };
        let sub_system = match self.sub_system.parse::<StorageSubSystem>() {
            Ok(s) => s,
            Err(e) => {
                return Err(PeripetyError::ConfError(format!(
                    "Invalid sub_system: {}",
                    e
                )))
            }
        };
        Ok(RegexConf {
            starts_with: self.starts_with.clone(),
            regex,
            sub_system,
            event_type: self.event_type.clone(),
        })
    }
}

#[derive(Deserialize, Debug)]
pub struct PeripetyConf {
    pub main: ConfMain,
    pub regexs: Vec<ConfRegex>,
}

pub fn load_conf() -> Option<PeripetyConf> {
    let path = Path::new(CONFIG_PATH);
    if !path.exists() {
        to_stderr!("Config file {} does not exist", CONFIG_PATH);
        return None;
    }

    let mut fd = match File::open(path) {
        Ok(fd) => fd,
        Err(e) => {
            to_stderr!(
                "Failed to open config file {}, error {}",
                CONFIG_PATH,
                e
            );
            return None;
        }
    };
    let mut contents = String::new();
    if let Err(e) = fd.read_to_string(&mut contents) {
        to_stderr!("Fail to read config file {}, error {}", CONFIG_PATH, e);
        return None;
    }

    match toml::from_str(&contents) {
        Ok(c) => Some(c),
        Err(e) => {
            to_stderr!(
                "Fail to parse config file {}, error {}",
                CONFIG_PATH,
                e
            );
            None
        }
    }
}
