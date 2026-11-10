# Tasks

## 1. VM state helpers

- [x] 1.1 New `share/vm.sh`: `IDIOT_VM_DIR="${IDIOT_VM_DIR:-${IDIOT_DATA_HOME}/vm}"`, `vm_name_from_image`, `vm_validate_name` (1–32 of `[A-Za-z0-9._-]`, no leading `.`/`-`), `vm_dir <name>` (follows a `--state-dir` symlink), `vm_conf_get`/`vm_conf_set` (line-based `key=value`, never sourced), `vm_running <name>` (pid file + `kill -0` + `/proc/<pid>/cmdline` names `qemu-system-`), `vm_names`, `vm_running_names`; verify from a scratch `IDIOT_DATA_HOME`: `ubuntu-core-24-amd64.img` → `ubuntu-core-24-amd64`, `../x`, `-x` and a 33-character name are rejected, a value with spaces and `$` round-trips through `vm_conf_set`/`get`, and a pid file holding a live non-qemu pid (`sleep`) reads as stopped
- [x] 1.2 `vm_resolve [<name>]` (unknown name → names `idiot vm list`; none running → error; several → fzf picker on `/dev/tty`, else error listing names and `--name`) and `vm_pick_ssh_port <name> [<requested>]` (recorded, 8022, 8023–8122; not another running VM's; python3 bind check when python3 exists); verify with two fake running entries (pid files of `qemu-system-x86_64`-named `sleep` copies) and `python3 -m http.server 8022` holding 8022
- [x] 1.3 `share/vm-qmp.py <socket> <command> [<json-args>]` (stdlib: greeting, `qmp_capabilities`, one command, print `return` as JSON on stdout, exit 1 on `error` with its `desc` on stderr); verify against a `qemu-system-x86_64 -qmp unix:…,server=on,wait=off -S -display none` with `query-status` and `quit`
- [x] 1.4 Run `make check` and commit (`feat(vm): state helpers for named VMs`)

## 2. Named launch with overlay, ports and serial log

- [ ] 2.1 `cmd/vm` `do_run`: `--name`, `--in-place`, image optional when the VM exists; create the state dir (umask 077), write `vm.conf` (image abs path, `stat -c '%s %Y'`, arch, sb, tpm, gfx, in_place, ports); fail on a different image/arch/sb/in-place naming `idiot vm delete <name>`; fail when already running (naming its port); `--state-dir` symlinks `${IDIOT_VM_DIR}/<name>` to the absolute dir and adopts a dir without `vm.conf`; drop the `mktemp` workdir; verify the "Named VMs" scenarios with an Ubuntu Core 24 amd64 image (two VMs from one image in two terminals; boot again by name; different image/arch; invalid name)
- [ ] 2.2 Overlay: `qemu-img create -f qcow2 -b <abs image> -F raw disk.qcow2` (via `tool_path qemu-img`, `missing_tool` naming the vm component), boot `format=qcow2`; `--in-place` boots the raw image; refuse a changed base naming `idiot vm reset <name>`; verify `sha256sum core.img` is unchanged after a first boot, `--in-place` then `idiot inspect image core.img` shows `First boot: complete`, and replacing the image triggers the error
- [ ] 2.3 Ports and QMP/pid: `vm_pick_ssh_port`, written to `vm.conf` before qemu starts, said when it differs from last time; `-pidfile`, `-qmp unix:…,server=on,wait=off`; check socket path lengths (≤107 bytes) up front naming `--state-dir`; verify the "SSH port per VM" scenarios (second VM gets 8023, `--ssh-port` on a busy port fails before qemu)
- [ ] 2.4 Serial: replace `-nographic`/`-serial mon:stdio` with the muxed stdio chardev + `logfile=serial.log`, `-display none` unless `--gfx`; rotate `serial.log` → `serial.log.1` before boot; verify Ctrl-a x and Ctrl-a c still work on amd64 and `--arch arm64`, the log has the boot output, the file is 0600, and a password typed at the login prompt is not in it
- [ ] 2.5 `tests/bare-host.d/vm`: drop the image copy (the overlay leaves it untouched) and check the image's checksum is unchanged, `serial.log` exists in the state dir, and the swtpm/TPM checks still pass; run `tests/bare-host --boot download --kvm` on amd64
- [ ] 2.6 Update help and README's vm section (names, state location, overlay vs `--in-place`, ports, the breaking change); run `make check` and commit (`feat(vm)!: named VMs with a disk overlay`)

## 3. list, stop, reset, delete, console-log

- [ ] 3.1 `do_list`: header + one line per VM (name, image, arch, running/stopped, port) on stdout, "no VMs" on stderr; verify the "Listing VMs" scenarios including a SIGKILLed qemu and `| grep running`
- [ ] 3.2 `do_stop [--timeout N] [--force] [<name>]`: `system_powerdown`, poll, `quit`, TERM, KILL, then swtpm from `swtpm.pid`; SIGTERM path without python3; verify with Ubuntu Core (clean shutdown, launching terminal returns, no swtpm left) and with a guest stuck in firmware (`--timeout 5`), and with python3 hidden from PATH
- [ ] 3.3 `do_reset <name>` (refuse running; remove disk.qcow2, fw_vars.fd, tpm2-*, serial logs; `--in-place` says the image can't be reset) and `do_delete [--yes] <name>` (refuse running; confirm on `/dev/tty`; `--state-dir` VMs: remove idiot's files and the dir only if empty, and the symlink); verify "Freshly flashed" (install mode runs again after reset), running refusals, and that delete leaves the image and a `--state-dir` directory's foreign files alone
- [ ] 3.4 `do_console_log [-f] [--previous] [<name>]` (stdout; `-f` is `tail -f --pid`); verify both "Serial console log" scenarios and that `-f` exits when the VM stops
- [ ] 3.5 Help for each subcommand and README; run `make check` and commit (`feat(vm): list, stop, reset, delete and console-log`)

## 4. Targeting a named VM from ssh, scp, vars, tpm

- [ ] 4.1 `do_ssh`/`do_scp`: `--name`, else `vm_resolve`; port from `vm.conf`; `--port` bypasses resolution; verify the "Choosing a VM" scenarios (one running on 8023; two running with `</dev/null` fails listing both; picker with two in a terminal; `scp --name a ./pc.snap vm:`)
- [ ] 4.2 `do_vars [--name <n> | <state-dir>]` and `do_tpm [--name <n> | <state-dir>]` (tpm refuses a running VM and requires a name or dir); verify every "Saved UEFI variables" scenario still holds plus "Named VM", and `idiot vm tpm --name a` on a stopped VM serves `tpm2_pcrread`
- [ ] 4.3 Help and README; run `make check` and commit (`feat(vm): ssh, scp, vars and tpm target a named VM`)

## 5. Integration

- [ ] 5.1 Walk every scenario in the change's vm spec once more on the snap (host side, components only, in the bare LXD container) and from a source checkout, and run `make check`
