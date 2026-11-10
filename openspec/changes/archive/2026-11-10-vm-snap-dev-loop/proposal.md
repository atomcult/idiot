# Proposal

## Why

The inner loop of snap development on Ubuntu Core is build, copy into the
device, install, watch it fail, read the logs. With `idiot vm` today that
is `idiot vm scp`, then `idiot vm ssh` and a `sudo snap install
--dangerous` typed by hand, then `snap changes`/`snap tasks` to find out
what happened, then `snap logs` in another ssh session — and a kernel or
gadget install reboots the guest mid-way and drops the session. With named
VMs (change `vm-named-instances`) idiot knows each VM's port and user, so
it can do the whole round trip in one command.

## What Changes

- `idiot vm install [--name <name>] <file.snap>... [--dangerous]
  [--classic] [--devmode]`: copies the snaps into the VM, installs them
  with `sudo snap install` in the guest, follows the snapd change to its
  end — through the guest rebooting when the snap is a kernel, gadget or
  base — and reports the result (installed name, version and revision, or
  the failed tasks and their errors), exiting non-zero on failure.
- Assertions: a `<file>.assert` next to `<file>.snap` (as `snap download`
  writes) is copied too and acknowledged in the guest first, so the snap
  installs as an asserted (store) revision without `--dangerous`. Without
  one, the install is `--dangerous`, and idiot says so.
- `idiot vm logs [--name <name>] [<snap>[.<app>]] [-f] [-n <lines>]`: the
  guest's journal for a snap's services (`snap logs`), or the whole
  journal of the current boot without a snap, optionally followed.
- `install` and `logs` reuse `ssh`/`scp`'s connection (user, port, host
  keys) through shared helpers, and pick the VM as `ssh` does.

stdout: `idiot vm logs` writes the guest's journal to stdout (data, so it
can be piped). `idiot vm install` writes only messages, to stderr.
Nothing is queued in the eval file.

Runtime dependencies: none new on the host — `ssh` and `scp` (openssh
client, already required by `idiot vm ssh`/`scp`, not bundled). In the
guest: `snap`, `sudo` usable without a password (true for Ubuntu Core's
console-conf and system-user accounts), `journalctl`. The snap stages
nothing new.

## Capabilities

### New Capabilities

### Modified Capabilities
- `vm`: adds "Installing snaps into a VM" and "Guest logs".

## Impact

`cmd/vm` (`do_install`, `do_logs`, dispatch, help), `share/vm.sh` (ssh/scp
helpers factored out of `do_ssh`/`do_scp`, used by all four), README's vm
section. Depends on `vm-named-instances` (VM selection, recorded ports);
implement after it.
