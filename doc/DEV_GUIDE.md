<!-- vim-markdown-toc GFM -->

* [Thread types](#thread-types)
* [Workflow](#workflow)

<!-- vim-markdown-toc -->

## Thread types
* **Collector**

  Collects raw events from journald.
  For a raw event, `dev_wwid` might be missing and `dev_name` might not
  be human friendly (for example, a SCSI disk event might have `dev_name`
  "4:0:0:1").
  TODO: Allows use to extend regex used for parsing journals.

* **Parser**

  Parses both raw and synthetic events then generates synthetic events for
  collectors.
  For the generated synthetic events the parser must provide a valid and
  consistent `dev_wwid` and human-friendly `dev_name` value.
  Restricts the events it parses to an appropriate subset using a filter.

  Examples: `mpath`, `scsi`, `fs`, and `dm` thread.

* **Notifier**

  Listens to all events, and generates appropriate actions.

  Examples: `stdout`, `journald`, `email`, `irc`, etc.

## Workflow

![work flow](../peripety_design.png)

0. The daemon starts all threads.
1. The `collector` thread collects an event from journald.
2. The `collector` thread parse the event and sends the raw event to the daemon.
3. The daemon sends the event to selected parser threads based on their filter
   settings.
4. The selected parser plugins process the event and each sends a synthetic
   event back to the daemon.
5. The daemon broadcasts all synthetic events to notifier threads.
