# Design

## Context

Every use of `python3` today is in a host command, run in the user's
shell, so it is the host's own python3. No component bundles Python. The
uses fall into four kinds:

| Kind | Where | What only Python could do here |
|---|---|---|
| Binary reads | `share/image.sh` (LUKS header), `share/uefi-vars.py` | seek and unpack bytes |
| JSON | `share/image.sh` (`sfdisk --json`, `state.json` preseed), `cmd/inspect/image` (3 report sections), `cmd/inspect/diff` (`installed.tsv`, `connection.tsv`) | parse JSON without jq |
| Sockets | `share/vm-qmp.py`, `share/vm.sh` (port bind test, swtpm `CMD_SHUTDOWN`) | unix and TCP sockets |
| Crypto | `share/assertion.py` (SHA3-384, SHA-512, RSA PKCS#1 v1.5) | hashing and bignum arithmetic |

Three of the uses already degrade when python3 is missing:
- `vm stop` quits qemu instead of shutting the guest down;
- `vm_port_free` assumes the port is free;
- the swtpm stop falls back to signals.

The others `require python3` and die.

`tools/idiot-store` already shows how a Rust binary ships:
- it lives in its own crate under `tools/`;
- `make install` builds it with cargo into `lib/idiot/libexec/`;
- in the snap, a `rust` part pinned to `rust-channel: 1.91.1` builds it
  with `enable-patchelf`, so it loads core24's libc host-side;
- `share/creds.sh` finds it under `$IDIOT_ROOT/libexec`, then in the
  crate's `target/release`, then on `PATH`.

## Goals / Non-Goals

**Goals:**
- No command runs `python3`, and each command's output is the same as
  before the change.
- The degraded paths go away: a graceful `vm stop`, a real port check and
  a clean swtpm stop on every host.
- The shell still owns the user experience wherever it already does.

**Non-Goals:**
- Changing any report's content or layout, error messages or exit codes.
- Moving `idiot-store`'s code, or merging the two crates.
- Running these commands confined. They stay host commands: they read
  host files and run `snap known`, `debugfs` and qemu.
- Python inside bundled tools (gdb's `libpython` in the debug component).

## Decisions

### Shell where the shell is enough
- **The LUKS check** becomes
  `dd if=<img> bs=1 skip=<off> count=6 2>/dev/null | od -An -tx1`,
  compared with `4c 55 4b 53 ba be`. `dd` and `od` are in coreutils,
  which every host has.
- **The partition table** comes from `sfdisk --dump <img>`, parsed with
  `awk`:
  - the `sector-size:` header gives the sector size;
  - each partition line gives `start=`, `size=`, `type=` and `name="…"`;
  - a partition without a name gets `#<n>`, as now.

  The work files `sector-size` and `partitions.tsv` keep their format, so
  `image_offset` and `inspect diff` don't change.

*Alternative:* the helper. Rejected: these are a few lines each with tools
the commands already need (sfdisk is required anyway), and keeping them in
shell means `image.sh` needs no helper for the partition layout.

### A second Rust binary, `idiot-helper`, for the rest
POSIX sh can't open a socket, parse JSON reliably, unpack a binary
variable store or check an RSA signature. The tools that could each be
missing on some host:
- `jq` is only in the debug component;
- `socat` is only in the network component;
- `swtpm_ioctl` comes from `swtpm-tools`, which a host can lack while
  having `swtpm`;
- `ss`'s availability on Core isn't something idiot controls;
- nothing common verifies a snapd assertion offline.

Rust is already a build dependency and already ships host-side, so a
second binary adds no new toolchain, packaging mechanism or runtime
dependency.

**Why a separate binary, not new `idiot-store` subcommands:**
- `idiot-store` handles credentials. CLAUDE.md keeps it a leaf tool that
  only talks to the store, and keeping its code small keeps it easy to
  audit.
- `idiot-helper` never reads, receives or prints a credential, and takes
  no network dependencies (no reqwest).

*Alternative:* one cargo workspace for both crates. Rejected for now: the
`rust` part builds one crate per part, and two parts with separate lock
files match the existing pattern. A workspace can come later if the
duplication hurts.

**Subcommands.** Each reads its arguments and prints data on stdout, errors
on stderr. Exit codes follow `idiot-store`: 0 success, 1 a negative
answer or invalid input, 2 usage or unreadable input.

- `assertion [--remote] [--no-verify] [--snap <path>] [--trusted <file>]… [--facts <file>] [--name <label>] <file|->`
  - It keeps `assertion.py`'s interface exactly:
    - the same report on stdout, coloured from `OUT_*` in the
      environment;
    - the same 0x1f-separated `--facts` lines;
    - the same exit codes (0, 1 invalid, 2 unparsable).

    `cmd/inspect/assertion` changes only the line that runs it.
  - The report stays in the binary instead of moving to shell. It's about
    200 lines of tables and per-type summaries that the shell would have
    to re-parse from an intermediate format, for no gain.
  - It ports the signature check as written: the PKCS#1 v1.5 block is
    rebuilt and compared whole after `pow(s, e, n)`. That keeps its
    behaviour on malformed signatures, which a library's verify call
    might not match.
  - Crates: `sha2`, `sha3`, `base64`, `num-bigint`.
  - `snap known` (for `--snap` and `--remote`) runs through
    `std::process::Command` with the same 60-second timeout.
- `uefi-vars <fw_vars.fd>`
  - It ports `uefi-vars.py` and prints the same listing.
  - It parses the X.509 subjects with a minimal DER walk, as the Python
    does, rather than adding an X.509 crate.
- `qmp <socket> <command> [<json-arguments>]`
  - It ports `vm-qmp.py` with the same exit codes.
  - It reads the greeting, sends `qmp_capabilities`, then the command,
    with a 5-second timeout on each read.
- `snapd-state [--report snaps|connections|changes] <state.json>`
  - By default it prints one tab-separated record per line, sorted as the
    Python sorted them:
    - `preseed\t<yes|no>`
    - `snap\t<name>\t<revision>\t<channel>`
    - `conn\t<plug-snap:plug>\t<slot-snap:slot>`

    Tabs and newlines inside values become spaces; snapd's names never
    contain them.
  - `share/image.sh` reads `preseed`, and `cmd/inspect/diff` builds
    `installed.tsv` and `connection.tsv` from the records with `awk`.
  - With `--report`, it prints that section of `inspect image`'s report
    instead, coloured from `CYAN`, `DIM`, `RESET`, `GREEN`, `YELLOW` and
    `RED`, as the Python snippets did.
  - *Changed during implementation:* the plan was for the shell to format
    these sections with `awk`. It can't do so portably: Python pads by
    characters, but mawk (Ubuntu's default `awk`, and the one on minimal
    and Core hosts) and `printf` count bytes, so a non-ASCII snap name or
    summary would misalign. The report stays in the binary, like
    `assertion`'s.
- `port-free <port>`: exits 0 if it can bind `0.0.0.0:<port>` with
  `SO_REUSEADDR`, which is what qemu's hostfwd will do, and 1 otherwise.
- `swtpm-shutdown <socket>`: sends `CMD_SHUTDOWN` (big-endian 3) on
  swtpm's control socket and waits up to 5 seconds for the reply.

**Finding it.** A new `helper_path` and `helper` in `share/common.sh` do
the lookup for both binaries:
- `helper_path <name>` searches `$IDIOT_ROOT/libexec/<name>`, then
  `$IDIOT_ROOT/tools/<name>/target/release/<name>`, then `PATH`.
- `helper <name>` dies with `<name> not found — build it with: make
  install`.
- `store_helper_path` and `store_helper` in `creds.sh` become thin
  wrappers, so their callers don't change.

Commands call `"$(helper_path idiot-helper)"` and stop on failure. The
paths that degrade today (`vm stop`, `vm_port_free`, the swtpm stop) keep
their fallbacks for a missing helper, so a broken install still stops
VMs.

**Building and shipping:**
- `make install` builds and installs both binaries; `make check` runs
  `cargo fmt --check` and `clippy` on both.
- The snap gets an `idiot-helper` part that mirrors `idiot-store`'s:
  `pull-tracked tools/idiot-helper`, the same pinned toolchain,
  `enable-patchelf`, and organised into `lib/idiot/libexec/`.
- Release builds keep `strip` and `lto`, and add `panic = "abort"`. The
  binary should stay around 1 MB.

### Proving the outputs are unchanged
Before any Python file is deleted, the old and new versions run side by
side on the same inputs and their outputs are diffed byte for byte:
stdout, stderr and the exit code. The inputs:
- **Assertions:** the bare-host fixtures, the bundled canonical keys, a
  serial, a system-user, a snap-declaration and a revision fetched with
  `snap known --remote`, plus corrupted copies (a bad signature, a
  truncated body, JSON).
- **UEFI variables:** a fresh OVMF secure-boot vars file, an AAVMF one,
  and one from a booted Core VM.
- **Images:** a fresh Core 24 image and a booted one. This covers
  `sector-size`, `partitions.tsv`, `preseeded`, `first-boot`,
  `installed.tsv`, `connection.tsv` and the full `inspect image` report,
  including a state with a failed change.
- **QMP, ports and swtpm:** checked by behaviour in bare-host: the guest
  powers down, a port that's in use is refused, and no swtpm is left
  running.

A small scratch script runs the comparison; it isn't committed, since
the Python is gone afterwards.

### Testing without Python
`tests/bare-host` installs the snap in a minimal container. A new check
hides python3 by bind-mounting an empty file over `/usr/bin/python3*`
inside the container. It then runs `inspect image`, `inspect diff`,
`inspect assertion`, `vm vars`, a VM's port selection, and `vm stop` with
`--tpm`, and expects the same results.

## Risks / Trade-offs

- **Formatting drift in the ports.** Python's `f"{x:>{n}}"` and Rust's
  `format!` differ for multi-byte characters: Python pads by code points,
  Rust by characters, which is the same thing for `str`. Width is the
  only likely trap. → The byte-for-byte comparison above, on inputs with
  non-ASCII summaries.
- **Escaping in `sfdisk --dump` names.** It escapes unusual characters in
  names (`\x22`). Core images only use plain names (`ubuntu-seed`,
  `ubuntu-boot`…), and the name is only displayed. → Accepted. A name
  with escapes shows them as `sfdisk` prints them.
- **Signature-check correctness.** A port bug could accept a bad
  signature. → The corrupted-signature fixtures must fail, and a unit
  test checks a known canonical account-key chain, a flipped bit and a
  wrong key.
- **Build time and size.** A second Rust part adds a few minutes per snap
  build and about 1 MB per architecture. → Accepted.
- **Credentials.** `idiot-helper` never handles credentials. The commands
  that call it don't pass any, and it has no network code: `--remote`
  runs `snap known --remote`, as it does today.
