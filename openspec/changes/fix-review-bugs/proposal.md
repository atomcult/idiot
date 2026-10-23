# Proposal

## Why

A review of v4 found bugs that make commands report success after failing,
store credentials with loose permissions, and leave VMs unbootable in common
cross-architecture cases. They are independent of the shell-integration and
packaging work and can land on their own.

## What Changes

- **git failures are reported.** `share/common.sh`'s `git()` wrapper returns
  the status of its trailing `printf`, so every `git … || die` is dead code:
  `idiot kernel` prints "Cloned to …" after a failed clone, and `idiot
  example`/`model update` carry on after failed fetches. The wrapper will
  return git's status.
- **Extracting several example subdirectories stops on the first failure**
  instead of `die` only exiting the pipeline's subshell.
- **auth import/export/add:**
  - `auth import` requires both `<name>` and `<file>` (its check uses `||`)
    and its header comment matches its argument order.
  - Credential names are validated like store names (no `/`, no `..`).
  - Credential files and the auth directory are private (0600/0700),
    including files written by `auth import` and `auth export -f`.
- **vm:**
  - Without KVM, guests get `-cpu max`; today arm64 under TCG gets QEMU's
    32-bit cortex-a15 default and cannot boot.
  - `--no-sb` on arm64/armhf/riscv64 boots plain UEFI firmware instead of no
    firmware.
  - The background swtpm is stopped when the VM exits.
- **BREAKING (clean):** `clean snaps`/`containers`/`all` list what they will
  remove and ask for confirmation (`-y/--yes` skips it); removal errors are
  shown and make the command exit non-zero instead of being discarded.
- **Docs and tooling drift:**
  - CLAUDE.md and STYLE.md describe tabs, `set -e` in the dispatcher, and
    `share/` (not `lib/`) as they are.
  - The Makefile checks every `share/*.sh` by glob.
- stdout is not affected by this change. No new runtime dependencies.

## Capabilities

### New Capabilities
- `repo-cloning`: clone and fetch behaviour of `idiot kernel`, `idiot example` and `idiot model update`, starting with failure reporting.
- `vm`: booting images under QEMU, starting with architecture/firmware selection and TPM lifetime.
- `clean`: removing snapcraft build containers and sideloaded snaps.

### Modified Capabilities
- `auth`: import argument handling, credential name validation, and private file permissions.

## Impact

- `share/common.sh` (`git()`), `cmd/kernel`, `cmd/example`, `cmd/model/update`
- `cmd/auth/{add,import,export}`, `share/creds.sh`
- `cmd/vm`, `share/arch.sh`
- `cmd/clean/{all,containers,snaps}`
- `CLAUDE.md`, `STYLE.md`, `Makefile`
