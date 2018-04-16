<!-- vim-markdown-toc GFM -->

* [Multipath path failure](#multipath-path-failure)
* [Sector error](#sector-error)

<!-- vim-markdown-toc -->

## Multipath path failure

## Sector error

 * Raw log when using targetcli SCSI disk:
```
6,1239,9436467326,-;sd 6:0:0:0: [sdf] tag#2 FAILED Result: hostbyte=DID_OK driverbyte=DRIVER_SENSE
 SUBSYSTEM=scsi
 DEVICE=+scsi:6:0:0:0
6,1240,9436467330,-;sd 6:0:0:0: [sdf] tag#2 Sense Key : Not Ready [current]
 SUBSYSTEM=scsi
 DEVICE=+scsi:6:0:0:0
6,1241,9436467332,-;sd 6:0:0:0: [sdf] tag#2 Add. Sense: Logical unit communication failure
 SUBSYSTEM=scsi
 DEVICE=+scsi:6:0:0:0
6,1242,9436467334,-;sd 6:0:0:0: [sdf] tag#2 CDB: Write(10) 2a 00 00 00 27 00 00 00 40 00
 SUBSYSTEM=scsi
 DEVICE=+scsi:6:0:0:0
3,1243,9436467335,-;print_req_error: I/O error, dev sdf, sector 9984
4,1244,9436471084,-;EXT4-fs warning (device sdf): ext4_end_bio:323: I/O error
10 writing to inode 12 (offset 464060416 size 1507328 starting block 6432)
4,1245,9436471087,-;buffer_io_error: 506 callbacks suppressed
3,1246,9436471088,-;Buffer I/O error on device sdf, logical block 4960
3,1247,9436471090,-;Buffer I/O error on device sdf, logical block 4961
3,1248,9436471091,-;Buffer I/O error on device sdf, logical block 4962
3,1249,9436471092,-;Buffer I/O error on device sdf, logical block 4963
3,1250,9436471095,-;Buffer I/O error on device sdf, logical block 4964
3,1251,9436471096,-;Buffer I/O error on device sdf, logical block 4965
3,1252,9436471097,-;Buffer I/O error on device sdf, logical block 4966
3,1253,9436471098,-;Buffer I/O error on device sdf, logical block 4967
3,1254,9436471100,-;Buffer I/O error on device sdf, logical block 4968
3,1255,9436471101,-;Buffer I/O error on device sdf, logical block 4969
4,1256,9440920164,-;JBD2: Detected IO errors while flushing file data on sdf-8
4,1257,9473175718,-;JBD2: Detected IO errors while flushing file data on sdf-8
```

 * Root cause - Disk SMART data:
   ```
    {
        event_type: "SMART_DISK_FAIL",
        severity: "Error",
        event_id: "decb3078-8179-455d-8f60-cc198a8d2712"
        root_cause_event_id: "decb3078-8179-455d-8f60-cc198a8d2712",
        dev_wwid: "60014053fba66028277457f86a4e6591",
        dev_name: "/dev/sdf",
        msg: "The SMART of disk /dev/sdf(60014053fba66028277457f86a4e6591) "
             "indicates a disk failure for Reallocated_Sector_Ct",
        extetions: {
            "name": "Reallocated_Sector_Ct",
            "value": 29,        // 255 is best, 0 is worst.
            "worst_value": 29,  // worst value since SMART enabeld.
            "threshhold": 140,  // below which is considerd bad.
            "raw_value": 1365,
        }
    }
   ```

 * SCSI layer:
    ```
    {
        event_type: "SCSI_DRIVE_IO_ERROR",
        severity: "Error",
        timestamp: 1522130579,
        event_id: "7fedf8b3-abac-48d8-899b-c70a18fed4f7",
        root_cause_event_id: "7fedf8b3-abac-48d8-899b-c70a18fed4f7",
        // ^ If we managed to find root cause, else we set it to itself.
        dev_wwid: "60014053fba66028277457f86a4e6591",
        dev_name: "/dev/sdf",
        msg: "SCSI disk sdf/60014053fba66028277457f86a4e6591 I/O failure: "
             "not ready, logical unit communication failure",
        extetions: {
            "block_range_start":  9984,     // parse from CDB: 0x00002700
            "block_range_size": 64,         // parse from CDB: 0x40
            "sense_key_msg": "NOT READY",
            "sense_msg": "LOGICAL UNIT COMMUNICATION FAILURE",
            "asc": 0x08,
            "ascq": 0x00,
            "sense_key": 0x2,
        }
    }
    ```

 * Collateral damages:

    ```
    {
        event_type: "FS_IO_ERROR",
        severity: "Error",
        timestamp: 1522130579,
        event_id: "92e78bbb-e790-4f96-a55b-11f995743c19",
        root_cause_event_id: "7fedf8b3-abac-48d8-899b-c70a18fed4f7",
        dev_wwid: "7e036c72-86d4-4315-83ce-90ff842ab9b9",
        // ^ UUID of file system.
        dev_name: "/dev/sdf",
        msg: "File system on /dev/sdf(/mnt, "
             "7e036c72-86d4-4315-83ce-90ff842ab9b9) got I/O error"
        extetions: {
            "mount_point":  "/mnt",         // parse from mount-table
        }
    }
    ```
