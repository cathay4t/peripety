extern crate dmmp;
extern crate peripety;

use peripety::{Ipc, StorageEvent};

fn handle_event(e: &mut StorageEvent) {
    let mut matched: bool = false;
    for mp in dmmp::mpaths_get() {
        for pg in mp.path_groups {
            for p in pg.paths {
                if p.major_minor == e.dev_name {
                    e.extention.insert("path_name".to_string(),
                                       p.dev_name);
                    e.extention.insert("path_major_minor".to_string(),
                                       p.major_minor);
                    e.dev_wwid = mp.wwid.to_owned();
                    e.dev_name = mp.name.to_owned();
                    matched = true;
                    break;
                }
            }
            if matched {
                break
            }
        }
        if matched {
            break;
        }
    }
}

fn main() {
    let so = Ipc::parser_ipc("mpath");
    loop {
        let msg = Ipc::ipc_recv(&so);
        let mut e: StorageEvent = StorageEvent::from_json_string(&msg);
        handle_event(&mut e);
        if ! e.dev_wwid.is_empty() {
            Ipc::ipc_send(&so, &StorageEvent::to_json_string(&e));
        }
    }
}
