// Copyright (C) 2018 Red Hat, Inc.
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//
// Author: Gris Ge <fge@redhat.com>

extern crate chrono;
#[macro_use]
extern crate clap;
extern crate libc;
extern crate libnotify;
extern crate nix;
extern crate peripety;
extern crate sdjournal;

use chrono::{DateTime, Datelike, Duration, Local, TimeZone};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use nix::sys::select::FdSet;
use peripety::{BlkInfo, LogSeverity, StorageEvent, StorageSubSystem};
use std::ffi::CStr;
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;
use std::process::exit;

macro_rules! to_stdout {
    ($($arg:tt)*) => (
        if writeln!(&mut io::stdout(), $($arg)*).is_err() {
            exit(0);
        });
}

macro_rules! to_stderr {
    ($($arg:tt)*) => (
        if writeln!(&mut io::stderr(), $($arg)*).is_err() {
            exit(0);
        });
}

#[derive(Debug, Clone)]
struct CliOpt {
    severity: Option<LogSeverity>,
    sub_systems: Option<Vec<StorageSubSystem>>,
    event_types: Option<Vec<String>>,
    since: Option<u64>,
    blk_info: Option<BlkInfo>,
    is_json: bool,
    need_notify: bool,
}

arg_enum!{
    #[derive(Debug)]
    enum Severity {
        Emergency,
        Alert,
        Ctritical,
        Error,
        Warning,
        Notice,
        Info,
        Debug
    }
}

fn quit_with_msg(msg: &str) {
    to_stderr!("Error: {}", msg);
    exit(1);
}

fn time_str_to_u64(time_str: &str) -> Option<u64> {
    if let Ok(t) = Local.datetime_from_str(time_str, "%F %H:%M:%S") {
        Some(
            t.timestamp() as u64 * 10u64.pow(6)
                + u64::from(t.timestamp_subsec_micros()),
        )
    } else {
        None
    }
}

fn since_cliopt_to_journald_timestamp(since: &str) -> Option<u64> {
    if since == "today" {
        let dt = Local::now();
        return time_str_to_u64(&format!(
            "{}-{}-{} 00:00:00",
            dt.year(),
            dt.month(),
            dt.day()
        ));
    }

    if since == "yesterday" {
        let dt = Local::now() - Duration::days(1);
        return time_str_to_u64(&format!(
            "{}-{}-{} 00:00:00",
            dt.year(),
            dt.month(),
            dt.day()
        ));
    }

    if since.contains(':') {
        return time_str_to_u64(since);
    }

    time_str_to_u64(&format!("{} 00:00:00", since))
}

fn arg_match_to_cliopt(matches: &ArgMatches) -> CliOpt {
    let mut ret = CliOpt {
        severity: None,
        sub_systems: None,
        event_types: None,
        since: None,
        blk_info: None,
        is_json: false,
        need_notify: false,
    };
    if matches.is_present("severity") {
        match matches.value_of("severity") {
            Some(s) => {
                ret.severity = match s.parse::<LogSeverity>() {
                    Ok(s) => Some(s),
                    Err(e) => {
                        quit_with_msg(&format!("{}", e));
                        None
                    }
                }
            }
            None => quit_with_msg("Invalid severity"),
        }
    }
    if matches.is_present("event-type") {
        match matches.values_of("event-type") {
            Some(ets) => {
                let mut event_types = Vec::new();
                for et in ets {
                    event_types.push(et.to_string());
                }
                ret.event_types = Some(event_types);
            }
            None => quit_with_msg("Invalid event-type"),
        }
    }
    if matches.is_present("sub-system") {
        match matches.values_of("sub-system") {
            Some(subs) => {
                let mut sub_systems = Vec::new();
                for s in subs {
                    match s.parse::<StorageSubSystem>() {
                        Ok(s) => sub_systems.push(s),
                        Err(e) => quit_with_msg(&format!("{}", e)),
                    }
                }
                ret.sub_systems = Some(sub_systems);
            }
            None => quit_with_msg("Invalid sub-system"),
        }
    }
    if matches.is_present("since") {
        match matches.value_of("since") {
            Some(s) => match since_cliopt_to_journald_timestamp(s) {
                Some(t) => {
                    ret.since = Some(t);
                }
                None => quit_with_msg("Invalid --since option"),
            },
            None => quit_with_msg("Invalid since"),
        }
    }

    ret.is_json = matches.is_present("J");
    ret.need_notify = matches.is_present("N");

    if matches.is_present("blk") {
        match matches.value_of("blk") {
            Some(s) => ret.blk_info = get_blk_info(&s),
            None => quit_with_msg("Invalid blk option"),
        };
    }
    ret
}

