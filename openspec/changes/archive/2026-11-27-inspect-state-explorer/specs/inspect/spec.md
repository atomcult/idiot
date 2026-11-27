## ADDED Requirements

### Requirement: Reading snapd's state
`idiot inspect state [options] <file|->`, `idiot inspect state [options]
--name <vm>` and `idiot inspect state [options] --local` SHALL read snapd's
`state.json` from exactly one source:
- a file: when its content starts (after whitespace) with `{`, a
  `state.json`; when it is a qcow2 disk, an Ubuntu Core disk read through
  qemu-img without changing it; otherwise an Ubuntu Core disk image, read
  as `idiot inspect image` reads one; `-` SHALL read a `state.json` from
  stdin;
- `--name <vm>`: the `idiot vm` VM of that name; while it runs, the state
  is read in the guest over its recorded ssh port as the Launchpad user
  with `sudo` that never prompts, and while it is stopped, from its disk
  overlay without booting it;
- `--local`: this machine's `/var/lib/snapd/state.json`, read directly
  when the user is root and through `sudo` otherwise.
Giving no source or more than one SHALL fail with the usage message.
Options SHALL come before a file. It SHALL fail with `file not found` for a
missing file, with `no VM named` (naming `idiot vm list`) for an unknown
VM, saying the input isn't snapd state for JSON that has no `data` object,
saying the image has no snapd state yet for an image or stopped VM that
hasn't completed its first boot, and saying ubuntu-data is encrypted for
one whose ubuntu-data is a LUKS volume. A running VM whose `sudo` asks for
a password SHALL fail saying so instead of prompting. It SHALL need
idiot-helper, and only the tools its source needs: sfdisk, debugfs and
mtools for an image, ssh for a running VM, qemu-img for a stopped VM or a
qcow2 file (each
from the host or a component), and sudo for `--local` as a non-root user.
It SHALL NOT need python3 or jq.

#### Scenario: A saved state file
- **WHEN** the user runs `idiot inspect state st.json` on a file saved with `idiot inspect image --extract-state st.json booted.img`
- **THEN** the explorer opens on that state

#### Scenario: A booted image
- **WHEN** the user runs `idiot inspect state booted.img` on an image that has completed its first boot without encryption
- **THEN** the explorer opens on the image's snapd state

#### Scenario: A fresh image
- **WHEN** the user runs `idiot inspect state fresh.img` on an image that has never booted
- **THEN** the command fails saying the image has no snapd state yet

#### Scenario: A running VM
- **WHEN** the VM `a` is running and its Launchpad user has passwordless sudo, and the user runs `idiot inspect state --name a`
- **THEN** the explorer opens on the guest's current snapd state, and nothing is left in the guest

#### Scenario: A stopped VM
- **WHEN** the VM `a` has completed its first boot and is stopped, and the user runs `idiot inspect state --name a`
- **THEN** the explorer opens on the state the guest last wrote, and the VM's disk is unchanged

#### Scenario: This machine
- **WHEN** the user runs `idiot inspect state --local` on a classic host
- **THEN** sudo may ask for the user's password, and the explorer opens on this machine's snapd state

#### Scenario: From another device
- **WHEN** the user runs `ssh dev sudo cat /var/lib/snapd/state.json | idiot inspect state -`
- **THEN** the explorer opens on the device's state

#### Scenario: Two sources
- **WHEN** the user runs `idiot inspect state --local st.json`
- **THEN** the command fails with `usage: idiot inspect state [options] <file|-|--name <vm>|--local>`

### Requirement: Exploring snapd's state
On a terminal, `idiot inspect state` SHALL open an fzf explorer of the
state with these views, its header naming the view, the source, how many
values were redacted, and the keys that switch views:
- snaps: each snap's name, current revision, tracked channel and type,
  previewing whether it is active, its flags (classic, devmode, jailmode),
  every revision in its sequence with the current one marked, and the
  snap's state;
- connections: each connection's plug and slot as `snap:name`, its
  interface, and whether it was made automatically, by the gadget, by
  hotplug or is undesired, previewing the connection's state;
