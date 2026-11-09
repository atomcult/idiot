# Design

## Context

Findings come from a read-through of v4 plus one reproduction: sourcing
`share/common.sh` and running a failing `git clone` returns 0, because the
`git()` wrapper's last command is `printf '%s' "${RESET}" >&2`. Firmware
paths were checked against Ubuntu's `ovmf`/`qemu-efi-aarch64` packages on
the dev host (`AAVMF_CODE.no-secboot.fd`, `AAVMF_VARS.fd`, `QEMU_EFI.fd`,
`OVMF_CODE_4M.fd`, `OVMF_VARS_4M.fd`).

## Goals / Non-Goals

**Goals:** make failure visible, make credential storage private, make
cross-arch VMs boot, and make destructive cleanup deliberate.

**Non-Goals:**
- Reworking `vm` beyond CPU, firmware and TPM lifetime (e.g. disk snapshots;
  the image is still attached read-write).
- Changing which containers/snaps `clean` targets.

## Decisions

### `git()` keeps the dim colouring and returns git's status
```sh
git() {
    printf '%s' "${DIM}" >&2
    command git "$@"
    _git_rc=$?
    printf '%s' "${RESET}" >&2
    return "${_git_rc}"
}
```
Callers' existing `|| die` / `|| say warning` then work as written. Loops
that update many repos (`model update`, `example update`) collect a
`failed=1` flag and `exit 1` after the loop. Alternative: drop the wrapper
and colour nothing; rejected, the dim git output is a deliberate style
choice.

### Pipelines that must stop use a here-document loop
`printf '%s\n' "${subdirs}" | while …; do … die; done` runs `die` in a
subshell. Replace with `while …; done <<EOF\n${subdirs}\nEOF` so `die`
exits the command. Same pattern for `example remove --all` and `drop --all`
where per-item failures should surface.

### Credential names follow the store-name rule, plus dots
`[A-Za-z0-9._-]`, not starting with `.` (so `..` and hidden files are out,
and `/` can never appear). Dots are allowed beyond the store rule because
names like `work.ci` are natural for credentials. A helper
`validate_cred_name` in `share/creds.sh` is shared by `add` and `import`.
Existing files with other names keep working for `login`/`remove`.

### Permissions are set explicitly
`umask 077` at the top of `auth add/import/export` and `mkdir -m 700`, then
`install -m 600 "${src}" "${dest}"` instead of `cp` (so the source's mode is
not copied). Existing files are left alone; the migration is a one-off
`chmod` the user can run, mentioned in the commit message.

### VM CPU and firmware
- No KVM → `-cpu max` for every architecture. QEMU's `max` is the most
  capable model it can emulate; on arm64 it is 64-bit, fixing the
  cortex-a15 default. Alternative: per-arch named CPUs (`cortex-a57`), which
  would need maintaining.
- `setup_firmware` takes `sb`: secure-boot paths as today; non-SB paths
  `OVMF_CODE_4M.fd`/`OVMF_VARS_4M.fd`, `AAVMF_CODE.no-secboot.fd` (falling
  back to `AAVMF_CODE.fd`)/`AAVMF_VARS.fd`, `AAVMF32_CODE.fd`/
  `AAVMF32_VARS.fd`, and the riscv64 files. `qemu-efi-aarch64/QEMU_EFI.fd`
  is not a fallback: it is 2 MiB, and the virt machine needs 64 MiB pflash
  images. Firmware is used for `--no-sb` on amd64 too (as the spec
  requires; Ubuntu Core images need UEFI); only i386 keeps QEMU's BIOS.
- swtpm is started with `--pid file="${workdir}/swtpm.pid"`; an EXIT trap
  (`_cleanup`, installed with or without `--state-dir`) kills that pid and
  removes a temporary workdir, and INT/TERM/HUP traps turn signals into
  exits so it runs. A failed swtpm start now dies (it was ignored, and
  QEMU then failed too); the usual cause is a socket path over the 107-byte
  UNIX limit with a long `--state-dir`. `vm` exits with QEMU's status (it
  used to exit 1 after a normal run without `--state-dir`).

### clean asks on the terminal
The prompt reads from `/dev/tty` (stdin may be a pipe, and the wrapper
captures nothing from stdin anyway). Default is no. `clean all` asks once
for the combined list rather than twice. Errors: drop `>/dev/null 2>&1`,
keep stderr, track a `failed` flag.

## Risks / Trade-offs

- [`clean` now blocks scripts that relied on it being unattended] → `--yes`;
  called out as BREAKING in the proposal.
- [`-cpu max` under TCG is slower than a named model in some QEMU versions]
  → acceptable for a dev tool; KVM path unchanged.
- [AAVMF file names differ across Ubuntu releases] → keep the fallback list
  and the "install package X" error.