// TODO(Gris Ge): If performance is a concern and moving search to journal API
//                add_match() could speed things up, we should do it.
//                Need investigation.
fn handle_event(event: &StorageEvent, cli_opt: &CliOpt) {
    let mut is_match = true;

    if let Some(ref l) = cli_opt.severity {
        if l < &event.severity {
            is_match = false;
        }
    }
    if let Some(ref subs) = cli_opt.sub_systems {
        if !subs.is_empty() && !subs.contains(&event.sub_system) {
            is_match = false;
        }
    }

    if let Some(ref ets) = cli_opt.event_types {
        if !ets.is_empty() && !ets.contains(&event.event_type) {
            is_match = false;
        }
    }

    if let Some(ref b) = cli_opt.blk_info {
        is_match = false;
        if event.blk_info.wwid == b.wwid {
            is_match = true;
        } else {
            for owner_info in &b.owners {
                if owner_info.wwid == event.blk_info.wwid {
                    is_match = true;
                    break;
                }
            }
        }
    }

    if is_match {
        let msg = if !event.msg.is_empty() {
            &event.msg
        } else {
            &event.raw_msg
        };
        if cli_opt.is_json {
            to_stdout!(
                "{}\n",
                event
                    .to_json_string_pretty()
                    .expect("BUG: event.to_json_string_pretty()")
            );
        } else {
            let ts = DateTime::parse_from_rfc3339(&event.timestamp)
                .expect("BUG: DateTime::parse_from_rfc3339()")
                .with_timezone(&Local)
                .to_rfc2822();
            to_stdout!(
                "{} {} {} {}",
                ts,
                event.hostname,
                event.sub_system,
                &msg
            );
        }
        if cli_opt.need_notify {
            let n = libnotify::Notification::new(
                &format!(
                    "{:?} {} {}",
                    event.severity, event.sub_system, event.event_type
                ),
                Some(msg.as_str()),
                None,
            );
            if let Err(e) = n.show() {
                to_stderr!("Failed to send notification: {}", e);
            }
        }
    }
    ()
}

fn handle_monitor(cli_opt: &CliOpt) {
    if cli_opt.since.is_some() {
        quit_with_msg("`monitor` sub-command does not allow `--since` option");
    }

    let mut journal =
        sdjournal::Journal::new().expect("Failed to open systemd journal");
    // We never want to block, so set the timeout to 0
    journal.timeout_us = 0;
    // Jump to the end as we cannot annotate old journal entries.
    journal
        .seek_tail()
        .expect("Unable to seek to end of journal!");
    journal
        .add_match("IS_PERIPETY=TRUE")
        .expect("Unable to search peripety journal");

    loop {
        let mut fds = FdSet::new();
        fds.insert(journal.as_raw_fd());
        if let Err(e) =
            nix::sys::select::select(None, Some(&mut fds), None, None, None)
        {
            quit_with_msg(&format!(
                "collector: Failed select against journal fd: {}",
                e
            ));
        }
        if !fds.contains(journal.as_raw_fd()) {
            continue;
        }
        if cli_opt.need_notify {
            libnotify::init("peripety")
                .expect("Failed to initialize libnotify");
        }
        for entry in &mut journal {
            match entry {
                Ok(entry) => {
                    if let Some(j) = entry.get("JSON") {
                        if let Ok(event) = StorageEvent::from_json_string(j) {
                            handle_event(&event, &cli_opt);
                        }
                    }
                }
                Err(e) => quit_with_msg(&format!(
                    "Failed to retrieve the journal entry: {:?}",
                    e
                )),
            }
        }
    }
}

