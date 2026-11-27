# Design

## Context

`do_run` in `cmd/vm` starts swtpm with `--daemon --terminate` (it exits
when QEMU drops its control connection), installs an EXIT trap that stops
swtpm, then runs QEMU in the foreground with the serial console and the
human monitor multiplexed on stdio (`-chardev stdio,mux=on,logfile=…`,
`-mon chardev=con0`). QEMU writes `qemu.pid` and listens on `qmp.sock`;
`vm_running`, `vm stop`, `vm list` and `vm logs --console -f` (`tail
--pid`) all go by those, not by the launching process. Under the snap `vm`
is a host command, so QEMU already runs in the user's shell, outside
confinement.

## Goals / Non-Goals

**Goals:**
- Start a VM without tying up the terminal, surviving the shell.
- Report start-up failures as a foreground boot does.
- Leave every other `vm` command working on a detached VM unchanged.
- A second VM never fails on something idiot could have sorted out or
  explained: forwards behave like the ssh port, shared images are
  refused by name.

**Non-Goals:**
- Attaching an interactive console to a detached VM later (`vm console`):
  `vm ssh` covers interactive use; the serial console is logged.
- Waiting for the guest to come up (ssh answering): `vm install` and
  `vm logs` already wait or say the guest isn't reachable.
- Remembering `--detach` per VM.
- Memory and CPUs: every VM gets `-m 2048 -smp $(nproc)` as now; the
  host overcommits, and a knob for them is a change of its own.
- Guest ports: two VMs forwarding to the same guest port is fine (each
  guest has its own).

## Decisions

**QEMU's `-daemonize`, not `setsid … &`.** With `-daemonize` the parent
QEMU process exits only after the machine is set up, with a non-zero
status and its error on stderr if it couldn't be; the child is detached
from the terminal and session. Backgrounding with `&`/`setsid` would
return before QEMU had failed or succeeded, so idiot would have to poll
`qemu.pid` and guess at a timeout, and QEMU's error would end up in a
file. `-daemonize` is in every QEMU idiot supports (host and bundled), so
nothing new is staged.

**Serial to a file, no human monitor.** `-daemonize` refuses stdio
chardevs, so a detached VM gets `-chardev file,id=con0,path=serial.log`
`-serial chardev:con0` and no `-mon`. The log is the same file a
foreground boot writes (rotated to `serial.log.1` the same way), so `vm
logs --console [-f] [--previous]` needs no change. QMP (`qmp.sock`) stays,
so `vm stop` is unchanged. A socket chardev (attach later) was considered
and left for a `vm console` of its own (Non-Goals).

**The EXIT trap stops swtpm only when the boot failed.** After a
successful detached start idiot exits while QEMU runs; the trap must not
kill the swtpm QEMU is using. A flag set after QEMU returns 0 makes
`_cleanup` leave swtpm alone; `--terminate` then ends it when QEMU exits,
however it exits (the "Emulated TPM lifetime" requirement), and `vm stop`
still stops it as before. If QEMU fails to start, the trap stops swtpm as
it does now.

**`--gfx` is not refused.** QEMU decides whether its display can run
daemonized; the bundled QEMU has no display anyway. idiot doesn't
second-guess it.

**Forwards go through `vm_port_free`.** It already says whether a port
is free of running VMs' ssh ports and host listeners (`idiot-helper
port-free`); it gains the running VMs' recorded `ports`, and says which
VM holds a port so the error can name it. Picking a moved forward reuses
the ssh port's search: the first free port above the recorded one (up to
100 above), skipping the VM's own ssh port and its other forwards. The
ssh port is picked first, so a forward never takes it. All of this runs
before anything is written, as the ssh check does now, so a refused boot
leaves `vm.conf` as it was; `vm.conf`'s `ports` is rewritten only with
what the VM boots with.

**Shared images are checked by path, before QEMU.** QEMU's image lock
already stops the second VM, but its error names neither the VM nor the
fix. idiot compares the absolute image paths recorded in the running
VMs' `vm.conf` (they are made absolute at creation) and their
`in_place`. Overlays only read the image, so they share it freely. Paths
are compared after `readlink -f`, so a symlink to the image counts.

**`vm list` gets a `FORWARDS` column before `IMAGE`.** `IMAGE` stays
last since it is the one that can be long. This changes the stdout of a
listing scripts may parse; the column order was never specified beyond
the header line, and the header names it.

## Risks / Trade-offs

- QEMU's stderr after daemonizing goes to `/dev/null`: a warning QEMU
  prints while the VM runs is lost. → The guest console is still logged;
  start-up errors (the common case) are still shown.
- A port freed between idiot's check and QEMU's bind (or taken) still
  makes QEMU fail. → The same race the ssh port has; QEMU's error shows.
- `vm list` scripts that split on columns see a new one. → Header line;
  the image path stays last.
- No HMP for a detached VM (Ctrl-a c). → QMP covers stop; anything else
  can use `-- -monitor unix:…,server,nowait` passed through to QEMU.
