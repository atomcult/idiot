# Design

## Context

`share/image.sh` `image_load` already reads a Core image into a private
work directory, dumping ubuntu-data's `state.json` there with debugfs.
`idiot-helper snapd-state` parses it: tab-separated records for
`inspect diff`, and `--report snaps|connections|changes` sections for
`inspect image`, colored from the environment. Neither prints anything
secret today, but `inspect image --extract-state` copies the raw file.
`share/vm.sh` has `vm_resolve`, `vm_running`, `vm_conf_get` and `vm_ssh`;
`cmd/vm`'s `install` and `logs` reach the guest as the Launchpad user
(`require_lp_user`) with `sudo -n` and ssh `BatchMode`. A VM's disk is
`disk.qcow2`, an overlay over the pristine image. fzf is bundled (0.44 in
core24, the host's here is 0.44 too).

No `state.json` was available to sample: the only Core image on disk
(`~/tmp/sc101/pc.img`) has never booted, no idiot VM exists, and this
machine's state is root-only and holds the user's own macaroons, so it
wasn't read. The secret-bearing fields come from snapd's source (a local
checkout at 92cc6565e7, 2026-11-19): `overlord/auth/auth.go` (`AuthState`,
`UserState`, `DeviceState`), `overlord/snapstate/cookies.go`
(`snap-cookies`), `overlord/snapstate/snapmgr.go`
(`refresh-privacy-key`, `cohort-key`, `instance-key`), and the sequence
format from `overlord/snapstate/sequence/sequence.go` (an array of side
infos, each with optional `components`). Nothing was checked live.

## Goals / Non-Goals

**Goals:**
- One place decides what is secret, and every path that shows or writes
  state goes through it.
- The unredacted state touches disk only where it already does today
  (image work directories), and only briefly.
- Views fast enough on a large state (this machine's is ~480 KB).

**Non-Goals:**
- A general JSON browser, or jq-style queries (`--json | jq` covers it).
- Redacting the reports `inspect image` prints today: they show no
  secrets, but they run on redacted input anyway once redaction is on
  load.
- An opt-out. Someone who needs the raw file has root on the device.

## Decisions

**Redaction is in idiot-helper, on load.** sh can't walk JSON, and the
host has no python3 or jq, so the tree walk is Rust, next to the parser
that already exists. Every `snapd-state` mode (records, `--report`, and the
new ones) redacts right after parsing, so no mode, present or future, can
print an unredacted value. It stays an offline parser: no network, no
credentials handled beyond replacing them. Alternatives: redacting in sh
with sed over the raw file (can't see structure: member names vs values,
cookie map keys); redacting only in the explorer (leaves `--report`,
records and future modes to remember it).

**The rules: known paths, member names, value shapes.** The spec lists
them. Known paths cover what snapd stores today. The member-name rule
catches what can't be listed: snap configuration (`data.config`), confdb
databags, a `configure` task's `patch`, and fields snapd adds later. The
value rules catch macaroons and URL passwords stored under innocent names
(a proxy setting, a task log line). Over-redaction is the accepted
failure: `cohort-key` is redacted, which is also correct since it grants a
cohort, and `instance-key` and `hotplug-key`, the two `-key` members that
plainly aren't secrets and that views need, are allowed by name. User
names, emails, serials and key IDs stay: they're what the explorer is for
(who is logged in, which device), and none authenticates anyone. Under a
redacted member the shape survives (array lengths, member counts as
`<redacted-n>`), so "two discharges" is still visible.

**Helper interface.** New `snapd-state` modes, all on a state file:
- `--redact <in|-> <out>`: write the redacted state as indented JSON to
  `<out>` (a temporary file in `<out>`'s directory, created `0600`, then
  renamed), and print `redacted <n>` on stdout. The count can only be made
  from the raw input, so it is reported here and kept by the caller.
- `--list snaps|connections|changes|tree` and `--list tasks <change-id>`,
  `--list tree <pointer>`: one fzf row per item, tab-separated, the first
  column the item's key (snap name, connection key, change or task id, or
  a JSON Pointer) and the rest the colored display columns, padded by
  characters as `--report` does. An empty view prints `(none)` with an
  empty key.
- `--show snap|connection|change|task|path <key>`: the preview text.
Tree rows are keyed by RFC 6901 JSON Pointers, because connection keys
hold spaces and colons and config keys hold dots; the displayed path is
dotted with `["…"]` for keys that need it. Exit status as today (2 for
usage or unreadable input), plus 1 for a key that isn't in the state.
Alternative: one long-running helper serving fzf over a socket; rejected,
each preview is a fresh parse of a few hundred KB, which takes
milliseconds.

**Each source feeds the redaction directly.** The work directory is
`mktemp -d` (mode 0700); after `--redact` it holds only `state.json`, the
redacted copy, and is removed on exit.
- file: `--redact <file>`; `-`: `--redact -` from stdin;
- `--local`: as root `--redact /var/lib/snapd/state.json`, else `sudo cat
  /var/lib/snapd/state.json | idiot-helper snapd-state --redact - …`
  (sudo prompts on the terminal; nothing raw is written);
- running VM: `sudo -n cat` in the guest over `vm_ssh` with `BatchMode`,
  piped into `--redact -`; a password prompt fails as `vm install` does;
- image: `image_load` into a subdirectory, then `--redact`, then remove
  that subdirectory before fzf starts;
- stopped VM, or a qcow2 file: `qemu-img convert -O raw` of the disk
  (overlay and backing image together) into a sparse file in a work
  directory under `IDIOT_CACHE_DIR` (not `/tmp`, often a small tmpfs),
  then as an image, then removed. qemu-img's image lock makes a VM that
  boots during the conversion fail it rather than race it. Alternative:
  extracting only ubuntu-data with qemu-img's raw `offset`/`size` options
  after reading the GPT; faster, but needs the partition table first and
  more qemu-img option handling. Worth doing if conversion proves slow.
Source detection reads the first bytes: `{` after whitespace is JSON,
`QFI\xfb` is qcow2, anything else goes to `image_load`, whose error for a
non-Core file is already right.

**The explorer is a loop in sh, one fzf run per view.** Each run uses
`--expect=alt-s,alt-c,alt-h,alt-t,enter`, `--delimiter='\t'`,
`--with-nth=2..`, `--ansi`, `--no-sort` and a `--preview` calling
`snapd-state --show <view> {1}` on the redacted file (helper and file
paths exported for the preview's shell). The loop keeps the view, the
change whose tasks are shown, and where a tree was opened from: Escape
goes back (tasks to changes, a tree opened from an item to that view) or
exits from a top-level view. Keys are alt-letters because fzf already
binds the obvious ctrl keys (ctrl-c, ctrl-g, ctrl-n…). Alternative: one
fzf with `reload`/`transform` bindings; smoother, but the state machine
then lives in fzf's binding language, `transform` needs a newer fzf than
core24's, and it's harder to test. Colors come from common.sh's
stderr colors (fzf draws on the terminal through stderr), so `NO_COLOR`
already applies.

**Export.** `--json` runs `--redact` into the work directory and cats the
result to stdout; `-o <file>` redacts straight to the file, after the
command has refused an existing one without `--force`. The terminal check
(stdin and stderr) comes before reading any source, so a script that
forgets `--json` fails before sudo or ssh.

**`inspect image --extract-state` uses `--redact`.** It changes what the
file holds (redacted, 0600) but not how it's asked for. Nothing in idiot
reads an extracted state back except `inspect state`, which redacts
anyway, so nothing depends on the raw content.

## Risks / Trade-offs

- [A secret under a name and shape the rules don't know] → The rules are
  deliberately broad; a unit test runs every mode over a fixture with one
  of each known secret and fails if any secret byte string appears in any
  output. New snapd fields get a test case when found.
- [Over-redaction hides something needed for debugging] → The member name
  is kept, so the user sees that it exists; the raw value is on the
  device for whoever has root there.
- [Converting a stopped VM's disk is slow and uses disk space] → Sparse
  output, a stderr message while it runs, the cache directory instead of
  `/tmp`, and the partition-only alternative above if it matters.
- [`sudo cat` on `--local` under the snap] → It's a host command, so it
  runs in the user's shell outside confinement, as `vm` does.
- [A state from a much older or newer snapd] → Views read members
  leniently (missing fields show empty, as `--report` does); the tree view
  shows whatever is there.
- [Changing `--extract-state`'s output] → Called out in the proposal;
  someone relying on the raw file loses it, by design.

## Migration Plan

None: a new command, and `--extract-state` keeps its interface. Rollback
is reverting the commits.

## Open Questions

- Should user email addresses be redacted too? Kept for now as identity,
  not credentials; adding `email` to the name rule later changes nothing
  else.
- Whether to add the partition-only qemu-img read for stopped VMs, once
  the conversion time on a real VM is known (task 4.3 measures it).
- Change 6 (`vm-install-from-store`) and others don't depend on this; if
  a later change wants "the VM's installed revisions" non-interactively,
  `snapd-state` records over the redacted file are the place to add it.