fn handle_query(cli_opt: &CliOpt) {
    let mut journal =
        sdjournal::Journal::new().expect("Failed to open systemd journal");
    // We never want to block, so set the timeout to 0
    journal.timeout_us = 0;
    journal
        .add_match("IS_PERIPETY=TRUE")
        .expect("Unable to search peripety journal");

    if let Some(since) = cli_opt.since {
        journal.seek_realtime_usec(since).unwrap_or_else(|_| {
            panic!("Unable to seek journal after {}", since)
        })
    }
    for entry in &mut journal {
        match entry {
            Ok(entry) => {
                if let Some(j) = entry.get("JSON") {
                    match StorageEvent::from_json_string(j) {
                        Ok(event) => handle_event(&event, &cli_opt),
                        Err(e) => quit_with_msg(&format!("Error: {}", e)),
                    };
                }
            }
            Err(e) => quit_with_msg(&format!(
                "Error retrieving the journal entry: {:?}",
                e
            )),
        }
    }
}

#[allow(unused_must_use)]
fn to_stdout_blk_info(
    i: &BlkInfo,
    is_json: bool,
    is_simple: bool,
    prefix: &str,
) {
    if is_json {
        to_stdout!(
            "{}",
            i.to_json_string_pretty().expect("BUG: handle_info()")
        );
    } else {
        if is_simple {
            if let Some(l) = i.blk_path.rfind('/') {
                if let Some(ref m) = i.mount_point {
                    to_stdout!(
                        "{}{} : {} : {}",
                        prefix,
                        &i.blk_path[l + 1..],
                        i.wwid,
                        m
                    );
                } else {
                    to_stdout!(
                        "{}{} : {}",
                        prefix,
                        &i.blk_path[l + 1..],
                        i.wwid
                    );
                }
            }
        } else {
            to_stdout!("{}blk_path     : {}", prefix, i.blk_path);
            to_stdout!("{}preferred    : {}", prefix, i.preferred_blk_path);
            to_stdout!("{}blk_type     : {}", prefix, i.blk_type);
            to_stdout!("{}wwid         : {}", prefix, i.wwid);
            to_stdout!("{}transport_id : {}", prefix, i.transport_id);
            let uuid = match i.uuid {
                Some(ref u) => u,
                None => "",
            };
            to_stdout!("{}uuid         : {}", prefix, uuid);
            let mp = match i.mount_point {
                Some(ref m) => m,
                None => "",
            };
            to_stdout!("{}mount_point  : {}", prefix, mp);
        }
        if prefix == "" && !i.owners.is_empty() {
            if !is_simple {
                to_stdout!("{}owners       :", prefix);
            }
            for owner_info in &i.owners {
                to_stdout_blk_info(
                    &owner_info,
                    is_json,
                    is_simple,
                    &format!("{}  ", prefix),
                );
                if !is_simple {
                    to_stdout!("");
                }
            }
        }
    }
}

fn is_match(blk: &str, info: &BlkInfo) -> bool {
    if info.wwid == blk {
        return true;
    }
    if let Some(ref m) = info.mount_point {
        if m == blk {
            return true;
        }
    }
    false
}

fn get_blk_info(blk: &str) -> Option<BlkInfo> {
    if blk.starts_with("/dev") {
        match BlkInfo::new(blk) {
            Ok(i) => return Some(i),
            Err(e) => quit_with_msg(&format!("{}", e)),
        };
    } else {
        // We might got wwid or other stuff
        match BlkInfo::list() {
            Ok(infos) => {
                for info in infos {
                    if is_match(blk, &info) {
                        return Some(info);
                    }
                    for owner_info in &info.owners {
                        if is_match(blk, &owner_info) {
                            return Some(info.clone());
                        }
                    }
                }
            }
            Err(e) => quit_with_msg(&format!("{}", e)),
        };
    }
    quit_with_msg("Specified block not found");
    None
}

fn handle_info(blk: &str, is_json: bool) {
    if let Some(i) = get_blk_info(blk) {
        to_stdout_blk_info(&i, is_json, false, "");
    }
}

#[allow(unused_must_use)]
fn handle_list(is_json: bool, is_simple: bool) {
    match BlkInfo::list() {
        Ok(blk_infos) => {
            for blk_info in blk_infos {
                to_stdout_blk_info(&blk_info, is_json, is_simple, "");
                to_stdout!("");
            }
        }
        Err(e) => {
            quit_with_msg(&format!("Failed to list current blocks: {}", e))
        }
    }
}

