# Tasks

## 1. The helper's skeleton and packaging

- [x] 1.1 `tools/idiot-helper`: a crate with hand-rolled subcommand dispatch (no clap: `assertion` must keep `assertion.py`'s exact arguments and errors; empty stubs exiting 2), release profile with `strip`, `lto` and `panic = "abort"`, and a `Cargo.lock`. Verify that `cargo build --release` produces `target/release/idiot-helper` and that `idiot-helper --help` lists the six subcommands
- [x] 1.2 Makefile: build, install (`lib/idiot/libexec/idiot-helper`) and uninstall it beside `idiot-store`; `make check` runs `cargo fmt --check` and `clippy` on both crates. Verify with `make install PREFIX=<scratch>`, then `make uninstall PREFIX=<scratch>` leaving nothing behind, and `make check` passing
- [x] 1.3 `share/common.sh`: `helper_path <name>` and `helper <name>` (libexec, then the crate's `target/release`, then PATH; `helper` dies with `<name> not found — build it with: make install`). `store_helper_path` and `store_helper` become wrappers around them. Verify that `idiot auth status` still finds `idiot-store`, and that `helper idiot-helper` finds the new binary from source and from a scratch install
- [x] 1.4 snapcraft.yaml: an `idiot-helper` part mirroring `idiot-store`'s (pull-tracked, the pinned rust-channel, `enable-patchelf`, organised to `lib/idiot/libexec/`). Verify that `snapcraft pack` puts `lib/idiot/libexec/idiot-helper` in the snap with core24's interpreter (`patchelf --print-interpreter`; checked with `snapcraft prime idiot-helper`, whose patchelf step set `/snap/core24/current/lib64/ld-linux-x86-64.so.2`; the full pack is 8.2)
- [x] 1.5 `make check` passes; commit (`build(helper): add the idiot-helper crate`)

## 2. Shell instead of Python in image.sh

- [x] 2.1 `share/image.sh`: `image_partitions` uses `sfdisk --dump` with `awk`, and the LUKS check uses `dd`/`od`. Verify that `sector-size`, `partitions.tsv` and `first-boot` are byte-identical to the Python version's for a fresh and a booted Core 24 image, plus a LUKS header written into a scratch image at ubuntu-data's offset (`encrypted`)
- [x] 2.2 `make check` passes; commit (`refactor(inspect): read partitions and LUKS headers without python`)

## 3. snapd-state

- [x] 3.1 `idiot-helper snapd-state`: the records from the design (`preseed`, `snap`, `conn`) and `--report snaps|connections|changes`, with unit tests on a small `state.json` that has a failed change with ERROR log lines and tabs inside values. Verify with `cargo test`
- [x] 3.2 `share/image.sh` (preseed), `cmd/inspect/image` (installed snaps, connections and changes from `--report`) and `cmd/inspect/diff` (`installed.tsv`, `connection.tsv`) use it. Verify that `inspect image` output (stderr and the exit code) and `inspect diff` output are byte-identical to the Python versions on a fresh and a booted image, including one whose state has a failed change and non-ASCII summaries
- [x] 3.3 Remove python3 from `inspect image`'s and `inspect diff`'s `require`s and from the doctor tool table. Verify that `idiot inspect image` runs with python3 hidden (`unshare -rm` with an empty file bind-mounted over `/usr/bin/python3`)
- [x] 3.4 `make check` passes; commit (`refactor(inspect): read snapd state with idiot-helper`)

## 4. Assertions

- [x] 4.1 `idiot-helper assertion`: port `share/assertion.py` (parser, stream decoding, SHA3-384 key IDs, PKCS#1 v1.5 over SHA-512 with the whole block compared, the key chain from stream, `snap known`, `--trusted` and `--remote`, summaries, `OUT_*` colours, `--facts`, exit codes). Unit tests cover a canonical account-key chain verifying, a flipped signature bit, a wrong key and a truncated body. Verify with `cargo test`
- [x] 4.2 Compare the two byte for byte (stdout, stderr, exit code, and the `--facts` file) on: a Core 24 model, `snap known account-key` for canonical, a serial, a system-user with a password hash, a snap-declaration, a snap-revision, a validation-set, a stream of several of these, corrupted copies (a bad signature, a truncated body), JSON input, `--no-verify`, `--remote`, without `snap`, with and without colours. Every case must be identical
- [x] 4.3 `cmd/inspect/assertion` runs `idiot-helper assertion`; delete `share/assertion.py`; drop its python3 `require` and doctor row. Verify that `tests/bare-host.d/assertion` passes from source, and that a model inspects with python3 hidden
- [x] 4.4 CLAUDE.md (layout: `assertion.py` gone, the `tools/idiot-helper/` entry). Run `make check`; commit (`refactor(inspect): verify assertions with idiot-helper`)

## 5. UEFI variables

- [x] 5.1 `idiot-helper uefi-vars`: port `share/uefi-vars.py` with a unit test on a small synthetic variable store (one X.509 and one SHA-256 list). Verify with `cargo test`
- [x] 5.2 Compare the two byte for byte on a fresh OVMF secure-boot vars file, an OVMF plain one, an AAVMF one and one from a booted Core VM, plus a file that isn't a store (same error and exit code)
- [x] 5.3 `cmd/vm`'s `vars` runs it; delete `share/uefi-vars.py`; drop its python3 `require` and doctor row; update CLAUDE.md. Verify that `idiot vm vars <state-dir>` lists the variables with python3 hidden. Run `make check`; commit (`refactor(vm): read UEFI variables with idiot-helper`)

## 6. Sockets: QMP, ports and swtpm

- [ ] 6.1 `idiot-helper qmp`, `port-free` and `swtpm-shutdown`, with unit tests: `qmp` against a fake QMP server on a temporary unix socket (greeting, capabilities, a return and an error), `port-free` on a port bound by the test and on a free one. Verify with `cargo test`
- [ ] 6.2 `cmd/vm` (`_qmp`, the "needs python3" message removed) and `share/vm.sh` (`vm_port_free`, the swtpm stop) use them, falling back as today when the helper is missing; delete `share/vm-qmp.py`. Verify with python3 hidden:
  - a booted Core VM stopped with `idiot vm stop` powers down through ACPI;
  - `--ssh-port` on a port that `nc -l` holds is refused before qemu starts;
  - after `vm stop` of a `--tpm` VM, no swtpm process remains.
- [ ] 6.3 Run `grep -rn python3 cmd share idiot` and check the only remaining hits are comments about gdb's Python, if any. Update CLAUDE.md (layout, helper list) and `openspec/config.yaml`'s stack line. Run `make check`; commit (`refactor(vm): talk to qemu and swtpm with idiot-helper`)

## 7. doctor

- [ ] 7.1 `cmd/doctor`: report `idiot-helper` beside `idiot-store` (`✓` with its path, or `✗` naming the commands that need it and `make install`). Verify from source: healthy, and with the binary moved away (`✗`, exit 1)
- [ ] 7.2 `tests/bare-host.d/doctor`: expect the `idiot-helper` line. README's Troubleshooting sample shows it. Run `make check`; commit (`feat(doctor): check idiot-helper`)

## 8. Integration

- [ ] 8.1 `tests/bare-host.d/no-python` (and a step in `tests/bare-host`): with python3 hidden in the container, `inspect image`, `inspect diff`, `inspect assertion`, `vm vars`, `vm --ssh-port` on a taken port, and `vm stop` of a `--tpm` VM behave as with python3. Run `make check`; commit (`test: bare-host without python3`)
- [ ] 8.2 `snapcraft pack`; `snap/local/check-components` passes; `tests/bare-host --boot download --kvm <snap>` on `local:` passes in full
