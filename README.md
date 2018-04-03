<!-- vim-markdown-toc GFM -->

* [Storage Event Notification Daemon](#storage-event-notification-daemon)
    * [Why another daemon?](#why-another-daemon)
        * [Why not expand udisks for this?](#why-not-expand-udisks-for-this)
        * [Why this tool is better?](#why-this-tool-is-better)
    * [How-to](#how-to)
    * [Event format](#event-format)
    * [Plugin types](#plugin-types)
    * [Workflow](#workflow)
    * [Plugin conf](#plugin-conf)

<!-- vim-markdown-toc -->

# Storage Event Notification Daemon

## Why another daemon?

### Why not expand udisks for this?

 * Current design of udisks require modules written in C which is not a good
   language for string manipulation which is quit common when parsing eventing.

 * Udisks components are trigger by uevent which only have add/change/del
   event type defined, modules need to extra work to find out what just
   happened. Yes, we can change udisks to support event types, but IMHO, that
   require much more work than creating new storage event daemon in rust.

### Why this tool is better?

 * Rust is memory-leak-proof and quite easy to handle threading, IPC and string
   manipulation.

 * Only do one thing quick and simple -- provide storage events.

## How-to

```bash
make
make run

# Then open another terminal to check on daemon's notification.
make run_stdout

# Then open another terminal to generate some storage errors
make test
```

## Event format
```json
{
    "hostname":             "gris-laptop.example.com",
    "severity":             "info|warn|error",
    "system":               "scsi|lvm|multipath|block|fs|mdraid",
    "timestamp":            1522130579,
    "event_id":             "uuid_of_event",
    "event_type":           "string_like DM_MPATH_PATH_DOWN",
    "dev_wwid":             "wwid_of_device_related",
    "dev_name":             "device_name",
    "msg":                  "human_readable_message",
    "extentions":           {
        "plugin_specifc_1":     "value_1",
        "plugin_specifc_2":     "value_2",
        "plugin_specifc_3":     "value_3"
    }
}
```

## Plugin types
* **Collector**

  Collects raw events.
  For a raw event, `dev_id` might be missing but `dev_name` might not
  be human friendly (for example, a SCSI disk event might have `dev_name`
  "4:0:0:1").

  Examples: `udev` and `kmsg` plugins.

* **Parser**

  Parses both raw and synthetic events then generates synthetic events for
  receivers.
  For the generated synthetic events the parser must provide a valid and
  consistent `dev_id` and human-friendly `dev_name` value.
  Restricts the events it parses to an appropriate subset using a filter.

  Examples: `multipath`, `scsi`, `block`, `fs`, and `mdraid` plugins.

* **Receiver**

  Listens to all events, and generates appropriate actions.

  Examples: `journald`, `email`, `irc`, etc.

  TODO: Create a receiver plugin to cache data.

## Workflow

![work flow](./peripety_design.png)

0. The daemon starts all plugins and establishes socket connections to each
parser and collector plugin.

1. The kernel generates an event in /dev/kmsg.
2. The `kmsg` collector plugin gathers the event and sends the raw event to the daemon.
3. The daemon sends the event to selected parser plugins based on their filter settings.
4. The selected parser plugins process the event and each sends a synthetic event back to the daemon.
5. The daemon broadcasts all synthetic events to receiver plugins via an IP multicast socket.

## Plugin conf

```toml
[main]
receiver_bind_ip = "127.0.0.1"
receiver_multicast_ip = "239.0.0.1"
receiver_multicast_port = "6000"

[kmsg]
type = "collector"

[mpath]
type = "parser"
filer_system = "dm"     # Only send dm event to this plugin.
filer_type = "raw"      # Only send raw event to this plugin.

[jounal]
type = "receiver"
```
