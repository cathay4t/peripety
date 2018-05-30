use data::RegexConfStr;

pub const BUILD_IN_REGEX_CONFS: &[RegexConfStr] = &[
    RegexConfStr {
        starts_with: Some("device-mapper: multipath:"),
        regex: r"(?x)
                ^device-mapper:\s
                multipath:\ Failing\ path\s
                (?P<kdev>\d+:\d+).$
                ",
        sub_system: "multipath",
        event_type: "DM_MPATH_PATH_FAILED",
    },
    RegexConfStr {
        starts_with: Some("device-mapper: multipath:"),
        regex: r"(?x)
                ^device-mapper:\s
                multipath:\ Reinstating\ path\s
                (?P<kdev>\d+:\d+).$
                ",
        sub_system: "multipath",
        event_type: "DM_MPATH_PATH_REINSTATED",
    },
    RegexConfStr {
        starts_with: Some("device-mapper: dirty region log:"),
        regex: r"(?x)
                ^device-mapper:\s
                dirty\ region\ log:\s
                (?P<kdev>\d+:\d+):\ Failed\ to\ read\ header\ on\ dirty\s
                region\ log\ device$
                ",
        sub_system: "DM-DirtyLog",
        event_type: "DM_DIRTY_LOG_READ_FAILED",
    },
    RegexConfStr {
        starts_with: Some("device-mapper: dirty region log:"),
        regex: r"(?x)
                ^device-mapper:\s
                dirty\ region\ log:\s
                (?P<kdev>\d+:\d+):\ Failed\ to\ write\ header\ on\ dirty\s
                region\ log\ device$
                ",
        sub_system: "DM-DirtyLog",
        event_type: "DM_DIRTY_LOG_WRITE_FAILED",
    },
    RegexConfStr {
        starts_with: Some("sd "),
        regex: r"(?x)
                ^sd\ \d+:\d+:\d+:\d+:\s
                \[(?P<kdev>sd[a-z]+)\]\s
                Unaligned\ partial\ completion\s
                \(resid=(?P<resid>\d+),\ sector_sz=(?P<sector_sz>\d+)\)$
                ",
        sub_system: "SCSI",
        event_type: "SCSI_UNALIGNED_PARTIAL_COMPLETION",
    },
    RegexConfStr {
        starts_with: Some("sd "),
        regex: r"(?x)
                ^sd\ \d+:\d+:\d+:\d+:\s
                \[(?P<kdev>sd[a-z]+)\]\s
                Spinning\ up\ disk...$
                ",
        sub_system: "SCSI",
        event_type: "SCSI_SPINNING_UP_DISK",
    },
    RegexConfStr {
        starts_with: Some("sd "),
        regex: r"(?x)
                ^sd\ \d+:\d+:\d+:\d+:\s
                \[(?P<kdev>sd[a-z]+)\]\s
                tag\#\d+\ Sense\ Key\ :\ (?P<sense_key>[^\[\]]+)\s
                \[(?P<is_deferred>(?:deferred)|(?:current))\]
                ",
        sub_system: "SCSI",
        event_type: "SCSI_SENSE_KEY",
    },
    RegexConfStr {
        starts_with: Some("sd "),
        regex: r"(?x)
                ^sd\ \d+:\d+:\d+:\d+:\s
                \[(?P<kdev>sd[a-z]+)\]\s
                tag\#\d+\ Add\.\ Sense:\ (?P<asc>.+)$
                ",
        sub_system: "SCSI",
        event_type: "SCSI_ADDITIONAL_SENSE_CODE",
    },
    RegexConfStr {
        starts_with: Some("sd "),
        regex: r"(?x)
                ^sd\ \d+:\d+:\d+:\d+:\s
                \[(?P<kdev>sd[a-z]+)\]\s
                Medium\ access\ timeout\ failure\.\ Offlining\ disk!$
                ",
        sub_system: "SCSI",
        event_type: "SCSI_MEDIUM_ACCESS_TIMEOUT_OFFLINEING_DISK",
    },
    RegexConfStr {
        starts_with: Some("EXT4-fs "),
        regex: r"(?x)
                ^EXT4-fs\s
                \((?P<kdev>[^\s\)]+)\):\s
                mounted\ filesystem\ with(?P<data_mode>.+).\s
                Opts:\ (?P<opts>.+)$
                ",
        sub_system: "ext4",
        event_type: "FS_MOUNTED",
    },
    RegexConfStr {
        starts_with: Some("EXT4-fs "),
        regex: r"(?x)
                ^EXT4-fs\s
                \((?P<kdev>[^\s\)]+)\):\s
                Remounting\ filesystem\ read-only$
                ",
        sub_system: "ext4",
        event_type: "FS_REMOUNT_READ_ONLY",
    },
    RegexConfStr {
        starts_with: Some("EXT4-fs (device "),
        regex: r"(?x)
                ^EXT4-fs\s
                \(device\ (?P<kdev>[^\s\)]+)\):\s
                panic forced after error
                ",
        sub_system: "ext4",
        event_type: "FS_PANIC",
    },
    RegexConfStr {
        starts_with: Some("EXT4-fs error (device "),
        regex: r"(?x)
                ^EXT4-fs\ error\s
                \(device\ (?P<kdev>[^\s\)]+)\):\s
                ",
        sub_system: "ext4",
        event_type: "FS_ERROR",
    },
    RegexConfStr {
        starts_with: Some("XFS "),
        regex: r"(?x)
                ^XFS \s
                \((?P<kdev>[^\s\)]+)\):\s
                Ending\ clean\ mount",
        sub_system: "xfs",
        event_type: "FS_MOUNTED",
    },
    RegexConfStr {
        starts_with: Some("XFS "),
        regex: r"(?x)
                ^XFS\s
                \((?P<kdev>[^\s\)]+)\):\s
                Unmounting\ Filesystem$",
        sub_system: "xfs",
        event_type: "FS_UNMOUNTED",
    },
    RegexConfStr {
        starts_with: Some("XFS "),
        regex: r"(?x)
                ^XFS \s
                \((?P<kdev>[^\s\)]+)\):\s
                writeback\ error\ on\ sector",
        sub_system: "xfs",
        event_type: "FS_IO_ERROR",
    },
    RegexConfStr {
        starts_with: Some("EXT4-fs "),
        regex: r"(?x)
                ^EXT4-fs\s
                warning\ \(device\s
                (?P<kdev>[^\s\)]+)\):\s
                ext4_end_bio:[0-9]+:\ I/O\ error
                ",
        sub_system: "ext4",
        event_type: "FS_IO_ERROR",
    },
    RegexConfStr {
        starts_with: Some("JBD2: "),
        regex: r"(?x)
                ^JBD2:\s
                Detected\ IO\ errors\ while\ flushing\ file\ data\ on\s
                (?P<kdev>[^\s]+)-[0-9]+$
                ",
        sub_system: "jbd2",
        event_type: "FS_IO_ERROR",
    },
];
