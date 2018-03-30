<!-- vim-markdown-toc GFM -->

* [Storage Event Notification Daemon](#storage-event-notification-daemon)
    * [How-to](#how-to)
    * [Event format](#event-format)
    * [Plugin types](#plugin-types)
    * [Workflow](#workflow)
    * [Plugin conf](#plugin-conf)

<!-- vim-markdown-toc -->

# Storage Event Notification Daemon

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

  Collect raw event.
  For raw event, the `dev_id` might be missing, but `dev_name` might be not
  be human friendly(for example, SCSI disk event might have "4:0:0:1").
  All events will send to parser to generate synthetic event.
  Example would be `udev` and `kmsg` plugins.

* **Parser**

  Parser both raw and synthetic events, then generate synthetic events to
  receivers.
  For synthetic event, the `dev_id` is valid and consistent, `dev_name`
  is human friendly.
  Allow filtering on incoming events.
  Example plugins would be `multipath`, `scsi`, `block`, `fs`, `mdraid`.

* **Receiver**

  Pluing is listening on all events, and make actions against events. Does
  not allow filter on incoming events.
  Example send events to journald/email/irc/websocket/etc.
  TODO: Create a receiver plugin to cache data.

## Workflow

* Daemon start all plugins and establish socket connections to each sender
  and parser.
* Daemon and receiver listen on IP multicast socket.
* Collector plugins gather events and send to daemon.
* Daemon send events to plugins base on their filter settings.
* Parser plugins do the heavy work and generate synthetic event if needed.
* Daemon send all synthetic events to receiver plugins via IP multicast socket.

## Plugin conf

```toml
[main]
receiver_multicast_ip = "127.0.0.1"

[kmsg]
type = "collector"

[mpath]
type = "parser"
filer_system = "dm"     # Only send dm event to this plugin.
filer_type = "raw"      # Only send raw event to this plugin.

[jounal]
type = "receiver"
```
