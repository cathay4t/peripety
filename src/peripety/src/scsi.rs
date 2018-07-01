use super::blk_info::{BlkInfo, BlkType};
use super::error::PeripetyError;
use super::sysfs::Sysfs;

use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub(crate) fn scsi_id_to_blk_name(
    scsi_id: &str,
) -> Result<String, PeripetyError> {
    let sysfs_path = format!("/sys/class/scsi_disk/{}/device/block", scsi_id);
    if !Path::new(&sysfs_path).exists() {
        return Err(PeripetyError::BlockNoExists(format!(
            "SCSI ID {} does not exists",
            scsi_id
        )));
    }
    let mut blks = match fs::read_dir(&sysfs_path) {
        Ok(b) => b,
        Err(e) => {
            return Err(PeripetyError::InternalBug(format!(
                "scsi_id_to_blk_name(): Failed to read_dir {}: {}",
                sysfs_path, e
            )))
        }
    };

    if let Some(Ok(blk)) = blks.next() {
        if let Some(n) = blk.path().file_name() {
            if let Some(s) = n.to_str() {
                return Ok(s.to_string());
            }
        }
    }

    Err(PeripetyError::InternalBug(format!(
        "scsi::scsi_id_to_blk_name(): Folder {} is empty",
        sysfs_path,
    )))
}

pub(crate) fn pretty_wwid(wwid: &str) -> String {
    let s = Regex::new(r"[ \t]+")
        .map(|r| r.replace_all(wwid.trim(), "-"))
        .expect("BUG: pretty_wwid()");
    // we never panic as above regex string is valid.
    Regex::new(r"(\\0)+$")
        .map(|r| r.replace_all(&s, ""))
        .expect("BUG: pretty_wwid()")
        // we never panic as above regex string is valid.
        .to_string()
}

// Support query on these formats:
//  * 4:0:0:1
//  * sda
//  * sda1
pub(crate) fn blk_info_get_scsi(blk: &str) -> Result<BlkInfo, PeripetyError> {
    let name;

    // Check if partition
    if let Ok(reg) = Regex::new("^(sd[a-z]+)([0-9]+)$") {
        if let Some(cap) = reg.captures(blk) {
            let name = cap.get(1)
                .expect("BUG: blk_info_get_scsi()")
                // ^ We never panic as above regex is valid.
                .as_str();
            let part = cap.get(2)
                .expect("BUG: blk_info_get_scsi()")
                // ^ We never panic as above regex is valid.
                .as_str();
            let blk_info = BlkInfo::new(&name)?;
            let mut blk_path = format!("/dev/{}", &blk);
            let uuid = match BlkInfo::uuid(&blk_path) {
                Ok(u) => Some(u),
                Err(_) => None,
            };
            // udev create mpatha-part1 while kpartx create mpatha1
            if blk_info.blk_type == BlkType::DmMultipath {
                blk_path = format!("{}-part{}", blk_info.blk_path, part);
                if !Path::new(&blk_path).exists() {
                    blk_path = format!("{}{}", blk_info.blk_path, part);
                    if !Path::new(&blk_path).exists() {
                        return Err(PeripetyError::BlockNoExists(format!(
                            "Multipath partition file of {} partition {} \
                             is missing, please use kpartx to create them",
                            blk_info.blk_path, part
                        )));
                    }
                }
            }

            let preferred_blk_path = if let Some(ref u) = uuid {
                format!("/dev/disk/by-uuid/{}", u)
            } else {
                get_prefered_blk_path(&blk_path)
            };
            let mut ret = BlkInfo {
                wwid: format!("{}-part{}", blk_info.wwid, part),
                blk_type: BlkType::Partition,
                blk_path,
                preferred_blk_path,
                uuid,
                owners_wwids: blk_info.owners_wwids,
                owners_types: blk_info.owners_types,
                owners_paths: blk_info.owners_paths,
                owners_transport_ids: blk_info.owners_transport_ids,
                mount_point: None,
                transport_id: "".to_string(),
            };
            ret.owners_wwids.insert(0, blk_info.wwid);
            ret.owners_types.insert(0, blk_info.blk_type);
            ret.owners_paths.insert(0, blk_info.blk_path);
            return Ok(ret);
        }
    }

    // Try 4:0:0:1 format
    let mut sysfs_path = format!("/sys/class/scsi_disk/{}/device/wwid", &blk);
    if Path::new(&sysfs_path).exists() {
        name = scsi_id_to_blk_name(blk)?;
    } else {
        // Try sda format
        sysfs_path = format!("/sys/block/{}/device/wwid", &blk);
        name = blk.to_string();
    }

    if Path::new(&sysfs_path).exists() {
        let blk_path = format!("/dev/{}", &name);
        return Ok(BlkInfo {
            wwid: pretty_wwid(&Sysfs::read(&sysfs_path)?),
            blk_type: BlkType::Scsi,
            preferred_blk_path: get_prefered_blk_path(&blk_path),
            blk_path,
            owners_wwids: Vec::new(),
            owners_types: Vec::new(),
            owners_paths: Vec::new(),
            owners_transport_ids: Vec::new(),
            uuid: None,
            mount_point: None,
            transport_id: get_transport_id(&name)?,
        });
    }

    Err(PeripetyError::InternalBug(format!(
        "scsi::blk_info_get_scsi(): Got invalid scsi blk {}",
        blk
    )))
}

