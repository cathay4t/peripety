<!-- vim-markdown-toc GFM -->

* [Storage Event Notification Daemon](#storage-event-notification-daemon)
    * [Features](#features)
    * [How-to](#how-to)
    * [Event examples](#event-examples)
    * [FAQ](#faq)
        * [What can be done by kernel](#what-can-be-done-by-kernel)
        * [Why another daemon?](#why-another-daemon)
            * [Why not expand udisks for this?](#why-not-expand-udisks-for-this)
            * [Why this tool is better?](#why-this-tool-is-better)

<!-- vim-markdown-toc -->

# Storage Event Notification Daemon

Peripety is designed to parse system storage logging into structured storage
event helping user investigate storage issues.

To do so, it provides three tools:
 * Daemon -- `peripetyd`

   The daemon parses incoming storage logs and saved the structured
   peripety storage event for future query or event notification.

   The daemon has build-in regular expressions for parsing system logs. If
   that does not works on your system. You may define your own regexs in
   `/etc/peripetyd.conf`. Please refer to [sample config][4] which
   contains detail documentation.

 * CLI -- `prpt`

   Command line tool of peripety for event query, event monitor, block
   information query. Please check manpage of prpr for detail documents.

 * Rust binding -- `peripety`

   Developer friendly interface for query, monitor peripety storage events.
   Please check crate document for detail. TODO: Add docs.rs link.

## Features

 * Provides device consistent id, event type and extra information like
   FC/iSCSI path detail and FS mount point.

 * CLI tool `prpt` to query, monitor events and query block information.

 * Non-invasive/non-IO generating for event processing.

 * Events are stored in journald with structured data(JSON).

 * Allows user defined regex in `/etc/peripetyd.conf`.

 * Rust crate `peripety` for query block information on all kind of dev
   string(major:minor, scsi_id, nvme ctrl_id+ns_id, etc).

 * TODO: Varlink(JSON) interface.

 * TODO: Handle user space tool logs like mulitpathd, iscsid.

## How-to

 * Install

```bash
# Please install systemd-devel package.
make
sudo make install
```

 * Start Daemon

```bash
sudo systemctl start peripetyd
```

 * Start monitor CLI

```bash
# You may remove the `sudo` if in `systemd-journal` group.
sudo prpt monitor -J
```

 * Trigger some test events

```
# SCSI sector hardware error
./tests/scsi.sh
# Multipath path failure
./tests/dmmp.sh
# File system I/O error
./tests/fs.sh
# File system over LVM over multipath
./tests/fs_lvm_dmmp.sh
# LVM ThinProvisioning pool full
./tests/lvm_tp.sh
```

 * Query events

```bash
# You may remove the `sudo` if in `systemd-journal` group.
sudo prpt query
```

## Event examples

* [Ext4 mounted on LVM LV over SCSI multipath][2]

* [FC Multipath got path failure][3]

## FAQ

### What can be done by kernel
I have created [some patches][1] hoping kernel could provides in logs:
 * Structured log via /dev/kmsg.
 * WWID of device matters to fix race issue.
 * Event type string to save regex capture.

### Why another daemon?

#### Why not expand udisks for this?

 * Current design of udisks require modules written in C which is not a good
   language for string manipulation which is quit common when parsing eventing.

 * Udisks components are trigger by uevent which only have add/change/del
   event type defined, modules need to extra work to find out what just
   happened. Yes, we can change udisks to support event types, but IMHO, that
   require much more work than creating new storage event daemon in rust.

#### Why this tool is better?

 * Rust is almost-memory-leak-proof and quite easy to handle threading, IPC and
   string manipulation.

 * Only do one thing quick and simple -- provide storage events.

[1]: https://github.com/cathay4t/linux/commits/structured_log
[2]: https://github.com/cathay4t/peripety/blob/master/examples/fs/ext4_mount_lv_mpath_scsi.json
[3]: https://github.com/cathay4t/peripety/blob/master/examples/mpath/mpath_fc_path_offline.json
[4]: https://github.com/cathay4t/peripety/blob/master/etc/peripetyd.conf
