# inspect Specification

## Purpose

Looking inside the things Ubuntu Core development produces without booting
or installing them: a Core disk image (its model, seed and, once booted,
snapd's state), a snap file's metadata, and this machine's snap changes.

## Requirements

### Requirement: Image summary
`idiot inspect image <disk.img>` SHALL locate the image's `ubuntu-seed`
partition from its GPT and print, on stderr:
- the model as `<brand-id>/<model>`, from the model assertion of the first
  recovery system in the seed, or `unknown` if there is none;
- whether the image is pre-seeded (`yes` when snapd's state says so);
- the first-boot state: `complete` when ubuntu-data's modeenv says `run`,
  `not yet complete` (with the mode, when one is recorded) otherwise, and
  `encrypted` when ubuntu-data is a LUKS volume;
- every snap in the seed with its revision and the model's default channel
  for it, and every optional snap the model lists but the seed lacks,
  marked `not seeded`.
It SHALL need sfdisk, debugfs and mtools, from the host or the `core`
component, and SHALL NOT need python3. An image without an `ubuntu-seed` partition SHALL fail
saying it may not be an Ubuntu Core image; a missing file SHALL fail with
`image not found`.

#### Scenario: Fresh image
- **WHEN** the user runs `idiot inspect image ubuntu-core-24-amd64.img` on an image that has never booted
- **THEN** it prints the model `canonical/ubuntu-core-24-amd64`, `Pre-seeded: no`, `First boot: not yet complete`, and the seeded snaps (pc-kernel, snapd, console-conf, core24, pc) with revisions and channels

#### Scenario: Not a Core image
- **WHEN** the user runs `idiot inspect image` on a file with no `ubuntu-seed` partition
- **THEN** the command fails saying the ubuntu-seed partition was not found

#### Scenario: Optional snap not seeded
- **WHEN** the model lists an optional snap that the seed doesn't contain
- **THEN** that snap is listed as `not seeded` with the model's channel

#### Scenario: Host without Python
- **WHEN** the host has no python3 (an Ubuntu Core 26 device with `idiot+core` installed) and the user runs `idiot inspect image ubuntu-core-24-amd64.img`
- **THEN** the same summary is printed as on a host with python3

### Requirement: Booted image details
When the image's ubuntu-data is readable (not encrypted) and holds snapd's
`state.json`, `idiot inspect image` SHALL also print, on stderr, the
installed snaps (revision and tracked channel), the interface connections
(plug and slot), and the changes (id, status, time and summary), with the
error lines of any change in the Error state under it. An image whose
ubuntu-data is encrypted SHALL get the summary only.

#### Scenario: After first boot
- **WHEN** the image has completed its first boot without full-disk encryption and the user runs `idiot inspect image` on it
- **THEN** `First boot: complete` is printed, followed by the installed snaps, the connections (`(none)` if there are none) and the changes, such as `Initialize system state`

#### Scenario: Encrypted data
- **WHEN** the image's ubuntu-data partition is a LUKS volume
- **THEN** `First boot: encrypted` is printed and no installed snaps, connections or changes

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

### Requirement: Snap metadata
`idiot inspect snap <snap-file>` SHALL print the snap's `snap/snapcraft.yaml`
if the snap contains one and its `meta/snap.yaml` otherwise, on stderr,
highlighted as YAML when `bat` is available. A file that contains neither
SHALL fail naming both paths; a missing file SHALL fail with `file not
found`. It SHALL need unsquashfs.

#### Scenario: Snap without its snapcraft.yaml
- **WHEN** the user runs `idiot inspect snap idiot_v0.1.0_amd64.snap` (which carries only meta/snap.yaml)
- **THEN** the snap's `meta/snap.yaml` is printed, starting `name: idiot`

#### Scenario: Not a snap
- **WHEN** the user runs `idiot inspect snap README.md`
- **THEN** the command fails saying neither snap/snapcraft.yaml nor meta/snap.yaml was found

### Requirement: Browsing changes
`idiot inspect changes` SHALL show this machine's `snap changes` in an fzf
list, previewing `snap change <id>` for the highlighted change, and SHALL
print nothing and exit zero when the list is closed. It SHALL need fzf and
snap.

#### Scenario: Browse and close
- **WHEN** the user runs `idiot inspect changes`, moves through the list and presses Escape
- **THEN** each highlighted change's tasks were shown in the preview, and the command exits zero printing nothing

### Requirement: Comparing two images
`idiot inspect diff [--state] [--tsv] <a.img> <b.img>` SHALL read two
Ubuntu Core disk images the way `idiot inspect image` does and print, on
stdout, only what differs between them:
- the model assertion's `authority-id`, `brand-id`, `model`,
  `display-name`, `architecture`, `base`, `grade`, `store`, `revision`,
  `timestamp`, `classic` and `sign-key-sha3-384` headers, and the gadget
  and kernel snaps the model names (from its `snaps` list, or its
  `gadget`/`kernel` headers for a pre-UC20 model); a header one model has
  and the other lacks SHALL count as a difference;
- the first recovery system's label, whether the image is pre-seeded, and
  its first-boot state (`complete`, `not yet complete` with the mode, or
  `encrypted`);
- every seeded snap present in only one image, and for a snap in both,
  its revision and the model's default channel for it;
- every GPT partition, matched by name, present in only one image, and for
  a partition in both, its start, size and type;
- with `--state`, when both images hold readable snapd state: every
  installed snap present in only one, the revision and tracked channel of
  one in both, and every interface connection present in only one.
Options SHALL come before the images. Snapd's changes SHALL NOT be
compared. When `--state` is given and either image has no readable snapd
state, the command SHALL say so on stderr, naming the image, and compare
the rest. The command SHALL need the same tools as `idiot inspect image`,
SHALL fail on a missing file with `image not found` and on a file that
isn't a Core image as `idiot inspect image` does, naming the file, and
SHALL fail with its usage message unless given exactly two images.

#### Scenario: New kernel revision
- **WHEN** `b.img` was built from the same model as `a.img` with a newer pc-kernel and the user runs `idiot inspect diff a.img b.img`
- **THEN** stdout shows pc-kernel's seeded revision in each image (and the model's `timestamp` if it changed), and nothing about the snaps, partitions or headers that are the same

#### Scenario: Different models
- **WHEN** the user compares an image built from `ubuntu-core-24-amd64` with one built from `ubuntu-core-24-amd64-dangerous`
- **THEN** stdout shows the differing `model` and `grade` (and `sign-key-sha3-384` when different keys signed them), and any seeded snap only one of them has

#### Scenario: Identical images
- **WHEN** the user runs `idiot inspect diff a.img copy-of-a.img`
- **THEN** stdout is empty, stderr says the images don't differ, and the command exits zero

#### Scenario: Images differ
- **WHEN** the images differ in anything compared
- **THEN** the command exits zero after printing the differences

#### Scenario: State compared
- **WHEN** both images have completed first boot without encryption and the user runs `idiot inspect diff --state a.img b.img`
- **THEN** stdout also shows installed snaps and connections that differ

#### Scenario: State unreadable
- **WHEN** the user runs `idiot inspect diff --state fresh.img booted.img` and `fresh.img` has never booted
- **THEN** stderr says `fresh.img` has no readable snapd state, the model, system, seed and partition differences are still printed, and no installed snaps or connections are compared

#### Scenario: Not a Core image
- **WHEN** the user runs `idiot inspect diff a.img README.md`
- **THEN** the command fails saying the ubuntu-seed partition of `README.md` was not found

#### Scenario: One image
- **WHEN** the user runs `idiot inspect diff a.img`
- **THEN** the command fails with `usage: idiot inspect diff [options] <a.img> <b.img>`

### Requirement: Comparison output
`idiot inspect diff` SHALL group its human-readable output by section
(model, system, seed snaps, partitions, and with `--state` installed snaps
and connections), omitting sections with no difference, and SHALL mark
each item present only in the first image `-`, only in the second `+`,
and in both but different `~` with each changed field's two values. It
SHALL color that output only when stdout is a terminal and `NO_COLOR` is
unset. With `--tsv` it SHALL instead print one line per differing field,
with five tab-separated columns: section (`model`, `system`, `seed`,
`partition`, `installed`, `connection`), item (the header, snap,
partition or connection), field, the value in the first image and the
value in the second, an empty value meaning the item is absent from that
image; `--tsv` output SHALL never be colored. Messages SHALL go to stderr
in both modes.

#### Scenario: Piped
- **WHEN** the user runs `idiot inspect diff a.img b.img | less`
- **THEN** the output contains no color escape sequences

#### Scenario: Machine-readable
- **WHEN** the user runs `idiot inspect diff --tsv a.img b.img` and only pc-kernel's revision differs, 1400 in `a.img` and 1500 in `b.img`
- **THEN** stdout is exactly the line `seed`, `pc-kernel`, `revision`, `1400`, `1500` separated by tabs

#### Scenario: Snap only in one image
- **WHEN** `b.img` seeds an optional snap `a.img` lacks and the user runs `idiot inspect diff --tsv a.img b.img`
- **THEN** stdout has a `seed` row for that snap's revision and one for its channel, each with an empty first value

### Requirement: Assertion summary
`idiot inspect assertion [--remote] [--no-verify] <file|->` SHALL read an
assertion stream from the file, or from stdin when given `-`, and print on
stdout, for each assertion in the order they appear:
- its type, its primary key headers, `authority-id`, the brand or account
  it is about, `sign-key-sha3-384`, and its `timestamp` and `revision`
  when present;
- its remaining headers, a multi-line header summarised by its number of
  entries unless the type's summary below shows it;
- for `model`: each snap with its type, default channel, presence and id,
  and each validation set with its mode (a pre-UC20 model: its kernel,
  gadget and required snaps);
- for `account-key`: the public key's algorithm, size and SHA3-384, and
  its `since`/`until` validity;
- for `validation-set`: each snap with its presence and revision;
- for `system-user`: its models, `since`/`until` and the number of SSH
  keys, showing only whether a password is set and never the password
  hash;
- for `snap-declaration` and `snap-revision`: the snap's id, name,
  publisher or developer, and for a revision its number, size and
  SHA3-384;
- for any other type: its headers and its body's size.
It SHALL color that output only when stdout is a terminal and `NO_COLOR`
is unset. It SHALL fail on a missing file, on input with no assertion in
it, and on an assertion it can't parse (naming its position in the
stream). JSON input SHALL fail saying it is an unsigned model, not a
signed assertion. It SHALL NOT need python3 or any other tool from the host or a
component; `snap`, when it is installed, only adds keys.

