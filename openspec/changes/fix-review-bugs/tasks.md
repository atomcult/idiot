# Tasks

## 1. Git failures

- [x] 1.1 Make `git()` in `share/common.sh` return git's status; verify `sh -c '. ./share/common.sh; git clone https://invalid.example/x /nonexistent/x; echo $?'` prints a non-zero status
- [x] 1.2 In `cmd/model/update` and `cmd/example` (`update`, `remove --all`, `drop --all`, multi-subdir extraction) collect per-item failures and exit non-zero at the end; replace pipe-fed `while` loops that call `die` with here-document loops; verify `idiot kernel noble --dest /nonexistent/x` (or with networking off) exits non-zero without "Cloned to", and `idiot example <alias> --subdir` with a missing second subdir exits non-zero naming it
- [x] 1.3 Run `make check` and commit (`fix: report git failures`)

## 2. Credentials

- [x] 2.1 Add `validate_cred_name` to `share/creds.sh`; use it in `auth add` and `auth import`; fix `auth import`'s argument check (`&&`) and header comment; verify `idiot auth import ci` and `idiot auth import ../x ./f` both fail and write nothing
- [x] 2.2 Create the auth dir 0700 and write credentials 0600 (`umask 077`, `install -m 600`) in `auth add`, `auth import` and `auth export -f`; verify with `stat -c %a` after importing a 0664 file and after `export -f`
- [x] 2.3 Run `make check` and commit (`fix(auth): validate names and keep credentials private`)

## 3. VM

- [x] 3.1 Use `-cpu max` when not using KVM; verify `idiot vm --arch arm64 <arm64 Core image>` on an amd64 host reaches the UEFI/GRUB output on the console
- [x] 3.2 Give `setup_firmware` non-secure-boot paths and use firmware for `--no-sb` on non-x86 arches; verify `idiot vm --arch arm64 --no-sb <image>` starts firmware, and that `idiot vm --arch riscv64 <image>` (firmware not installed on the dev host) fails naming `qemu-efi-riscv64`
- [x] 3.3 Start swtpm with a pid file and kill it from the exit trap (also with `--state-dir`); verify `pgrep swtpm` is empty after a `--tpm` VM is shut down and after it is interrupted with Ctrl-A X
- [x] 3.4 Update `idiot vm --help` if option descriptions change; run `make check` and commit (`fix(vm): boot cross-arch and non-secure-boot guests, stop swtpm`)

## 4. Clean

- [ ] 4.1 Add a `/dev/tty` confirmation (default no) and `-y/--yes` to `clean containers`, `clean snaps` and `clean all` (one combined question); show removal errors and exit non-zero if any item failed; update each help text; verify: declining removes nothing, `--yes` removes without asking, `--dry-run` lists only, and `idiot clean snaps --yes` as a non-root user shows snapd's refusal and exits non-zero
- [ ] 4.2 Run `make check` and commit (`feat(clean)!: confirm before removing`)

## 5. Docs and tooling

- [ ] 5.1 Correct CLAUDE.md (indent style, the dispatcher's `set -e`) and STYLE.md (`lib/` → `share/`, tabs) to match the code and `.editorconfig`/shfmt; verify `grep -n 'lib/common\|4-space' CLAUDE.md STYLE.md` returns nothing stale
- [ ] 5.2 Replace the hand-written `share/*.sh` lists in the Makefile with a glob; verify `make check` lints a new empty `share/tmp.sh` (then delete it)
- [ ] 5.3 Run `make check` and commit (`docs: align CLAUDE.md and STYLE.md with the code`, `build: lint every share module`)
