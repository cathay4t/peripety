// prpt monitor --event-type <> --sub-system <> --wwid <> --level <>
// prpt query --event-type <> --sub-system <> --wwid <> --level <>
// prpt blkinfo /dev/sda
//
extern crate chrono;
#[macro_use]
extern crate clap;
extern crate nix;
extern crate peripety;
extern crate sdjournal;

use chrono::{DateTime, Local, TimeZone};
use clap::{App, Arg, ArgMatches, SubCommand};
use nix::sys::select::FdSet;
use peripety::{LogSeverity, StorageEvent, StorageSubSystem};
use std::os::unix::io::AsRawFd;
use std::process::exit;

#[derive(Debug, Clone)]
struct CliOpt {
    severity: Option<LogSeverity>,
    sub_systems: Option<Vec<StorageSubSystem>>,
    event_types: Option<Vec<String>>,
    since: Option<u64>,
    fmt_type: OutputFormat,
}

arg_enum!{
    #[derive(Debug, Clone)]
    enum OutputFormat {
        Json,
        JsonPretty,
        Basic
    }
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
    println!("{}", msg);
    exit(1);
}

fn arg_match_to_cliopt(matches: &ArgMatches) -> CliOpt {
    let mut ret = CliOpt {
        severity: None,
        sub_systems: None,
        event_types: None,
        since: None,
        fmt_type: OutputFormat::Basic,
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
            Some(s) => match Local
                .datetime_from_str(&format!("{} 00:00:00", s), "%F %H:%M:%S")
            {
                Ok(t) => {
                    let timestamp = (t.timestamp() as u64) * 10u64.pow(6)
                        + t.timestamp_subsec_micros() as u64;
                    ret.since = Some(timestamp);
                }
                Err(e) => {
                    quit_with_msg(&format!("Failed to parse --since: {}", e))
                }
            },
            None => quit_with_msg("Invalid since"),
        }
    }
    ret.fmt_type = value_t!(matches.value_of("format"), OutputFormat)
        .unwrap_or_else(|e| e.exit());
    return ret;
}

fn handle_event(event: &StorageEvent, cli_opt: &CliOpt) {
    let mut is_match = true;

    if let Some(l) = &cli_opt.severity {
        if l < &event.severity {
            is_match = false;
        }
    }
    if let Some(subs) = &cli_opt.sub_systems {
        if subs.len() != 0 && !subs.contains(&event.sub_system) {
            is_match = false;
        }
    }

    if let Some(ets) = &cli_opt.event_types {
        if ets.len() != 0 && !ets.contains(&event.event_type) {
            is_match = false;
        }
    }

    if is_match {
        match cli_opt.fmt_type {
            OutputFormat::Basic => {
                let ts = DateTime::parse_from_rfc3339(&event.timestamp)
                    .expect("BUG: DateTime::parse_from_rfc3339()")
                    .with_timezone(&Local)
                    .to_rfc2822();
                println!(
                    "{} {} {} {}",
                    ts, event.hostname, event.sub_system, event.msg
                )
            }
            OutputFormat::Json => {
                println!(
                    "{}",
                    event
                        .to_json_string()
                        .expect("BUG: event.to_json_string()")
                );
            }
            OutputFormat::JsonPretty => {
                println!(
                    "{}",
                    event
                        .to_json_string_pretty()
                        .expect("BUG: event.to_json_string_pretty()")
                );
            }
        }
    }
}

fn handle_monitor(cli_opt: &CliOpt) {
    if let Some(_) = &cli_opt.since {
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
            println!(
                "collector: Failed select against journal fd: {}",
                e
            );
            continue;
        }
        if !fds.contains(journal.as_raw_fd()) {
            continue;
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
                Err(e) => {
                    println!("Error retrieving the journal entry: {:?}", e)
                }
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
        journal
            .seek_realtime_usec(since)
            .expect(&format!(
                "Unable to seek journal after {}",
                since
            ));
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
            Err(e) => println!("Error retrieving the journal entry: {:?}", e),
        }
    }
}

fn main() {
    let sev_arg = Arg::from_usage(
        "--severity=[SEVERITY] 'Only show event with equal or higher severity'",
    ).possible_values(&Severity::variants())
        .default_value("Debug");

    let evt_arg = Arg::from_usage(
        "--event-type=[EVENT-TYPE]... \
         'Only show event with specific event type, argument could be \
         repeated'",
    );
    let sub_arg = Arg::from_usage(
        "--sub-system=[SUB-SYSTEM]... \
         'Only show event with specific sub-system, argument could be \
         repeated'",
    );

    let fmt_arg = Arg::from_usage("--format [FORMAT] 'Event output format'")
        .possible_values(&OutputFormat::variants())
        .default_value("Basic");

    let matches = App::new("Peripety CLI")
        .version("0.1")
        .author("Gris Ge <fge@redhat.com>")
        .about("CLI utile for peripety events")
        .subcommand(
            SubCommand::with_name("monitor")
                .about("Monitor following up events")
                .arg(&fmt_arg)
                .arg(&sev_arg)
                .arg(&evt_arg)
                .arg(&sub_arg),
        )
        .subcommand(
            SubCommand::with_name("query")
                .about("Query saved events")
                .arg(&fmt_arg)
                .arg(&sev_arg)
                .arg(&evt_arg)
                .arg(&sub_arg)
                .arg(Arg::from_usage(
                    "--since [SINCE] \
                     'Only show event on or newer than the specified \
                     date, format is ISO 8601: 2018-05-21'",
                )),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("monitor") {
        let cli_opt = arg_match_to_cliopt(&matches);
        handle_monitor(&cli_opt);
        exit(0);
    }

    if let Some(matches) = matches.subcommand_matches("query") {
        let cli_opt = arg_match_to_cliopt(&matches);
        handle_query(&cli_opt);
        exit(0);
    }
}
