# Design

## Context

A VM's state directory (`share/vm.sh`, `cmd/vm`) holds, besides
`vm.conf`:

| File | What | Created by |
|---|---|---|
| `disk.qcow2` | overlay over the raw image (`-b <image> -F raw`) | `_build_disk_flags`, first boot and after a reset |
| `fw_vars.fd` | raw UEFI variable store (pflash unit 1, writable) | `_build_fw_flags`, copied from the firmware's template; absent for i386 BIOS VMs |
| `tpm2-*` | swtpm state (`tpm2-00.permall`, sometimes `.volatile`) | swtpm, only for VMs booted with `--tpm` |
| `serial.log`, `serial.log.1` | console logs | qemu |
| `qemu.pid`, `qmp.sock`, `swtpm.pid`, `swtpm.sock`, `inspect.sock*` | runtime only | qemu, swtpm, `vm tpm` |

What a VM *is* between boots is the first three; `vm.conf` holds settings
and `image_stat`, which pins the overlay to the image it was made on (a
changed image makes the boot fail, naming `vm reset`). `vm reset` removes
the first three and the logs; `vm delete` removes the directory (for a
`--state-dir` VM, an explicit file list). `--in-place` VMs boot the raw
image as the disk and have no overlay. Everything is written under
`umask 077`. `qemu-img` comes from `tool qemu-img` (host, else the `vm`
component, which stages it; `idiot doctor` checks it).

Checked live on this laptop with qemu 8.2.2, on scratch files only (a
64 MiB sparse raw base, a qcow2 overlay over it, a 1 MiB raw file standing
in for the variable store; no guest, no image downloaded):

- `qemu-img snapshot -c <tag>` on the overlay works and touches only the
  overlay; `-a <tag>` reverts its contents (checked with `qemu-io` write
  and read patterns); `-l` lists `ID TAG VM-SIZE DATE VM-CLOCK ICOUNT`
  with a disk-only snapshot's VM size `0 B`; `-d` of a missing tag exits 1.
- `qemu-img snapshot -c` **accepts a duplicate tag** (two `clean`
  snapshots, IDs 1 and 3), and `-a 1` with a snapshot *tagged* `1` applied
  the one with *ID* 1. So idiot must check uniqueness itself and refuse
  digit-only names.
- With the overlay open in a (paused, guestless) qemu, `qemu-img snapshot
  -c` and `-l` fail: `Failed to get "write" lock` / `shared "write"
  lock`. `-l -U` (force share) still lists.
- In that qemu, HMP `savevm` with the qcow2 drive and a **writable raw**
  drive beside it fails: `Device 'v' is writable but does not support
  snapshots`. With the raw drive read-only it succeeds (933 B of VM
  state). idiot's `fw_vars.fd` pflash is a writable raw drive.

## Goals / Non-Goals

**Goals:**
- A snapshot is the whole of what a VM is between boots: disk, variables,
  TPM. Restoring one can never pair a disk with another moment's TPM
  (sealed keys would fail to unseal) or variables (dbx, MOK).
- Cheap to take and restore: no copy of the overlay.
- Consistent after any failure: no half-made snapshot, no half-restored VM.
- POSIX sh only, with the qemu-img idiot already needs.

**Non-Goals:**
- Snapshots of running VMs (see Decisions).
- Snapshots of `--in-place` VMs.
- Exporting, cloning or moving snapshots between VMs, or snapshotting the
  settings in `vm.conf` (TPM on/off, ports, gfx stay as they are).
- Managing internal snapshots someone made in `disk.qcow2` by hand with
  qemu-img: idiot lists and restores only its own.

## Decisions

**Disk: an internal qcow2 snapshot of `disk.qcow2`.** The snapshot's tag
is the snapshot's name. It costs nothing to take (refcounts, no data
copied), later guest writes copy-on-write, and `qemu-img snapshot -a` is
the restore. The raw image, already opened read-only as a backing file, is
never touched; `image_stat` stays valid because the overlay file is the
same one.
- *Alternative: copy `disk.qcow2`.* Simple and independent of qemu-img
  features, but a Core VM's overlay is gigabytes after first boot and
  every snapshot would cost that.
