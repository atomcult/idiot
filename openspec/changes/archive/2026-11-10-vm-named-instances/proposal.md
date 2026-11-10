# Proposal

## Why

`idiot vm` runs one anonymous VM at a time: the ssh port is a fixed 8022, so
a second VM can't be reached; nothing records which VMs exist or whether
they are running; and the only way back to a freshly flashed device is to
decompress the image again and delete a hand-made `--state-dir`. Without
`--state-dir` the UEFI variables and TPM state are thrown away after every
run while the disk keeps what the guest wrote, so a TPM-backed FDE image
fails to unseal on its second boot. Iterating on gadget, kernel and app
snaps needs devices that can be named, run side by side, stopped, and reset.

## What Changes

- **Named VMs.** `idiot vm [--name <name>] <image>` boots a VM whose state
  (disk overlay, UEFI variables, TPM state, serial log, QMP socket, pid,
  ssh port, launch settings) lives in its own directory,
  `${IDIOT_DATA_HOME}/vm/<name>/`. Without `--name` the name comes from the
  image's file name (`ubuntu-core-24-amd64.img` → `ubuntu-core-24-amd64`),
  so today's invocation keeps working. `idiot vm --name <name>` without an
  image boots an existing VM again.
- **BREAKING (behaviour):** the image is no longer written. By default the
  VM boots from a qcow2 overlay backed by the read-only image, and its UEFI
  variables and TPM state persist between runs. `--in-place` writes the
  image directly, as today (for images that will be inspected or flashed
  afterwards). Without `--state-dir`, state used to be discarded on exit;
  it is now kept until `idiot vm reset` or `idiot vm delete`.
- **ssh port per VM.** `--ssh-port` is still honoured; without it the VM
  keeps the port it had last time, or takes 8022, or the next free port,
  and the port is recorded so `idiot vm ssh`/`scp` find it.
- `idiot vm list`: name, image, architecture, running/stopped and ssh port
  of each VM, on stdout.
- `idiot vm stop <name>`: ACPI power-down through QMP, then quit and kill
  after a timeout.
- `idiot vm reset <name>`: discard the overlay, UEFI variables and TPM
  state, so the next boot is a freshly flashed device.
- `idiot vm delete <name>`: remove a stopped VM's state directory (asks
  first).
- `idiot vm console-log [<name>] [-f] [--previous]`: the serial console of
  the current (or previous) boot, which is always logged to the VM's state
  directory while staying interactive on the terminal.
- `ssh`, `scp`, `vars` and `tpm` take `--name <name>`; without it they
  target the only running VM. `vars`/`tpm` still accept a state directory
  path, and `--state-dir` still chooses where a VM's state lives.

stdout: `idiot vm list` and `idiot vm console-log` write data to stdout
(new commands). Nothing else changes: the launch, `ssh` and `scp` keep
writing to stderr, and no command queues anything in the eval file.

Runtime dependencies: `qemu-img` (already in the `vm` component and the
qemu-utils package; the snap stages nothing new). python3, already needed
by `idiot vm vars` and `idiot inspect`, is now used by `stop` (QMP) and to
check that a port is free; `stop` falls back to signals without it.

## Capabilities

### New Capabilities

### Modified Capabilities
- `vm`: adds named VMs and their state directory, the disk overlay and
  `--in-place`, ssh port allocation, `list`, `stop`, `reset`, `delete`,
  the serial log and `console-log`, and VM selection for `ssh`/`scp`;
  "Saved UEFI variables" also accepts `--name`.

## Impact

`cmd/vm` (launch, new subcommands, VM selection), a new `share/vm.sh`
(state directory, name, port and pid helpers, sourced by `cmd/vm` and,
in the next change, by snap install/logs), a new `share/vm-qmp.py` (a
standard-library QMP client), README's vm section. `tests/bare-host.d/vm`
keeps working (it passes `--state-dir`), and gains checks for the overlay
and the serial log. No snap or component changes: `qemu-img` is already
in the `vm` component and its index.
