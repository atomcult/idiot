# Proposal

## Why

`idiot vm boot` keeps the terminal on the guest's serial console until the
guest powers off. To boot a VM and carry on in the same shell (or from a
script or an agent), the user has to background it themselves (`&`,
stdin from `/dev/null`, output to a file) and then has a job tied to that
shell, which dies with it. Everything that drives a running VM afterwards
(`vm ssh`, `install`, `logs --console`, `stop`) already works without that
terminal, so the only thing missing is a way to start the VM detached.

With VMs easy to keep running side by side, starting a second one should
sort out what the two would otherwise fight over. The ssh port already
moves to a free one; the extra `--port host:guest` forwards don't: a host
port another VM forwards makes QEMU fail with a terse hostfwd error, and
a VM booted again with its recorded forwards fails the same way. Two VMs
on one image where either writes it `--in-place` fail on QEMU's image
lock, with an error that doesn't say which VM holds it.

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
- `--port` forwards are handled as the ssh port is: a host port given on
  the command line that another running VM forwards, or something on the
  host listens on, fails before QEMU starts, naming the VM when it is one;
  a recorded forward whose host port is now taken moves to the first free
  port above it, and idiot says the new mapping and records it. A
  `--port` host port that is the VM's own ssh port, or given twice, is
  refused.
- `idiot vm list` gains a `FORWARDS` column (the `--port` mappings, `-`
  for none) before `IMAGE`: **this changes `vm list`'s stdout**.
- Booting a VM on an image that another running VM writes `--in-place`,
  or booting `--in-place` an image another running VM uses, fails before
  QEMU starts, naming that VM.
- `--detach` is a per-boot choice: it is not recorded in the VM's
  settings.
- `vm boot` prints nothing on stdout, detached or not; nothing is queued in the
  eval file (`vm` stays a host command).
- No new runtime dependency: QEMU's own `-daemonize` does the
  backgrounding, in the host's QEMU and the bundled one alike.

## Capabilities

### New Capabilities

### Modified Capabilities
- `vm`: new "Detached VMs", "Port forwards per VM" and "VMs sharing an
  image" requirements; "Serial console log" no longer promises an
  interactive console on the terminal for a detached VM; "Listing VMs"
  shows the forwards.

## Impact

- `cmd/vm` (`do_run`: option parsing, help, the serial and monitor flags,
  the exit trap, the closing message, the forward and image checks;
  `do_list`).
- `share/vm.sh` (`vm_port_free` counts other running VMs' forwards; a
  picker for forwards; finding the running VM that uses an image).
- `README.md`, `skills/using-idiot/SKILL.md` (the advice to background
  `idiot vm boot` by hand).
- `tests/bare-host.d/vm`: a detached boot, console log, stop, no swtpm
  left; `tests/bare-host.d/vm-snapshot` (or a new check) for forwards and
  shared images, which need no guest that boots.
