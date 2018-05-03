extern crate peripety;
extern crate regex;

use data::{BlkInfo, BlkType, Sysfs};
use std::path::Path;
use std::fs;

// Support query on these formats:
//  * dm-0
pub fn blk_info_get_dm(kdev: &str) -> Option<BlkInfo> {
    // TODO Check if partition
    let sysfs_uuid = format!("/sys/block/{}/dm/uuid", &kdev);

    if Path::new(&sysfs_uuid).exists() {
        let sysfs_name = format!("/sys/block/{}/dm/name", &kdev);
        let name = Sysfs::read(&sysfs_name);
        let mut ret = BlkInfo {
            wwid: Sysfs::read(&sysfs_uuid),
            blk_type: BlkType::Dm,
            blk_path: format!("/dev/mapper/{}", &name),
            name: name,
            owners_wwids: Vec::new(),
            owners_types: Vec::new(),
            owners_names: Vec::new(),
            owners_paths: Vec::new(),
        };
        if ret.wwid.starts_with("LVM-") {
            ret.blk_type = BlkType::DmLvm;
        } else if ret.wwid.starts_with("mpath-") {
            ret.blk_type = BlkType::DmMultipath;
        }
        let slave_dir = format!("/sys/block/{}/slaves", &kdev);
        for entry in fs::read_dir(&slave_dir).unwrap() {
            let slave = entry.unwrap().path();
            let slave_kdev = slave.file_name().unwrap().to_str().unwrap();
            if let Some(slave_info) = BlkInfo::new(slave_kdev) {
                if !ret.owners_wwids.contains(&slave_info.wwid) {
                    ret.owners_wwids.push(slave_info.wwid.clone());
                    ret.owners_types.push(slave_info.blk_type.clone());
                    ret.owners_names.push(slave_info.name.clone());
                    ret.owners_paths.push(slave_info.blk_path.clone());
                }
                if slave_info.blk_type == BlkType::DmLvm
                    || slave_info.blk_type == BlkType::Dm
                    || slave_info.blk_type == BlkType::DmMultipath
                {
                    for sub_slave_blk_path in slave_info.owners_paths {
                        let sub_slave_kdev = Path::new(&sub_slave_blk_path)
                            .canonicalize()
                            .unwrap();
                        let sub_slave_kdev = sub_slave_kdev
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap();

                        if let Some(sub_slave_info) =
                            BlkInfo::new(sub_slave_kdev)
                        {
                            if !ret.owners_wwids.contains(&sub_slave_info.wwid)
                            {
                                ret.owners_wwids.push(sub_slave_info.wwid);
                                ret.owners_types.push(sub_slave_info.blk_type);
                                ret.owners_names.push(sub_slave_info.name);
                                ret.owners_paths.push(sub_slave_info.blk_path);
                            }
                        }
                    }
                }
            }
        }
        if ret.owners_wwids.len() == 0 {
            return None;
        }
        return Some(ret);
    }

    // TODO(Gris Ge): Handle partition

    None
}
