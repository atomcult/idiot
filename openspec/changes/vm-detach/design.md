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

**Non-Goals:**
- Attaching an interactive console to a detached VM later (`vm console`):
  `vm ssh` covers interactive use; the serial console is logged.
- Waiting for the guest to come up (ssh answering): `vm install` and
  `vm logs` already wait or say the guest isn't reachable.
- Remembering `--detach` per VM.

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

## Risks / Trade-offs

- QEMU's stderr after daemonizing goes to `/dev/null`: a warning QEMU
  prints while the VM runs is lost. → The guest console is still logged;
  start-up errors (the common case) are still shown.
- No HMP for a detached VM (Ctrl-a c). → QMP covers stop; anything else
  can use `-- -monitor unix:…,server,nowait` passed through to QEMU.
