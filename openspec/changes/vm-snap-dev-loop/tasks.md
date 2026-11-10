# Tasks

## 1. Shared connection helpers

- [ ] 1.1 `share/vm.sh`: `vm_ssh_opts`, `vm_ssh <port> <remote word>...` (each word `shell_quote`d) and `vm_scp <port> <args>...`; switch `do_ssh`/`do_scp` to the shared options with their current argument handling; verify `idiot vm ssh -- uname -a` and `idiot vm scp ./f vm:` behave as before, and `vm_ssh <port> printf '%s\n' 'a b' '$HOME'` prints `a b` and `$HOME` literally
- [ ] 1.2 Run `make check` and commit (`refactor(vm): share the ssh and scp connection`)

## 2. `idiot vm install`

- [ ] 2.1 `do_install`: options (`--name`, `--dangerous`, `--classic`, `--devmode`, `--timeout`), host-side file checks, `.assert` detection, guest temp dir, scp, `snap ack`, `sudo -n snap install --no-wait`, cleanup trap, `sudo -n` password error; verify "Missing file", "Unasserted app snap" (no copy left in the guest's temp dir) and "With assertions" (`snap download hello-world`) on an Ubuntu Core 24 amd64 VM
- [ ] 2.2 Follow the change: `snap watch`, status from `snap change`, reboot wait loop (ssh exit 255 → poll until `--timeout`, early exit when qemu stops), result lines from `snap list`, failed-task output and exit 1; verify "Kernel snap reboots the guest" (rebuild pc-kernel with a changed version or repack the store one), "Install fails" (an app snap whose install hook exits 1) and "Several VMs running"
- [ ] 2.3 Help for `install` and the vm help's subcommand list; README's vm section (the dev loop); run `make check` and commit (`feat(vm): install snaps into a VM`)

## 3. `idiot vm logs`

- [ ] 3.1 `do_logs`: `--name`, optional `<snap>[.<app>]`, `-f`, `-n`; `snap logs` or `journalctl -b` through `vm_ssh` with stdout passed through; verify the four "Guest logs" scenarios, including `-f` during a reinstall and `| grep` with nothing but journal lines on stdout
- [ ] 3.2 Help and README; run `make check` and commit (`feat(vm): stream guest logs`)

## 4. Integration

- [ ] 4.1 Walk every scenario in this change's vm spec on the snap (host side, components only) and from a source checkout, with two VMs running, and run `make check`