- changes: each change's id, status, time (ready, else spawn) and summary,
  in id order, previewing its tasks with their status and the `ERROR`
  lines of their logs; Enter SHALL open the tasks view for that change;
- tasks of a change: each task's id, status, kind and summary, previewing
  its spawn and ready times, the tasks it waits for, the names of its data
  members and its whole log, `ERROR` lines highlighted; leaving the view
  SHALL return to the changes;
- tree: every path in the state, objects and arrays included, shown as
  dotted keys and `[n]` indexes, previewing the value under it as indented
  JSON.
It SHALL open on the snaps view, or the one given with `--view
snaps|connections|changes|tree`. Enter on a snap, connection or task SHALL
open the tree at that item. Closing the explorer from a top-level view
SHALL exit zero, print nothing on stdout and leave nothing behind. A state
with nothing in a view SHALL show that view empty, saying `(none)`, not
fail. Every view and preview SHALL show only the redacted state. Colors
SHALL follow `NO_COLOR`.

#### Scenario: Kept revisions
- **WHEN** the snap `pc-kernel` has revisions 1801, 1855 and 1900 in its sequence with 1900 current, and the user highlights it in the snaps view
- **THEN** the preview lists the three revisions with 1900 marked current

#### Scenario: A failed change
- **WHEN** change 7 is in the Error state because a task's hook failed, and the user presses Enter on it in the changes view
- **THEN** the tasks view lists change 7's tasks, and highlighting the failed one shows its log with the `ERROR` line highlighted

#### Scenario: Back from tasks
- **WHEN** the user is in the tasks view of change 7 and presses Escape
- **THEN** the changes view is shown again, and a second Escape exits zero

#### Scenario: Raw tree
- **WHEN** the user switches to the tree view and types `snaps.pc.sequence`
- **THEN** the paths under `data.snaps.pc.sequence` are listed, each previewing its value as indented JSON

#### Scenario: A preseeded image
- **WHEN** the state holds snaps but no changes and the user switches to the changes view
- **THEN** the view shows `(none)` and the explorer stays open

#### Scenario: No terminal
- **WHEN** the user runs `idiot inspect state st.json | cat` without `--json` or `-o`
- **THEN** the command fails saying to pass `--json` or `-o <file>`, before reading the source

### Requirement: Redacting snapd's secrets
`idiot inspect state` SHALL replace every secret in the state before
anything from it is shown, written to a file or printed, and SHALL offer
no way to turn that off. A secret is:
- under `data.auth`: each user's `macaroon`, `discharges`,
  `store-macaroon` and `store-discharges`, the device's
  `session-macaroon`, and `macaroon-key`;
- `data.snap-cookies` (whose member names are the cookies) and
  `data.refresh-privacy-key`;
- anywhere in the state, the value of a member whose name, ignoring case,
  contains `macaroon`, `discharge`, `password`, `passphrase`, `secret`,
  `token`, `cookie`, `credential` or `private`, or is `key` or ends in
  `-key` or `_key`, except `instance-key` and `hotplug-key` (which name a
  snap instance and a device);
- anywhere in the state, a string that is a base64-encoded serialised
  macaroon (it starts `MDAx` and is at least 64 characters long), and the
  user and password of a URL (`scheme://user:pass@host`), which SHALL
  become `scheme://<redacted>@host`.
A redacted string SHALL become `<redacted>`. Under a redacted member,
every string, number and boolean SHALL become `<redacted>`, an array SHALL
keep its length, and an object's member names SHALL become `<redacted-1>`,
`<redacted-2>`… in order. Everything else, including user names, email
addresses, the device's brand, model and serial, and key IDs, SHALL be
kept. The command SHALL never write the unredacted state to a file, except
an image's ubuntu-data copy in a temporary directory readable by the user
only, removed before the explorer opens. It SHALL say on stderr how many
values were redacted.

#### Scenario: A logged-in device
- **WHEN** a Core device's state has a user who ran `snap login`, with a macaroon and two discharges, and a device session macaroon, and the user runs `idiot inspect state --json st.json`
- **THEN** stdout holds the user's email and the device's serial, the macaroons read `<redacted>`, `discharges` is an array of two `<redacted>`, and no part of any macaroon or discharge appears

