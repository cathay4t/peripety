use super::error::PeripetyError;
use std::fs;
use std::io::Read;

pub(crate) struct Sysfs;

impl Sysfs {
    pub(crate) fn read(path: &str) -> Result<String, PeripetyError> {
        let mut contents = String::new();
        match fs::File::open(path) {
            Ok(mut fd) => {
                if let Err(e) = fd.read_to_string(&mut contents) {
                    return Err(PeripetyError::InternalBug(format!(
                        "Sysfs::read(): Failed to read file {}: {}",
                        path, e
                    )));
                }
                if contents.ends_with('\n') {
                    contents.pop();
                }
            }
            Err(e) => {
                return Err(PeripetyError::InternalBug(format!(
                    "Sysfs::read(): Failed to read file {}: {}",
                    path, e
                )));
            }
        };
        Ok(contents)
    }
}
