# Design

## Context

`cmd/vm` is one host command (~530 lines). `do_run` parses options, finds
qemu and firmware (`_find_qemu`, `setup_firmware`, `share/vm-firmware.sh`),
creates a work directory (`--state-dir`, else a `mktemp -d` removed on
exit), copies the vars template to `fw_vars.fd` unless it exists, starts
`swtpm --daemon` with a pid file that the EXIT trap kills, and runs qemu in
the foreground with `-drive file=<image>,format=raw` (written in place),
`-net user,hostfwd=tcp::8022-:22` and `-nographic` (or `-serial mon:stdio`
with `--gfx`). `ssh`/`scp` hard-code port 8022 (overridable with
`--port`), log in as the Launchpad user, and disable host-key checking
(`StrictHostKeyChecking=no`, `UserKnownHostsFile=/dev/null`); `vars` and
`tpm` take a state directory. `tests/bare-host.d/vm` boots with
`--state-dir`, copies the image first because it is written, and kills
qemu with `pkill`. `qemu-img` is already in the `vm` component (kept by
`snap/local/vm-build`, listed by `check-components` and the bundled-tools
spec). See proposal.md for why.

## Goals / Non-Goals

**Goals:** several named, persistent VMs at once; a reset that equals
reflashing; enough recorded state (`vm.conf`, pid, QMP socket, port) that
later commands — including the next change's `idiot vm install`/`logs` —
can find a VM by name without the user repeating options.

**Non-Goals:** running qemu in the background (`--detach`): the launching
terminal stays the console, as today; snapshots of a VM's state (save /
restore points); exporting an overlay as a flat image (`qemu-img convert`
does it; `--in-place` covers inspect-after-boot); changing memory/CPU
options; compressed images (`.img.xz`); binding forwarded ports to
localhost only (see Open Questions).

## Decisions

### State under IDIOT_DATA_HOME, not IDIOT_CACHE_DIR
`${IDIOT_DATA_HOME}/vm/<name>/` (`~/.local/share/idiot/vm/<name>`; under
the snap `~/snap/idiot/common/vm/<name>`). The directory is the device:
the overlay holds what was installed, the TPM state holds the keys an FDE
image sealed to it, and losing one without the other leaves an unbootable
guest. Cache is, by XDG's definition, safe to delete at any time
(cleaners, `rm -rf ~/.cache`), which would silently break a device the
user still thinks they have. *Alternative:* `IDIOT_CACHE_DIR` because
overlays are large and regenerable from the image — true only of the disk,
and `idiot vm reset` is the explicit way to regenerate. *Alternative:*
`XDG_STATE_HOME` — the closest XDG fit, but idiot has no state root and
the snap maps both roots into `common` anyway; not worth a third root for
one command. Under the snap both roots are in `~/snap/idiot/common`, so
the choice only matters for source installs.

### Overlay by default, `--in-place` to opt out
Default: `qemu-img create -f qcow2 -b <abs image> -F raw <dir>/disk.qcow2`,
booted with `-drive file=<dir>/disk.qcow2,format=qcow2`. Reasons:
- The disk and the TPM/UEFI state must live and die together (FDE seals
  to the TPM; the shim/boot entries live in UEFI vars). An overlay in the
  same directory makes that structural; an image written in place pairs
  with whichever state directory booted it last.
- An Ubuntu Core image is no longer "freshly flashed" after one boot
  (install mode repartitions it); users re-decompress `.img.xz` today.
  With an overlay, reset is deleting files, in milliseconds.
- One pristine image can back many VMs (qemu takes a shared lock on a
  read-only backing file), which is what two named VMs from one image
  need; written in place, the second VM would fail qemu's write lock.
- The cost is a behaviour change for anyone who relied on the image being
  modified (to `inspect image` it, or flash it to a board): `--in-place`
  gives that back explicitly and is recorded, so it sticks for that VM.