fn get_prefered_blk_path(raw_blk_path: &str) -> String {
    let dev_folder = "/dev/disk/by-id";
    let raw_path = Path::new(raw_blk_path);
    let mut matches = Vec::new();
    if let Ok(entries) = fs::read_dir(dev_folder) {
        for entry in entries {
            let e = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if let Some(s) = e.file_name().to_str() {
                if let Ok(p) = Path::new(dev_folder).join(s).canonicalize() {
                    if p == raw_path {
                        matches.push(s.to_string())
                    }
                }
            }
        }
    }
    for s in &matches {
        if s.starts_with("wwn-") {
            return format!("{}/{}", dev_folder, s);
        }
    }

    match matches.get(0) {
        Some(s) => format!("{}/{}", dev_folder, s),
        None => raw_blk_path.to_string(),
    }
}

fn iscsi_session_id_of_host(host_id: &str) -> Result<String, PeripetyError> {
    let path = format!("/sys/class/iscsi_host/host{}", host_id);
    let p = match fs::read_link(&path) {
        Ok(l) => match l.to_str() {
            Some(s) => s.to_string(),
            None => {
                return Err(PeripetyError::InternalBug(format!(
                    "iscsi_session_id_of_host: Got non-unicode: {:?}",
                    path
                )));
            }
        },
        Err(e) => {
            return Err(PeripetyError::InternalBug(format!(
                "iscsi_session_id_of_host: Error when read_link {}: {}",
                path, e
            )));
        }
    };
    let dev_path = match Regex::new(r"\.+/(devices/.+/host[0-9]+)/iscsi_host/")
        .expect("BUG: Regex string should be valid")
        .captures(&p)
    {
        Some(c) => format!(
            "/sys/{}",
            c.get(1).expect("BUG: Regex capture group missing").as_str()
        ),
        None => {
            return Err(PeripetyError::InternalBug(format!(
                "iscsi_session_id_of_host: Failed to do regex \
                 parsing on path {}",
                p
            )));
        }
    };

    let dir_entries = match fs::read_dir(&dev_path) {
        Ok(b) => b,
        Err(e) => {
            return Err(PeripetyError::InternalBug(format!(
                "iscsi_session_id_of_host: Failed to read_dir {}: {}",
                p, e
            )))
        }
    };
    for e in dir_entries {
        if let Ok(e) = e {
            if let Ok(name) = e.file_name().into_string() {
                if name.starts_with("session") {
                    return Ok(name["session".len()..].to_string());
                }
            }
        }
    }
    return Err(PeripetyError::InternalBug(format!(
        "iscsi_session_id_of_host: Failed to find session id of \
         host {}",
        host_id
    )));
}

fn get_iscsi_info(
    host_id: &str,
) -> Result<HashMap<String, String>, PeripetyError> {
    let mut ret = HashMap::new();
    let sid = iscsi_session_id_of_host(host_id)?;
    let session_dir = format!("/sys/class/iscsi_session/session{}", sid);
    let conn_dir = format!("/sys/class/iscsi_connection/connection{}:0", sid);
    if !Path::new(&session_dir).exists() {
        return Err(PeripetyError::InternalBug(format!(
            "iSCSI Session dir {} does not exists",
            session_dir
        )));
    }
    if !Path::new(&conn_dir).exists() {
        return Err(PeripetyError::InternalBug(format!(
            "iSCSI connection dir {} does not exists",
            conn_dir
        )));
    }
    ret.insert(
        "address".to_string(),
        Sysfs::read(&format!("{}/{}", conn_dir, "address"))?,
    );
    ret.insert(
        "port".to_string(),
        Sysfs::read(&format!("{}/{}", conn_dir, "port"))?,
    );
    ret.insert(
        "tpgt".to_string(),
        Sysfs::read(&format!("{}/{}", session_dir, "tpgt"))?,
    );
    ret.insert(
        "target_name".to_string(),
        Sysfs::read(&format!("{}/{}", session_dir, "targetname"))?,
    );
    ret.insert(
        "iface_name".to_string(),
        Sysfs::read(&format!("{}/{}", session_dir, "ifacename"))?,
    );

    Ok(ret)
}

