<!-- vim-markdown-toc GFM -->

* [Storage Event Notification Daemon](#storage-event-notification-daemon)
    * [Event format](#event-format)
    * [Plugin types](#plugin-types)
    * [Workflow](#workflow)
    * [Plugin conf](#plugin-conf)

<!-- vim-markdown-toc -->

# Storage Event Notification Daemon

## Event format
```json
{
    "hostname":             "gris-laptop.example.com",
    "severity":             "info|warn|error",
    "system":               "scsi|lvm|multipath|block|fs|mdraid",
    "event_id":             "uuid_of_event",
    "event_type":           "string_like DM_MPATH_PATH_DOWN",
    "dev_wwid":             "wwid_of_device_related",
    "dev_name":             "device_name",
    "msg":                  "human_readable_message",
    "plugin_specifc_1":     "value_1",
    "plugin_specifc_2":     "value_2",
    "plugin_specifc_3":     "value_3"
}
```

## Plugin types
* **Sender**

  Generate raw or synthetic events.
  For raw event, the `dev_id` is missing, but `dev_name` is mandatory for
  Parser to do look up.
  For synthetic event, the `dev_id` is valid and consistent.
  Example would be `udev` and `kmsg` plugins.

* **Parser**

  Parser both raw and synthetic events, then generate synthetic events to
  receivers. Allow filter on incoming events.
  Example plugins would be `dm`, `scsi`, `block`, `fs`, `mdraid`.

* **Receiver**

  Pluing is listening on all events, and make actions against events. Does
  not allow filter on incoming events.
  Example send events to journald/email/irc/websocket/etc.
  TODO: Create a receiver plugin to cache data.

## Workflow

* Daemon start all plugins and establish socket connections to each sender
  and parser.
* Daemon and receiver listen on IP multicast socket.
* Sender plugins gather events and send to daemon.
* Daemon send events to plugins base on their filter settings.
* Parser plugins do the heavy work and generate synthetic event if needed.
* Daemon send all synthetic events to receiver plugins via IP multicast socket.

## Plugin conf

```toml
[main]
receiver_multicast_ip = "127.0.0.1"

[kmsg]
type = "sender"

[mpath]
type = "parser"
filer_type = "dm"

[jounal]
type = "receiver"
```