*Alternative:* in place by default with `--snapshot` opt-in — keeps
today's semantics but leaves the default path with the mismatched-state
problem the change exists to fix, and makes `reset` partial by default.
*Alternative:* qemu's `-snapshot` — a temporary overlay discarded on exit;
it can't persist between runs, which a dev device needs. The flag is
named `--in-place` rather than `--persist`: the overlay persists too; the
difference is what gets written.

The overlay records the image by absolute path; `vm.conf` also records its
size and mtime (`stat -c '%s %Y'`), checked before each boot, because
qemu would happily boot an overlay over a replaced base and corrupt the
guest's filesystems.

### Names and the state directory layout
Name from `--name`, else the image's basename minus `.img`/`.raw`,
sanitised to `[A-Za-z0-9._-]`, 1–32 characters, no leading `.`/`-`
(rejects `..`, path separators, option look-alikes). 32 bounds the socket
paths (below). Layout, all mode 0600 in a 0700 directory (`umask 077`
around creation):

```
vm.conf          key=value: image, image_stat, arch, sb, tpm, gfx,
                 in_place, ssh_port, ports
disk.qcow2       overlay (absent with --in-place)
fw_vars.fd       UEFI variables (name unchanged: vars and old dirs)
tpm2-*.permall   swtpm state (unchanged: tpm and old dirs)
swtpm.sock  swtpm.pid
qemu.pid  qmp.sock
serial.log  serial.log.1
```

`vm.conf` is read line by line with `sed`/`case`, never sourced or eval'd,
so a hand-edited file can't run code. `--state-dir <dir>` makes
`${IDIOT_DATA_HOME}/vm/<name>` a symlink to the absolute `<dir>`: one
lookup path for every subcommand, and old `--state-dir` directories (no
`vm.conf`) are adopted as new VMs with their existing vars and TPM state.

### Ports
Order: `--ssh-port`, else recorded port if free, else 8022, else the
first free port in 8023–8122. "Free" = not the recorded port of another
running VM, and binding it succeeds (`python3 -c` socket bind on the same
address qemu's `hostfwd=tcp::<port>` uses). Without python3 the bind
check is skipped and qemu's own "could not set up host forwarding rule"
error is the fallback. A race between two launches picking the same port
at once is left to that qemu error (rare; rerun). The chosen port is
written to `vm.conf` before qemu starts, so `ssh` works from the first
boot.

### Running state: pid file and QMP
qemu gets `-pidfile <dir>/qemu.pid` and
`-qmp unix:<dir>/qmp.sock,server=on,wait=off`. "Running" = the pid file
exists, `kill -0` succeeds, and `/proc/<pid>/cmdline` names
`qemu-system-` (a recycled pid is not taken for the VM). Unix socket
paths are limited to 107 bytes: the launch checks the longest socket path
(`swtpm.sock`/`qmp.sock`) up front and fails naming `--state-dir` with a
shorter path, instead of swtpm's opaque error.

### QMP client in python3 (`share/vm-qmp.py`)
`stop` needs to send `system_powerdown` and `quit`; POSIX sh can't talk
to a unix socket, socat is only in the `network` component, and `nc -U`
isn't everywhere. A ~40-line standard-library script (connect, read the
greeting, `qmp_capabilities`, send one command, print its `return` as
JSON, exit non-zero on `error`) follows `share/uefi-vars.py`'s precedent:
python3 is already in the project's stack and required by `vm vars` and
`inspect`. *Alternative:* the HMP monitor on a socket — same client
problem. *Alternative:* SIGTERM only — qemu quits without the guest
shutting down. Without python3, `stop` says graceful shutdown needs
python3 and sends SIGTERM, then SIGKILL after the timeout. Stop sequence:
`system_powerdown`; poll the pid every second up to `--timeout`; `quit`;
2 s; `kill -TERM`; 2 s; `kill -KILL`; then kill an orphaned swtpm from
`swtpm.pid` (the launching `idiot vm`'s EXIT trap normally does it).

### Serial console: chardev with a log file
Replace `-nographic` / `-serial mon:stdio` with
`-chardev stdio,id=con0,mux=on,logfile=<dir>/serial.log -serial
chardev:con0 -mon chardev=con0,mode=readline`, plus `-display none`
unless `--gfx`. The mux keeps today's keys (Ctrl-a x, Ctrl-a c for the
monitor) and stdin/stdout on the terminal. qemu's chardev log records
what the guest sends, not what the user types, so a password typed at a
login prompt (not echoed by the guest) is not logged; the file is 0600.
Before each boot `serial.log` is moved to `serial.log.1`. `console-log -f`
is `tail -f --pid=<qemu pid>` (GNU tail, present on every Ubuntu host), so
it stops when the VM does.