fn check_permission() {
    let journal_group_name = "systemd-journal";
    if nix::unistd::geteuid() == nix::unistd::Uid::from_raw(0) {
        return;
    }

    // The getgrgid is not thread safe, since we are not using any thread in
    // ptpr, we should be OK.
    for gid in nix::unistd::getgroups().unwrap() {
        unsafe {
            let gp: libc::group = *libc::getgrgid(From::from(gid));
            if journal_group_name
                == CStr::from_ptr(gp.gr_name).to_str().unwrap()
            {
                return;
            }
        }
    }
    quit_with_msg(&format!(
        "Permission check failed: Not root, not in \
         '{}' group. Use '-f' to skip this check.",
        journal_group_name
    ));
}

fn main() {
    let sev_arg = Arg::from_usage(
        "--severity=[SEVERITY] 'Only show event with equal or higher severity'",
    ).possible_values(&Severity::variants())
        .case_insensitive(true)
        .default_value("Debug");

    let evt_arg = Arg::from_usage(
        "--event-type=[EVENT-TYPE]... \
         'Only show event with specific event type, argument could be \
         repeated'",
    );
    let sub_arg = Arg::from_usage(
        "--sub-system=[SUB-SYSTEM]... \
         'Only show event from specific sub-system, argument could be \
         repeated'",
    );
    let blk_arg =
        Arg::from_usage("--blk=[BLOCK] 'Only show event with specific block'");

    let json_arg = Arg::from_usage("-J 'Use json format'");

    let no_permission_arg = Arg::from_usage("-f 'Skip permission check'");

    let matches = App::new("Peripety CLI")
        .version("0.1")
        .author("Gris Ge <fge@redhat.com>")
        .about("CLI utile for peripety events")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("monitor")
                .about("Monitor following up events")
                .arg(&no_permission_arg)
                .arg(&json_arg)
                .arg(&sev_arg)
                .arg(&evt_arg)
                .arg(&sub_arg)
                .arg(&blk_arg)
                .arg(Arg::from_usage("-N 'Notify via dbus'")),
        )
        .subcommand(
            SubCommand::with_name("query")
                .about("Query saved events")
                .arg(&no_permission_arg)
                .arg(&json_arg)
                .arg(&sev_arg)
                .arg(&evt_arg)
                .arg(&sub_arg)
                .arg(&blk_arg)
                .arg(Arg::from_usage(
                    "--since [SINCE] \
                     'Only show event on or newer than the specified \
                     time, supported formats are \"2018-05-21\" or \"today\", \
                     \"yesterday\" or \"2012-10-30 18:17:16\".",
                )),
        )
        .subcommand(
            SubCommand::with_name("info")
                .about("Query block information")
                .arg(Arg::from_usage(
                    "<blk> 'Block to query, could be \'major:minor\', \
                     block name, block path, symbolic link to block, \
                     uuid, wwid, or fs mount point'",
                ))
                .arg(&json_arg),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("List current blocks")
                .arg(Arg::from_usage("-D 'Detailed output'"))
                .arg(&json_arg),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("monitor") {
        if !matches.is_present("f") {
            check_permission();
        }
        let cli_opt = arg_match_to_cliopt(&matches);
        handle_monitor(&cli_opt);
        exit(0);
    }

    if let Some(matches) = matches.subcommand_matches("query") {
        if !matches.is_present("f") {
            check_permission();
        }
        let cli_opt = arg_match_to_cliopt(&matches);
        handle_query(&cli_opt);
        exit(0);
    }

    if let Some(matches) = matches.subcommand_matches("info") {
        let is_json = matches.is_present("J");
        match matches.value_of("blk") {
            Some(s) => handle_info(s, is_json),
            None => quit_with_msg("Invalid 'blk' argument"),
        }
        exit(0);
    }

    if let Some(matches) = matches.subcommand_matches("list") {
        if matches.is_present("J") && matches.is_present("D") {
            quit_with_msg("Argument 'D' conflict with 'J'");
        }
        let is_json = matches.is_present("J");
        let is_simple = !matches.is_present("D");
        handle_list(is_json, is_simple);
        exit(0);
    }
}
