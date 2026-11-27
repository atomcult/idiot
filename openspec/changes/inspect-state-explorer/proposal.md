# Proposal

## Why

snapd's `state.json` is where to look when a Core device misbehaves: which
revisions a snap has kept, how its interfaces are connected, and every
change and task with its log. `idiot inspect image` prints a few fixed
sections of it, and `--extract-state` hands over the raw file, but there
is no way to browse it, and none at all for a running VM or this machine.
The raw file also holds the device's and users' store macaroons, so
copying it around to read it with `jq` spreads credentials.

## What Changes

- **`idiot inspect state [options] <source>`**, an fzf explorer of snapd's
  state. The source is one of:
  - a file: a `state.json`, or an Ubuntu Core disk image (read as
    `inspect image` does), told apart by content; `-` reads `state.json`
    from stdin;
  - `--name <vm>`: an `idiot vm` VM, over ssh while it runs, or from its
    disk overlay while it is stopped;
  - `--local`: this machine's `/var/lib/snapd/state.json` (through `sudo`
    unless already root).
- **Views**, switched with keys shown in the header: snaps (current
  revision, channel, type, the kept revisions in the sequence),
  connections (plug, slot, interface, how they were made), changes (status,
  time, summary; Enter opens its tasks), tasks of a change (status, kind,
  summary, with the task's log in the preview) and the raw tree (every JSON
  path, previewing the value under it). Enter on a snap, connection or task
  opens the tree at it.
- **Mandatory redaction.** Secrets are replaced before anything is shown,
  written or exported: users' and the device's macaroons and discharges,
  snapd's macaroon key, snap cookies, the refresh privacy key, any member
  whose name marks it as secret (password, token, secret, key…), anything
  that looks like a serialised macaroon, and passwords in URLs. Redaction
  lives in idiot-helper and applies to every `snapd-state` output, so no
  mode can print an unredacted value. There is no option to turn it off.
- **Non-interactive export.** `--json` prints the redacted state on stdout
  instead of opening the explorer; `-o <file>` writes it to a file
  readable by the user only. Without a terminal, one of them is required.
- **`inspect image --extract-state` saves the redacted state** too, so no
  idiot command writes snapd's secrets to a file.

Output: the explorer writes nothing to stdout. `--json` is new stdout data
(the redacted state as JSON). `inspect image --extract-state` writes the
redacted state where it wrote the raw one. Nothing is queued in the eval
file. It is a host command (user files, ssh, sudo, disk tools).

Dependencies: none new. fzf (bundled), idiot-helper, and per source the
tools that already exist for it: sfdisk, debugfs and mtools for an image
(host or `core` component), ssh for a running VM, qemu-img for a stopped
one (host or `vm` component), sudo for `--local`. The snap stages nothing
new.

Out of scope: editing state, comparing two states (`inspect diff --state`
covers installed snaps and connections), reading state from encrypted
images, and devices reached other than through `idiot vm` (pipe their
state in with `-`).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `inspect`: adds exploring snapd's state (sources, views, redaction,
  export), and `--extract-state` saving the redacted state.

## Impact

- `cmd/inspect/state` (new), `cmd/inspect/help`, `cmd/inspect/image`
  (`--extract-state` through the redaction).
- `share/image.sh`: unchanged interface; the explorer uses `image_load`.
  `share/vm.sh` (`vm_resolve`, `vm_running`, `vm_conf_get`, `vm_ssh`) and
  `share/lp.sh` for the VM source.
- `tools/idiot-helper/src/snapd_state.rs`: redaction on load, and new
  `--redact`, `--list <view>` and `--show <view> <key>` modes.
- Tests: Rust unit tests for redaction and the views;
  `tests/bare-host.d/inspect` (state file, stdin, booted image,
  `--extract-state`, `--local`), `tests/bare-host.d/vm` (stopped VM with
  `--boot`).
- README (Inspecting images and assertions), the using-idiot skill
  reference, and the inspect spec.