- *Alternative: external overlays (freeze `disk.qcow2`, a new overlay on
  top).* Also cheap, but restoring and deleting a snapshot in the middle
  of a chain needs `qemu-img commit`/`rebase`, and the chain grows with
  each snapshot. Internal snapshots keep one file.

**Variables and TPM: copies beside the overlay.** `snapshots/<name>/` in
the state directory (mode 0700) holds `fw_vars.fd` and `tpm2-*` as they
were (a few KB to 4 MB) and `snapshot.conf` (key=value, never sourced, as
vm.conf): `created=<unix time>`, `vars=0|1`, `tpm=0|1`. The raw variable
store can't hold an internal snapshot, and swtpm's state is plain files.

**Names.** VM-name rules (1-32 of `[A-Za-z0-9._-]`, no leading `.`/`-`),
plus not digits only, because qemu-img resolves an argument that is an
existing snapshot ID as that ID (checked above). Uniqueness is checked by
idiot against both `snapshots/` and the overlay's tags, since qemu-img
doesn't refuse duplicates. A tag in the overlay that idiot didn't make is
therefore also a taken name, and create says so.

**Running VMs: refused.** Live snapshots were considered and dropped:
- QMP/HMP `savevm` refuses while `fw_vars.fd` is a writable raw pflash
  (checked above). Supporting it means booting the variables as qcow2,
  which changes `vm vars` (it reads the raw store), the firmware template
  copy, and every existing VM's file format: a change of its own.
- A disk-only live snapshot (QMP `blockdev-snapshot-internal-sync` or an
  external `blockdev-snapshot-sync`) is only crash-consistent, and the
  TPM and variable copies would be taken at a different moment from the
  disk (swtpm only writes its state file when the TPM changes it, and the
  firmware writes the variable store at any time). A snapshot that may not
  restore to a bootable, unsealable state is worse than none.
- So create/restore/delete check `vm_running` (and qemu's image lock is
  a second guard: qemu-img fails on a locked overlay even in a race).
  `list` reads the overlay with `-U` so it works on a running VM.
A user who wants a running VM's state stops it first (`idiot vm stop` is
a clean ACPI shutdown) and boots it again; the error names the stop.

**`--in-place` VMs: refused.** Without an overlay the disk is the user's
image file. A full copy (`cp --sparse=always`, or a reflink where the
filesystem has one) would work but costs the image's size per snapshot and
writes copies of the user's image into idiot's state; a qcow2 overlay
added after the fact would change what `--in-place` means. Left as an open
question; the error says snapshots need an overlay (a VM created without
`--in-place`).

**Create, in an order that leaves nothing half-made:**
1. resolve the VM, refuse running / `--in-place` / no overlay, validate
   the name and check it's free (also refusing a leftover
   `snapshots/.new.<name>` by removing it first);
2. copy `fw_vars.fd` and `tpm2-*` into `snapshots/.new.<name>/` and write
   `snapshot.conf`;
3. `qemu-img snapshot -c <name> disk.qcow2`; on failure remove `.new.<name>`;
4. `mv snapshots/.new.<name> snapshots/<name>`; on failure delete the
   internal snapshot.

**Restore, so the state is either before or after:**
1. resolve, refuse running / `--in-place`, check the snapshot is whole
   (directory, `snapshot.conf`, the internal tag, the copies it claims);
2. confirm (`[y/N]` on a terminal, `--yes`, else die naming `--yes`, as
   `vm delete` does);
3. copy the saved variables and TPM files into the state directory under
   temporary names (`.restore` suffix);
4. `qemu-img snapshot -a <name> disk.qcow2`; on failure remove the
   temporaries (nothing else changed);
5. `mv` each temporary over its file, then remove current files the
   snapshot didn't have (`fw_vars.fd` when `vars=0`, `tpm2-*` not in the
   snapshot). Also remove `swtpm.pid`/`swtpm.sock` leftovers so the next
   boot starts swtpm cleanly.
