# Tasks

## 1. Shared connection helpers

- [x] 1.1 `share/vm.sh`: `vm_ssh_opts`, `vm_ssh <port> <remote word>...` (each word `shell_quote`d) and `vm_scp <port> <args>...`; switch `do_ssh`/`do_scp` to the shared options with their current argument handling; verify `idiot vm ssh -- uname -a` and `idiot vm scp ./f vm:` behave as before, and `vm_ssh <port> printf '%s\n' 'a b' '$HOME'` prints `a b` and `$HOME` literally
- [x] 1.2 Run `make check` and commit (`refactor(vm): share the ssh and scp connection`)

## 2. `idiot vm install`

- [x] 2.1 `do_install`: options (`--name`, `--dangerous`, `--classic`, `--devmode`, `--timeout`), host-side file checks, `.assert` detection, guest temp dir, scp, `snap ack`, `sudo -n snap install --no-wait`, cleanup trap, `sudo -n` password error; verify "Missing file", "Unasserted app snap" (no copy left in the guest's temp dir) and "With assertions" (`snap download hello-world`) on an Ubuntu Core 24 amd64 VM
  - Tested on an Ubuntu 24.04 cloud-image guest (cloud-init user `tester`, passwordless sudo, snapd 2.x), not Ubuntu Core, which needs a console-conf login. Checked: missing file, unasserted (`--dangerous` said; no copy left in the guest), hello-world 29 with its .assert (revision 29, not x1), explicit `--dangerous`, and sudo asking for a password (fails saying so). Bug found and fixed: the ssh helpers now split their options on spaces whatever the caller's IFS is.
- [ ] 2.2 Follow the change: `snap watch`, status from `snap change`, reboot wait loop (ssh exit 255 → poll until `--timeout`, early exit when qemu stops), result lines from `snap list`, failed-task output and exit 1; verify "Kernel snap reboots the guest" (rebuild pc-kernel with a changed version or repack the store one), "Install fails" (an app snap whose install hook exits 1) and "Several VMs running"
  - Done except a real kernel snap: classic snapd won't install one. Reboot path: rebooting the guest during a 45 s install hook makes idiot say the guest is rebooting, wait for ssh and resume; it then reports snapd's result (the hook was killed, Error) and exits 1. A failing install hook shows its task and error and exits 1. Two VMs: `--name noble2` installs only there; without `--name` it fails listing both.
- [x] 2.3 Help for `install` and the vm help's subcommand list; README's vm section (the dev loop); run `make check` and commit (`feat(vm): install snaps into a VM`)

## 3. `idiot vm logs`

- [x] 3.1 `do_logs`: `--name`, optional `<snap>[.<app>]`, `-f`, `-n`; `snap logs` or `journalctl -b` through `vm_ssh` with stdout passed through; verify the four "Guest logs" scenarios, including `-f` during a reinstall and `| grep` with nothing but journal lines on stdout
  - All four checked on the cloud-image guest: `-n`, default 10 lines, the whole journal piped to grep, an unknown snap (snapd's error, empty stdout, exit 1), and `-f` showing x3, then Stopped/Started, then x4 during a reinstall.
- [x] 3.2 Help and README; run `make check` and commit (`feat(vm): stream guest logs`)
  - Committed together with install as one `feat(vm)` commit: both live in cmd/vm and were finished together.

## 4. Integration

- [ ] 4.1 Walk every scenario in this change's vm spec on the snap (host side, components only) and from a source checkout, with two VMs running, and run `make check`
