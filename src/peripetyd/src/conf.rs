use toml;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use regex::Regex;
use data::RegexConf;

static CONFIG_PATH: &'static str = "/etc/peripetyd.conf";

#[derive(Deserialize, Debug)]
pub struct ConfMain {
    pub notify_stdout: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct ConfCollectorRegex {
    pub regex: String,
    pub event_type: String,
    pub starts_with: Option<String>,
    pub sub_system: String,
}

impl ConfCollectorRegex {
    pub fn to_regex_conf(&self) -> RegexConf {
        RegexConf {
            starts_with: self.starts_with.clone(),
            regex: Regex::new(&self.regex).unwrap(),
            sub_system: self.sub_system.parse().unwrap(),
            event_type: self.event_type.clone(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ConfCollector {
    pub regexs: Vec<ConfCollectorRegex>,
}

#[derive(Deserialize, Debug)]
pub struct Conf {
    pub main: ConfMain,
    pub collector: ConfCollector,
}

pub fn load_conf() -> Option<Conf> {
    let path = Path::new(CONFIG_PATH);
    if !path.exists() {
        println!("Config file {} does not exist", CONFIG_PATH);
        return None;
    }

    let mut fd = match File::open(path) {
        Ok(fd) => fd,
        Err(e) => {
            println!("Failed to open config file {}, error {}", CONFIG_PATH, e);
            return None;
        }
    };
    let mut contents = String::new();
    if let Err(e) = fd.read_to_string(&mut contents) {
        println!("Fail to read config file {}, error {}", CONFIG_PATH, e);
        return None;
    }

    match toml::from_str(&contents) {
        Ok(c) => Some(c),
        Err(e) => {
            println!("Fail to parse config file {}, error {}", CONFIG_PATH, e);
            None
        }
    }
}