Before step 4 it writes `snapshots/.restoring` (the snapshot's name) and
removes it after step 5. A crash between 4 and 5 leaves the disk restored
and old TPM/variables; see Risks for how that is reported and finished.

**Delete:** remove the internal tag if present (`qemu-img snapshot -d`)
and then the directory; either missing is fine (that's a broken snapshot).

**List:** names from `snapshots/*/snapshot.conf`, ordered by `created`,
with the overlay's tags from `tool qemu-img snapshot -l -U disk.qcow2`
(`NR > 2 { print $2 }`: names have no spaces). Columns: `NAME`, `CREATED`
(`date -d @<t> '+%Y-%m-%d %H:%M'`), `UEFI` and `TPM` (`yes`/`no`), and
`STATE` (`ok`/`broken`), like `vm list`'s plain table. On stderr, a note
when the overlay has tags idiot didn't make.

**Choosing the VM.** `--name <vm>`, else the only VM; several fail naming
`--name`. Not `vm_resolve`'s "the only running VM", which would pick
exactly the VM these commands refuse. Snapshot name is the positional
argument, as with `vm install`'s files.

**Reset and delete.** Reset removes `snapshots/` with the overlay (the
internal snapshots die with `disk.qcow2` anyway) and, when there are
snapshots, confirms first with the same `--yes` rules. Asking only when
there are snapshots keeps today's reset unchanged for everyone else.
`vm delete` of a `--state-dir` VM adds `snapshots/` to the list of what
idiot created (`rm -rf` of that one directory). An image that changed
under the overlay still makes the boot fail naming `vm reset`, which is
right: the snapshots are of the old base.

**Where the code lives.** `do_snapshot` in `cmd/vm` with one function per
subcommand, like the other vm subcommands (cmd/vm is a single host file;
snapshot needs qemu-img, so it is a host command anyway); helpers in
`share/vm.sh` (`vm_snapshot_dir`, `vm_validate_snapshot_name`,
`vm_snapshot_names`, `vm_snapshot_tags`, `vm_snapshot_problem`). The
confirmation prompt shared by delete, reset and the snapshot commands
becomes one helper (`_confirm "<question>"`) in cmd/vm. No Rust: qemu-img
does the qcow2 work, and the rest is files.

**Credentials.** None are involved: the commands touch only VM state
directories. Snapshot copies keep the state directory's 0700/0600 modes
(`umask 077`, `cp` without `-p` of group bits), since TPM state can hold
sealed secrets.

## Risks / Trade-offs

- [qemu-img versions differ between the host and the `vm` component] →
  internal snapshots and `-U` have existed since qemu 2.10; the snapshot
  commands use whatever `tool qemu-img` finds, as the overlay does, and
  the bare-host checks run them with the component's.
- [Interrupted restore between the disk and the file moves] → the
  `.restore` temporaries and a `snapshots/.restoring` file naming the
  snapshot mark it; `snapshot list` says the VM is half-restored and
  names `idiot vm snapshot restore <name>`, which completes it (applying
  the disk snapshot again is idempotent). The boot isn't changed to
  refuse such a VM: that would widen this change for a crash window of
  a few `mv`s.
- [Internal snapshots keep old clusters alive, so the overlay grows] →
  that is the cost of the snapshot; `snapshot delete` frees them for
  reuse (the file doesn't shrink), and `reset` drops everything.
- [A snapshot of a VM that was killed (`vm stop --force`) is only
  crash-consistent] → as consistent as the boot after it would be anyway;
  documented in help.
- [Restoring TPM state from a snapshot under a different `--tpm` setting]
  → settings aren't snapshotted; list shows `TPM yes/no` so the user can
  boot with the matching flag.

## Migration Plan

New files only appear when a snapshot is taken; existing VMs are
unchanged. Rollback: an older idiot ignores `snapshots/`, and its
`vm reset` removes `disk.qcow2` (and with it the internal snapshots) but
leaves `snapshots/` behind as harmless files.

## Open Questions

- Should `--in-place` VMs get full-copy snapshots (`snapshot create
  --copy`, reflink when possible)? Deferred; refused for now.
- Should `restore`/`delete` without a snapshot name open an fzf picker on
  a terminal, as `vm_resolve` does for VMs? Easy to add later without
  changing the spec's behaviour for named snapshots.
- Should `vm list` show a snapshot count column? Left out to keep its
  output unchanged for scripts.
- Live snapshots, if wanted later, need the variable store booted as
  qcow2 (a separate change touching `vm vars` and existing VMs).