#### Scenario: A signed model
- **WHEN** the user runs `idiot inspect assertion ubuntu-core-24-amd64.model`
- **THEN** stdout shows `type: model`, brand `canonical`, model `ubuntu-core-24-amd64`, grade `signed`, its signing key, and a table of its snaps (pc, pc-kernel, core24, snapd, console-conf…) with type, default channel, presence and id

#### Scenario: From stdin
- **WHEN** the user runs `snap known account-key account-id=canonical | idiot inspect assertion -`
- **THEN** every account-key in the stream is summarised in turn, each with its name, key size and SHA3-384

#### Scenario: System user password
- **WHEN** the user inspects a system-user assertion that carries a password hash
- **THEN** the output says a password is set and contains no part of the hash

#### Scenario: Unsigned model
- **WHEN** the user runs `idiot inspect assertion ubuntu-core-24-amd64.json`
- **THEN** the command fails saying the file is an unsigned model, not a signed assertion

#### Scenario: Not an assertion
- **WHEN** the user runs `idiot inspect assertion README.md`
- **THEN** the command fails saying no assertion was found

#### Scenario: Host without Python
- **WHEN** the host has no python3 and the user runs `idiot inspect assertion ubuntu-core-24-amd64.model`
- **THEN** the same summary is printed, and the signature is checked, as on a host with python3