### Choosing a VM
One helper, `vm_resolve [<name>]`, used by every subcommand: given a name,
the VM or die (naming `idiot vm list`); else the running VMs — one: it;
none: die; several: fzf picker of the names when `/dev/tty` is usable and
fzf is found, else die listing them. `ssh`/`scp` `--port` bypasses it.

### Code layout
`cmd/vm` stays one host command: its run form takes the image as the
first word (`idiot vm core.img`), so it can't become a `dispatch_subcmd`
group without breaking that. VM state helpers (name validation, state
dir lookup, `vm.conf` read/write, running check, port choice, resolve)
move to a new `share/vm.sh` (4-space indent), sourced by `cmd/vm`, which
gains `do_list`, `do_stop`, `do_reset`, `do_delete`, `do_console_log` and
the next change's subcommands. `_setup_workdir` loses the `mktemp` branch:
every VM has a directory.

## Risks / Trade-offs

- [Behaviour change: the image is no longer written, and state that was
  discarded now persists] → the proposal marks it breaking; README and
  the launch's first-boot message say where the state is and name
  `--in-place` and `idiot vm reset`.
- [Overlays grow to the size of everything the guest writes; under the
  snap, `snap remove idiot` snapshots `~/snap/idiot/common`, VMs
  included] → `idiot vm list` could show sizes later; README notes
  `idiot vm delete` and `snap remove --purge`.
- [A recorded port may be taken by another program while the VM is
  stopped] → the next boot picks another and says so; `ssh` reads the
  current port from `vm.conf`.
- [Ubuntu Core hosts without python3 lose the graceful stop and the
  port check] → both degrade (signals, qemu's own error) rather than
  fail.
- [`-chardev stdio` instead of `-nographic` changes qemu's defaults
  subtly (no implicit parallel/monitor devices)] → verified by the
  bare-host boot test and a manual Ctrl-a check on all four
  architectures' machine types.

## Migration Plan

No data migration: an existing `--state-dir` directory keeps working
(same file names) and is adopted on its next boot. Rollback is the
previous `cmd/vm`; the state directories it doesn't know are ignored by
it, and `--state-dir` dirs stay readable by it.

## Assumptions

- A VM's default name comes from the image file name, not a fixed
  `default`, so `idiot vm a.img` then `idiot vm b.img` are two VMs rather
  than a conflict.
- The launch stays in the foreground; `stop`, `list`, `ssh` run from
  another terminal.
- `--arch`, `--sb` and `--in-place` are fixed per VM (the vars file and
  disk depend on them); `--tpm`, `--gfx` and ports are not.
- python3 is on the host where graceful stop matters (Ubuntu desktops and
  servers; core24-based Ubuntu Core is believed to ship it — to confirm).
- ssh keeps host-key checking off: VMs are recreated often, and per-VM
  known_hosts would only add prompts.

## Open Questions

- Should forwarded ports bind to 127.0.0.1 only (today they listen on all
  interfaces, reachable from the LAN)? Deferrable: it changes one
  `hostfwd` string and no spec here.
- Show overlay disk usage in `idiot vm list`? Deferrable, additive.
