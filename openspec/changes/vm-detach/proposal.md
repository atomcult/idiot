# Proposal

## Why

`idiot vm boot` keeps the terminal on the guest's serial console until the
guest powers off. To boot a VM and carry on in the same shell (or from a
script or an agent), the user has to background it themselves (`&`,
stdin from `/dev/null`, output to a file) and then has a job tied to that
shell, which dies with it. Everything that drives a running VM afterwards
(`vm ssh`, `install`, `logs --console`, `stop`) already works without that
terminal, so the only thing missing is a way to start the VM detached.

## What Changes

- `idiot vm boot --detach` (and the short form `idiot vm --detach
  <image>`) starts the VM in the background and returns once QEMU has
  started it: the same checks and the same errors as a foreground boot,
  then a line naming the VM, its ssh port, and how to follow its console
  and stop it.
- A detached VM has no console on a terminal: what the guest writes to the
  serial console goes only to its log, which `idiot vm logs --console -f`
  follows. QEMU's human monitor (Ctrl-a c) is not available; `vm stop`
  already uses QMP.
- Its swtpm keeps running with QEMU after `idiot vm boot` returns, and
  still ends with QEMU however QEMU exits.
- `--detach` is a per-boot choice: it is not recorded in the VM's
  settings.
- stdout is unchanged (nothing is printed there); nothing is queued in the
  eval file (`vm` stays a host command).
- No new runtime dependency: QEMU's own `-daemonize` does the
  backgrounding, in the host's QEMU and the bundled one alike.

## Capabilities

### New Capabilities

### Modified Capabilities
- `vm`: a new "Detached VMs" requirement; "Serial console log" no longer
  promises an interactive console on the terminal for a detached VM.

## Impact

- `cmd/vm` (`do_run`: option parsing, help, the serial and monitor flags,
  the exit trap, the closing message).
- `README.md`, `skills/using-idiot/SKILL.md` (the advice to background
  `idiot vm boot` by hand).
- `tests/bare-host.d/vm`: a detached boot, console log, stop, no swtpm
  left.
