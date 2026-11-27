# Proposal

## Why

Testing a gadget, kernel or recovery flow on an `idiot vm` means getting a
VM to an interesting state (first boot done, a model remodelled, keys
enrolled, a snap installed) and then breaking it. Today the only way back
is `idiot vm reset`, which goes all the way to a freshly flashed device and
a full install-mode boot, or copying the state directory by hand and
hoping the overlay, UEFI variables and TPM state still match. A named
snapshot taken once and restored in seconds makes those loops cheap.

## What Changes

A new `idiot vm snapshot` group (host commands, inside `cmd/vm` like the
other vm subcommands):

- **`idiot vm snapshot create [--name <vm>] <snapshot>`** saves a stopped
  VM's state under a name: its disk overlay (as an internal qcow2 snapshot
  of `disk.qcow2`, so it costs only the clusters that change afterwards),
  a copy of its UEFI variables (`fw_vars.fd`) and a copy of its TPM state
  (`tpm2-*`), whichever of those the VM has.
- **`idiot vm snapshot list [--name <vm>]`** prints the VM's snapshots on
  stdout: name, when it was taken, and whether it holds UEFI variables and
  TPM state. A snapshot whose parts don't match (its disk snapshot or its
  copies gone) is listed as broken.
- **`idiot vm snapshot restore [--yes] [--name <vm>] <snapshot>`** puts a
  stopped VM back to that state, discarding what changed since, after
  confirmation on the terminal (or `--yes`). The snapshot is kept, so it
  can be restored again.
- **`idiot vm snapshot delete [--yes] [--name <vm>] <snapshot>`** removes
  a snapshot (every part of it, broken ones too), after confirmation.
- **Running VMs are refused**, naming `idiot vm stop`: a live snapshot
  (QMP `savevm`) is impossible with the raw UEFI variable store idiot
  boots (qemu refuses a writable drive that can't snapshot), and a disk
  snapshot of a running guest wouldn't match its TPM and variables.
- **`--in-place` VMs are refused**: they have no overlay to snapshot, and
  copying the whole image is a different feature (an open question).
- `idiot vm reset` now also discards the VM's snapshots (they live in the
  overlay it removes), and asks for confirmation first when there are any
  (`--yes` skips it); `idiot vm delete` of a `--state-dir` VM also removes
  the snapshots idiot created there.

Output: `vm snapshot list` prints its table on stdout; everything else
(progress, confirmations, errors) goes to stderr. Nothing new goes into
the eval file. No new runtime dependency: `qemu-img` is already needed for
the overlay and comes from the host or the `vm` component, which already
stages it. No Rust helper changes.

Out of scope: live snapshots of running VMs, snapshots of `--in-place`
VMs, exporting or cloning a snapshot into a new VM, and anything the other
proposals cover (`vm-install-from-store` installs snaps into a VM;
`inspect-state-explorer` reads a VM's snapd state).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vm`: adds named snapshots of a stopped VM (create, list, restore,
  delete) and changes resetting (snapshots are discarded, with
  confirmation) and deleting (`--state-dir` snapshots are removed).

## Impact

- `cmd/vm`: `do_snapshot` dispatching `create`, `list`, `restore`,
  `delete`, with help; `do_reset` gains the confirmation and `--yes`;
  `do_delete`'s `--state-dir` file list gains the snapshots directory; the
  top-level help lists `snapshot`.
- `share/vm.sh`: snapshot helpers (name validation, the snapshot
  directory, listing and checking a snapshot's parts).
- State: a `snapshots/` directory in each VM's state directory (mode 0700,
  like the rest), one subdirectory per snapshot; internal snapshots in
  `disk.qcow2`.
- Tests: bare-host checks on a hand-made VM state directory (no boot) with
  the `vm` component's qemu-img, and a restore-and-boot check in the
  `--boot` run.
- README (VMs section), skills/using-idiot/references/commands.md, the
  `vm` spec.
