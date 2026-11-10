# Design

## Context

After `vm-named-instances`, `share/vm.sh` resolves a VM (`vm_resolve`) and
records its ssh port in `vm.conf`. `do_ssh`/`do_scp` in `cmd/vm` each
build their own command: `-p`/`-P <port>`, the Launchpad user from
`require_lp_user` (`-l` / `-o User=`), `StrictHostKeyChecking=no`,
`UserKnownHostsFile=/dev/null`, `LogLevel=ERROR`, `localhost`, with all
output sent to stderr. `ssh` and `scp` are the host's openssh client (not
bundled; Ubuntu Core ships it). See proposal.md for why.

## Goals / Non-Goals

**Goals:** one command from a built `.snap` to an installed, reported
result, surviving the reboot kernel/gadget/base installs cause; logs in
the same terminal-friendly way; one connection path for every subcommand
that talks to the guest.

**Non-Goals:** building snaps (snapcraft's job); `snap try`/mounting a
prime directory into the guest (needs a shared filesystem — virtiofs or
9p — and a separate change); removing/reverting snaps (`idiot vm ssh --
sudo snap revert` is one line); retrieving files other than logs; real
boards (see "Future `idiot device`").

## Decisions

### One connection helper for ssh, scp, install, logs
`share/vm.sh` gains `vm_ssh_opts` (sets the options above for a port and
user) plus `vm_ssh <port> <remote word>...` and `vm_scp <port> <args>...`.
`vm_ssh` quotes each remote word with `shell_quote` before handing them to
ssh, because ssh joins its arguments into one string for the guest's
shell: a snap path with spaces or `$` reaches `snap install` as one
literal argument. `do_ssh`/`do_scp` keep their current user-facing
behaviour (arguments passed through raw, as ssh users expect) and only
share the options. Host keys stay unchecked, as today: a VM's host key
changes on every reset.

### Copy to a guest temp dir, install, clean up
`vm_ssh … mktemp -d` → `$gdir`; `scp` the snaps (and assertions) into it;
`sudo -n snap ack <each .assert>`; `sudo -n snap install --no-wait [flags]
<files>` → change id on stdout; follow; finally `rm -rf $gdir` (in a trap,
and again after a reboot, when `/tmp` is gone anyway). `sudo -n` makes a
password-protected sudo fail at once with "a password is required", which
idiot turns into its own error, instead of hanging on a prompt the user
can't see. All files go in one `snap install` so snapd makes one change,
and interdependent snaps (an app and the content snap it needs) install
together.

### Assertions decide `--dangerous`
All snaps have `<base>.assert` next to them → ack them all and install
without `--dangerous` (snapd verifies the revision against the
assertions). Otherwise `--dangerous` for all (snapd's flag is per
command), said in dim text naming the snap without an assertion. An
explicit `--dangerous` wins. *Alternative:* fail without assertions
unless `--dangerous` — snapd's own default, but in a dev loop nearly every
local build is unasserted, so it would be typed every time. *Assumption to
confirm:* auto-`--dangerous` is what Lauren wants. A model with `grade:
signed` refuses dangerous installs; snapd's error says so and idiot shows
it.

### Following the change across a reboot
`--no-wait` returns the change id first, so the command can lose the
connection and still know what to follow. Loop: `vm_ssh snap watch <id>`
(snapd's own progress, to stderr); then `snap change <id>` status. On ssh
exit 255 (connection lost: the guest rebooted) say "Guest rebooting —
waiting for ssh", poll `vm_ssh true` every 2 s until `--timeout` (default
300 s; also fails early if `vm_running` says qemu exited), then resume
`snap watch`. `Done` → report; `Error`/`Undone`/`Hold` → print `snap
change <id>` (the task table) and the `snap tasks` error log lines,
exit 1. Result line per snap from `snap list <name>` (name, version,
revision); names come from the guest (`snap info --verbose <file>`
before installing), so the host needs no unsquashfs.

### Logs: `snap logs` or `journalctl`, on stdout
With a snap: `sudo -n snap logs -n=<N> [-f] <snap[.app]>`; without:
`sudo -n journalctl -b --no-pager [-n N] [-f]`. Through `vm_ssh` with
stdout passed through (unlike `do_ssh`, which sends everything to
stderr), because the journal is the data the user asked for. With `-f`,
Ctrl-C ends ssh and the command exits 130 (common.sh's INT trap). sudo
because journal access for non-root users depends on group membership the
guest's user may not have.

### Future `idiot device`
Everything above talks to a target through four values: host, port, user,
ssh options. Today `vm_resolve` produces them from `vm.conf` (`localhost`,
the recorded port). A later `idiot device` (out of scope) could resolve the
same four values from a saved board (`idiot device add <name> <host>`) and
reuse `do_install`/`do_logs` unchanged, by moving them from `cmd/vm` into
`share/` with the target as a parameter. The one thing that must change
for real boards is host-key checking: a board's key is stable and the
network is not loopback, so it must be checked (a per-device known_hosts
file in idiot's data dir). This change keeps the target values together
so that refactor is mechanical.

## Risks / Trade-offs

- [`snap watch` output when stderr is not a terminal is a stream of
  progress lines] → acceptable; with a terminal ssh gets `-t` so snapd
  draws its progress bar.
- [A snapd change that never finishes (e.g. waiting on a hook)] →
  `snap watch` follows it; Ctrl-C leaves the change running in the guest
  and idiot says its id so the user can `idiot vm ssh -- snap change <id>`.
- [Reboot detection by ssh exit 255 also fires on other connection
  errors] → after reconnecting, the change's status, not the guess,
  decides the outcome.
- [Guest user without passwordless sudo] → clear error naming the cause;
  Ubuntu Core's console-conf and system-user accounts have it.

## Assumptions

- The guest user is the Launchpad user (as for `ssh` today) and has
  passwordless sudo (console-conf / system-user default on Ubuntu Core).
- Unasserted snaps install with `--dangerous` automatically (see above).
- `idiot vm logs` defaults to 10 lines with a snap (snap logs' default) and
  the whole current boot without one.
- Classic Ubuntu guests (not Core) work the same, given snapd and
  passwordless sudo; not a target to test beyond one smoke check.
