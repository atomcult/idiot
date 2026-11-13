# Tasks

## 1. Shared lookups

- [x] 1.1 Move `cmd/run`'s `_list_components` to `share/common.sh` as `snap_component_table` and use it in `run --list`; verify `idiot run --list` output is byte-identical before and after under the snap (with only `core` installed) and from source
- [x] 1.2 Move `cmd/vm`'s qemu lookup and `_find_firmware` into `share/vm-firmware.sh` (`vm_find_qemu`, `vm_find_firmware`, non-dying), keeping `cmd/vm`'s error messages; verify `idiot vm` still boots with host qemu and with the `vm` component only (bare-host `vm` check, `tests/bare-host --boot`), and that the missing-firmware errors are unchanged
- [x] 1.3 `share/creds.sh`: add `store_helper_path` (fails instead of dying) and make `store_helper` use it; verify `auth add --help` and `auth status` behave as before; update CLAUDE.md's helper lists; run `make check` and commit (`refactor: share component, firmware and helper lookups`)

## 2. `idiot doctor`

- [x] 2.1 Add `cmd/doctor` (`# idiot: host`, help with `--quiet`): result helpers (`_ok`, `_warn`, `_fail`, section buffering for `--quiet`, the closing count, exit 1 on any `✗`) and the integration/install section (judged as in design.md); verify "Healthy setup", "Nothing on stdout", "No integration" and "Two installs" from source
- [x] 2.2 `idiot`: dispatch `doctor` before the `IDIOT_SHELL` check (current wrapper → count and `run_cmd`; otherwise `exec` directly) and add it to the top-level help; under the snap verify "Outdated integration under the snap" (confined, host sections skipped) and that a current wrapper hands it off
- [x] 2.3 Tools and components section from the tool table in design.md; verify by grepping `cmd/` for `require`/`tool_path`/`missing_tool` that every required tool is in the table, and check "Tool from a component", "Missing tool" and "Source install" (bare-host LXD container with and without `idiot+vm`/`idiot+tpm`)
- [x] 2.4 VM section (`/dev/kvm` access, firmware for the host arch with and without secure boot and for other arches with qemu); verify "KVM not accessible" (a user outside `kvm`) and "Firmware missing" in the bare-host container, and that the files reported are the ones `idiot vm` then boots
- [x] 2.5 Credentials and stores section (modes, active credential present/matching, store files, `UBUNTU_STORE_ID` match; expiry lines only when `read_expiry` exists); verify "Loose permissions", "Active credential removed" and, if `credential-expiry` has landed, "Expired active credential"; check with `strace -f -e trace=execve` that no credential appears in any argv
- [x] 2.6 Data directories section; verify "Data left behind after switching to the snap" and "Stale eval files" (touch `-d '2 days ago'` three fake eval files), and "Read-only and offline" (`unshare -rn`, and `find "$IDIOT_DATA_HOME" "$IDIOT_CACHE_DIR" -newer <stamp>` lists only `invoke-count` afterwards)
- [x] 2.7 Verify `--quiet` ("Quiet and healthy", "Quiet with a problem"); README (a "Troubleshooting" section introducing `idiot doctor` with a sample report), STYLE.md (doctor runs without shell initialisation, limited), CLAUDE.md (layout); run `make check` and commit (`feat(doctor): idiot doctor`)

## 3. Integration

- [x] 3.1 Add a `doctor` check to `tests/bare-host.d/` (report runs, exit status 0 with all components and a kvm-less container gives only `!`), run `tests/bare-host` on a fresh build, walk every scenario in the change's doctor and shell-integration specs once more, and run `make check`; commit (`test: bare-host doctor check`)

## Notes

- 1.1: `run --list` from source is byte-identical; the moved body differs
  from `_list_components` only in its name and variable prefix, and the
  bare-host `run` check (seven components, installed/available) passes.
- 1.2: the missing-qemu error is unchanged (compared before and after);
  bare-host with `--boot download --kvm` boots Ubuntu Core from the vm and
  tpm components, and the vm check now also checks qemu's arguments
  carry the secure boot firmware `idiot doctor` reports.
- 2.3: the tool table also covers `inspect diff`, `inspect assertion`,
  `vm --tpm`, `vm ssh` (ssh), `vm` (qemu-img) and `example --subdir`
  (nnn), from the grep of `require`/`tool_path`/`missing_tool`. Tools are
  looked up with common.sh's `git()` function out of the way (it shadows
  `command -v git`; it runs PATH's git, which is what doctor reports).
- 2.5: only credential files and the directory are `✗` when loose; the
  `.store.*`/`.expires.*` records beside them hold no secret and are `!`.
  The first bare-host run caught a 644 `.store.ci` the profile test wrote
  by hand; the test now writes it as idiot does.
- 2.6/2.7: from source, 24 scenarios pass in a user namespace (to hide
  `/snap/bin` for a healthy setup); `unshare -n` run changes only
  `invoke-count`; strace shows no credential in any argv.
- 3.1: bare-host 196/199 then 189/189 after the fixes (the second run
  without `--boot`: no vm code changed). `lxc launch` stalled once on the
  remote image server; launching from the cached image's fingerprint
  (`--image local:<fp>`) avoids it.