### Requirement: Assertion signatures
Unless given `--no-verify`, `idiot inspect assertion` SHALL report for
each assertion one of:
- `verified`, naming the trusted key the chain ends at, when its
  signature and each signature up the chain of account-keys verify and
  the chain ends at a key in this machine's snapd assertion database or
  at Canonical's root key, which idiot bundles together with Canonical's
  `models` key (itself checked against the root);
- `invalid`, saying why, when a signature in the chain doesn't match, a
  signing account-key's `account-id` isn't the signed assertion's
  `authority-id`, or the assertion's timestamp is outside the key's
  `since`/`until`;
- `unverified`, naming the missing key, when an account-key in the chain
  isn't in the stream, the local database, or (with `--remote`) the
  store.
It SHALL look up account-keys in the stream first, then in the local
database, then among the bundled keys, and in the store only with
`--remote`; without `--remote` it
SHALL make no network request. It SHALL NOT change the local assertion
database and SHALL NOT need root or store credentials. When `snap` isn't
available it SHALL use only the stream and the bundled keys, and say so
on stderr. The command SHALL exit 1, after printing every assertion,
when any assertion is `invalid`, and 0 otherwise.

#### Scenario: Canonical model, offline
- **WHEN** the user runs `idiot inspect assertion ubuntu-core-24-amd64.model` without `--remote`, on a classic host whose snapd has never seen Canonical's `models` key
- **THEN** the model is `verified` through the bundled `canonical/models` key, ending at `canonical/root`, and no network request is made

