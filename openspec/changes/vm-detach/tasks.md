# Tasks

## 1. vm boot --detach

- [ ] 1.1 Parse `--detach` in `do_run` (both `idiot vm boot` and `idiot vm <image>`); add it to the help and the usage line; don't record it in `vm.conf`
- [ ] 1.2 Detached: `-daemonize`, the serial console to `-chardev file,…,path=serial.log` and no `-mon`; foreground flags unchanged
- [ ] 1.3 `_cleanup` leaves swtpm running once a detached QEMU has started; stops it as now when QEMU fails
- [ ] 1.4 On success say the name and ssh port and name `idiot vm logs --console -f --name <n>` and `idiot vm stop <n>`; nothing on stdout
- [ ] 1.5 `make check`; commit `feat(vm): boot --detach`

## 2. Docs and tests

- [ ] 2.1 README: a `--detach` line in the vm examples
- [ ] 2.2 `skills/using-idiot/SKILL.md`: recommend `--detach` over backgrounding `idiot vm boot` by hand
- [ ] 2.3 `tests/bare-host.d/vm`: a detached `--tpm` boot returns, `vm list` shows it running, the console log fills, `vm stop` leaves no swtpm; a detached boot QEMU refuses fails and leaves no swtpm
- [ ] 2.4 `make check`; commit `docs: vm boot --detach` and `test(bare-host): vm boot --detach`
- [ ] 2.5 Run `tests/bare-host --image local:8257342f834e <snap> <comps>` on `local:` after `snapcraft pack`