#### Scenario: Snap cookies
- **WHEN** the state's `snap-cookies` maps two cookies to `pc` and `snapd`
- **THEN** the tree view shows `data.snap-cookies.<redacted-1>` and `data.snap-cookies.<redacted-2>`, and neither cookie appears anywhere in the explorer

#### Scenario: A secret in snap configuration
- **WHEN** `snap set my-app db.password=hunter2` was run on the device and the user opens the tree at `data.config.my-app`
- **THEN** `db.password` reads `<redacted>` and the snap's other options are shown as set

#### Scenario: Proxy credentials
- **WHEN** core's `proxy.http` is `http://bob:hunter2@proxy:3128`
- **THEN** it is shown as `http://<redacted>@proxy:3128`

#### Scenario: Count on stderr
- **WHEN** the user runs `idiot inspect state --json st.json >out.json` on a state with seven secrets
- **THEN** stderr says 7 values were redacted, and `out.json` is valid JSON

### Requirement: Exporting the redacted state
With `--json`, `idiot inspect state` SHALL print the redacted state as
indented JSON on stdout instead of opening the explorer. With `-o <file>`
it SHALL write the redacted state to `<file>`, readable and writable by
the user only, refusing to overwrite an existing file unless `--force` is
given, and say where it was saved. Either SHALL work without a terminal.
`--json` and `-o` together, or either with `--view`, SHALL fail with the
usage message. Messages SHALL go to stderr.

#### Scenario: Piped to jq
- **WHEN** the user runs `idiot inspect state --name a --json | jq '.data.snaps | keys'`
- **THEN** jq prints the VM's installed snap names, and stdout carried no messages or color codes

#### Scenario: Saved to a file
- **WHEN** the user runs `idiot inspect state -o redacted.json --local`
- **THEN** `redacted.json` holds the redacted state with mode `0600`, and stderr names it

#### Scenario: File exists
- **WHEN** `redacted.json` exists and the user runs `idiot inspect state -o redacted.json st.json`
- **THEN** the command fails naming the file and `--force`, and the file is unchanged

## MODIFIED Requirements

### Requirement: Extracting snapd's state
With `--extract-state [<file>]` (or `--extract-state=<file>`), given
before the image, `idiot inspect image` SHALL also save the image's snapd
`state.json`, redacted as `idiot inspect state` redacts it, to `<file>`,
or to `./state.json` when no file is given, readable and writable by the
user only, and say where it was saved and how many values were redacted.
The argument after `--extract-state` SHALL be taken
as the file only when another argument follows it. `--extract-state=` with
no file SHALL fail before inspecting. When the image has no readable snapd
state (never booted, or its ubuntu-data encrypted), the command SHALL print
the summary and then fail saying no state could be saved. Options SHALL
come before the image; an option after it SHALL fail with the usage
message.

#### Scenario: Save the state
- **WHEN** the user runs `idiot inspect image --extract-state st.json booted.img` on a booted image
- **THEN** `st.json` holds the image's snapd state as valid JSON with its secrets redacted, with mode `0600`, and the command exits zero

#### Scenario: Option after the image
- **WHEN** the user runs `idiot inspect image booted.img --extract-state`
- **THEN** the command fails with `usage: idiot inspect image [options] <disk.img>`

#### Scenario: Default file
- **WHEN** the user runs `idiot inspect image --extract-state booted.img` on a booted image
- **THEN** the redacted state is saved to `./state.json`

#### Scenario: Empty file name
- **WHEN** the user runs `idiot inspect image --extract-state= booted.img`
- **THEN** the command fails saying `--extract-state=` needs a file name, before inspecting

#### Scenario: No state in the image
- **WHEN** the user runs `idiot inspect image --extract-state st.json fresh.img` on an image that has never booted
- **THEN** the summary is printed, then the command fails saying the image has no snapd state to save, and `st.json` isn't created

#### Scenario: Secrets left behind
- **WHEN** the image's snapd state holds a device session macaroon and the user runs `idiot inspect image --extract-state st.json booted.img`
- **THEN** `st.json`'s `data.auth.device.session-macaroon` is `<redacted>`