#### Scenario: Brand key not known locally
- **WHEN** the user inspects a model signed by a brand key this machine's snapd has never seen, without `--remote`
- **THEN** the model is `unverified`, naming the key's SHA3-384 and suggesting `--remote`, and the command exits zero

#### Scenario: Fetching the key
- **WHEN** the user runs the same command with `--remote` and the store has the brand's account-key
- **THEN** the key is fetched, its own signature checked up to `canonical/root`, and the model is `verified`

#### Scenario: Tampered assertion
- **WHEN** one header of a signed model has been edited after signing
- **THEN** the model is `invalid` (the signature doesn't match), every other assertion in the stream is still reported, and the command exits 1

#### Scenario: Skipping verification
- **WHEN** the user runs `idiot inspect assertion --no-verify pc.model`
- **THEN** the summary is printed with no signature status, `snap` isn't run, and the command exits zero

### Requirement: Assertion cross-checks
For a `model` assertion, `idiot inspect assertion` SHALL warn on stderr
when:
- its `store` header differs from the active store (`UBUNTU_STORE_ID`),
  or it names a store and no store is active, or a store is active and it
  names none; the warning SHALL name the saved store (`idiot store`) whose
  ID is the model's `store`, when there is one;
- a model with the same `brand-id` and `model` exists in a cloned model
  repository (`idiot model`) with a different revision or different
  content, naming that file and both revisions;
- its `authority-id` differs from its `brand-id`.
When a saved model is identical it SHALL say so on stderr, naming the
file. These checks SHALL NOT change the exit status.

#### Scenario: Wrong store active
- **WHEN** `UBUNTU_STORE_ID` is `acme-store-id` and the user inspects a model whose `store` header is `other-store-id`, saved with `idiot store add other other-store-id`
- **THEN** stderr warns the model's store is `other-store-id` (saved as `other`) while `acme-store-id` is active, and the command still exits zero

#### Scenario: Saved model is newer
- **WHEN** the user inspects revision 2 of a model and the canonical clone holds revision 3 of the same brand and model
- **THEN** stderr warns that the saved model differs, naming its file and revision 3

#### Scenario: Matches a saved model
- **WHEN** the user inspects a file copied with `idiot model pick --signed`
- **THEN** stderr says it matches the saved model, naming the file

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