fn get_fc_info(
    host_id: &str,
    scsi_id: &str,
) -> Result<HashMap<String, String>, PeripetyError> {
    let mut ret = HashMap::new();
    // fc-hosts are using the same host id with scsi host.
    if let Some(index) = scsi_id.rfind(':') {
        let target_id = &scsi_id[..index];
        let target_dir = format!("/sys/class/fc_transport/target{}", target_id);
        let host_dir = format!("/sys/class/fc_host/host{}", host_id);
        if !Path::new(&host_dir).exists() {
            return Err(PeripetyError::InternalBug(format!(
                "FC host dir {} does not exists",
                host_dir
            )));
        }
        if !Path::new(&target_dir).exists() {
            return Err(PeripetyError::InternalBug(format!(
                "FC transport dir {} does not exists",
                target_dir
            )));
        }
        ret.insert(
            "target_wwpn".to_string(),
            Sysfs::read(&format!("{}/{}", target_dir, "port_name"))?,
        );
        ret.insert(
            "host_wwpn".to_string(),
            Sysfs::read(&format!("{}/{}", host_dir, "port_name"))?,
        );
        ret.insert(
            "speed".to_string(),
            Sysfs::read(&format!("{}/{}", host_dir, "speed"))?,
        );
        ret.insert(
            "port_state".to_string(),
            Sysfs::read(&format!("{}/{}", host_dir, "port_state"))?,
        );
    } else {
        return Err(PeripetyError::InternalBug(format!(
            "Got invalid scsi_id {}",
            scsi_id
        )));
    }

    Ok(ret)
}

fn is_iscsi_host(host_id: &str) -> bool {
    Path::new(&format!("/sys/class/iscsi_host/host{}", host_id)).exists()
}

fn is_fc_host(host_id: &str) -> bool {
    Path::new(&format!("/sys/class/fc_host/host{}", host_id)).exists()
}

fn get_scsi_transport_info(
    sd_name: &str,
) -> Result<HashMap<String, String>, PeripetyError> {
    let mut ret = HashMap::new();
    let scsi_id = scsi_id_of_disk(sd_name)?;
    let host_id = match scsi_host_id_of_scsi_id(&scsi_id) {
        Some(h) => h,
        None => {
            return Err(PeripetyError::InternalBug(format!(
                "Failed to query scsi_host_id of disk {}",
                sd_name
            )))
        }
    };
    ret.insert(
        "driver_name".to_string(),
        Sysfs::read(&format!(
            "/sys/class/scsi_host/host{}/proc_name",
            &host_id
        ))?,
    );
    if is_iscsi_host(&host_id) {
        ret.insert("transport".to_string(), "iSCSI".to_string());
        for (key, value) in get_iscsi_info(&host_id)? {
            ret.insert(key, value);
        }
    } else if is_fc_host(&host_id) {
        ret.insert("transport".to_string(), "FC".to_string());
        for (key, value) in get_fc_info(&host_id, &scsi_id)? {
            ret.insert(key, value);
        }
    }

    Ok(ret)
}

fn get_transport_id(sd_name: &str) -> Result<String, PeripetyError> {
    let info = get_scsi_transport_info(sd_name)?;
    match info.get("transport") {
        Some(t) => {
            if t == "iSCSI" {
                Ok(format!(
                    "{},{},{},{},{}",
                    info["address"],
                    info["port"],
                    info["tpgt"],
                    info["target_name"],
                    info["iface_name"]
                ))
            // ^ get_iscsi_info() has already ensured these keys exist.
            } else if t == "FC" {
                Ok(format!(
                    "{},{}",
                    info["host_wwpn"], info["target_wwpn"]
                ))
            // ^ get_fc_info() has already ensured these keys exist.
            } else {
                Ok("".to_string())
            }
        }
        None => Ok("".to_string()),
    }
}

fn scsi_host_id_of_scsi_id(scsi_id: &str) -> Option<String> {
    if let Some(index) = scsi_id.find(':') {
        return Some(scsi_id[..index].to_string());
    }

    None
}

fn scsi_id_of_disk(sd_name: &str) -> Result<String, PeripetyError> {
    let sysfs_path = format!("/sys/block/{}/device", sd_name);
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
                "scsi_host_id_of_disk(): Failed to read link {}: {}",
                sysfs_path, e
            )))
        }
    };
    Err(PeripetyError::InternalBug(format!(
        "Failed to query scsi_id of disk {}",
        sd_name
    )))
}
