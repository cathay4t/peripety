use super::error::PeripetyError;
use std::fs;
use std::io::Read;

pub(crate) struct Sysfs;

impl Sysfs {
    pub(crate) fn major_minor_to_blk_name(
        major_minor: &str,
    ) -> Result<String, PeripetyError> {
        let sysfs_path = format!("/sys/dev/block/{}", major_minor);
        match fs::read_link(&sysfs_path) {
            Ok(p) => {
                if let Some(p) = p.file_name() {
                    if let Some(s) = p.to_str() {
                        return Ok(s.to_string());
                    }
                }
            }
            Err(e) => {
                return Err(PeripetyError::InternalBug(format!(
                    "sysfs::major_minor_to_blk_name(): \
                     Failed to read link {}: {}",
                    sysfs_path, e
                )));
            }
        };

        Err(PeripetyError::InternalBug(format!(
            "sysfs::major_minor_to_blk_name():  Got non-utf8 path {}",
            sysfs_path
        )))
    }

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
                if contents.ends_with("\n") {
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
