# Tasks

## 1. vm boot --detach

- [x] 1.1 Parse `--detach` in `do_run` (both `idiot vm boot` and `idiot vm <image>`); add it to the help and the usage line; don't record it in `vm.conf`
- [x] 1.2 Detached: `-daemonize`, the serial console to `-chardev file,…,path=serial.log` and no `-mon`; foreground flags unchanged
- [x] 1.3 `_cleanup` leaves swtpm running once a detached QEMU has started; stops it as now when QEMU fails
- [x] 1.4 On success say the name and ssh port and name `idiot vm logs --console -f --name <n>` and `idiot vm stop <n>`; nothing on stdout
- [x] 1.5 `make check`; commit `feat(vm): boot --detach`

## 2. A second VM

- [x] 2.1 `vm_port_free` also counts other running VMs' recorded forwards, and can name the VM that holds a port
- [x] 2.2 `--port`: refuse a host port given twice or equal to the VM's ssh port; refuse a taken one before qemu, naming the VM; move a taken recorded forward to the first free port above it, say the new mapping, record what the VM boots with
- [x] 2.3 Refuse booting on an image another running VM writes `--in-place`, or `--in-place` on an image another running VM uses, naming it (paths through `readlink -f`)
- [x] 2.4 `vm list`: a `FORWARDS` column before `IMAGE`
- [x] 2.5 `make check`; commit (with group 1: the two share do_run, so one `feat(vm)` commit)

## 3. Docs and tests

- [x] 3.1 README: a `--detach` line and a second VM with forwards in the vm examples
- [x] 3.2 `skills/using-idiot/SKILL.md`: recommend `--detach` over backgrounding `idiot vm boot` by hand
- [x] 3.3 `tests/bare-host.d/vm`: a detached `--tpm` boot returns, `vm list` shows it running, the console log fills, `vm stop` leaves no swtpm; a detached boot QEMU refuses fails and leaves no swtpm
- [x] 3.4 `tests/bare-host.d/vm-snapshot` (or a new check without a bootable guest): a second VM's clashing `--port` refused naming the first, a recorded one moved, `vm list`'s forwards, an `--in-place` clash refused, overlays side by side
- [x] 3.5 `make check`; commit `docs: vm boot --detach` and `test(bare-host): vm boot --detach`
- [ ] 3.6 Run `tests/bare-host --image local:8257342f834e <snap> <comps>` on `local:` after `snapcraft pack` — not run yet: vm-second was run locally against the source tree (14 ok)
