use peripety::StorageSubSystem;
use regex::Regex;

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
