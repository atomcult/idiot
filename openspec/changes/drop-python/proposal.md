# Proposal

## Why

idiot runs `python3` from the host in eight places. That always works on
classic Ubuntu, but on an Ubuntu Core device the host's root filesystem is
the base snap, and Python is being removed from core26. On a Core26 device
`inspect image`, `inspect diff`, `inspect assertion` and `vm vars` would
stop working; `vm stop` would lose its graceful shutdown; and `vm` would
skip its port and swtpm checks. Bundling Python in a component would cost
about 30 MB for roughly 900 lines of code. Nothing idiot does with Python
needs it.

## What Changes

- The small inline uses move to shell and tools idiot already needs:
  - the LUKS header check reads its 6 bytes with `dd` and `od`;
  - the partition table comes from `sfdisk --dump` and `awk` instead of
    `sfdisk --json` and Python.
- Everything else moves into **`idiot-helper`**, a new Rust binary that
  runs host-side and sits next to `idiot-store` in `lib/idiot/libexec/`.
  It has one subcommand per job:
  - `assertion`: today's `share/assertion.py`: the same arguments, report,
    `--facts` file and exit status;
  - `uefi-vars`: today's `share/uefi-vars.py`, with the same listing;
  - `qmp`: today's `share/vm-qmp.py`;
  - `snapd-state`: reads snapd's `state.json` and prints the snaps,
    connections, changes and preseed flag as tab-separated lines. The
    shell formats them for `inspect image` and reads them for
    `inspect diff`;
  - `port-free`: the bind test behind the SSH port choice;
  - `swtpm-shutdown`: sends swtpm its shutdown command over the control
    socket.
- `share/assertion.py`, `share/uefi-vars.py` and `share/vm-qmp.py` are
  deleted, and no command calls `python3` any more.
- `idiot-store` is unchanged and stays a leaf tool that only talks to the
  store.
- `idiot doctor` drops its four `python3` tool checks and checks
  `idiot-helper` the way it checks `idiot-store`.
- Specs that say a command "needs python3" stop saying so. That includes
  inspect's claim that python3 can come from the `core` component, which
  was never true: no component bundles it.

stdout and the eval file are unchanged. Every command prints the same data
on stdout as before, apart from formatting differences that the specs
don't fix (the design lists the ones to check). Nothing new is queued in
the eval file.

**Runtime dependencies:** one is removed (host `python3`) and none is
added. The snap stages `idiot-helper` in the main snap, built by its own
Rust part like `idiot-store`, and patched to run host-side. A source
install already needs cargo for `idiot-store`, and `make install` builds
both binaries.

## Capabilities

### New Capabilities

None. `idiot-helper` is an internal binary; the behaviour it carries is
already specified by the commands that use it.

### Modified Capabilities

- `inspect`: "Image summary" and "Assertion summary" no longer need
  python3.
- `vm`: "Saved UEFI variables" needs no host tool at all.
- `doctor`: "Shell integration and install checks" also reports the
  `idiot-helper` binary.

## Impact

- **New:** `tools/idiot-helper/` (a Rust crate) and an `idiot-helper`
  part in `snap/snapcraft.yaml`.
- **Changed:**
  - `Makefile`: build, install, check and uninstall `idiot-helper`;
  - `cmd/inspect/assertion`, `cmd/inspect/image`, `cmd/inspect/diff`;
  - `share/image.sh`;
  - `cmd/vm`, `share/vm.sh`;
  - `cmd/doctor`;
  - `share/creds.sh`: `store_helper_path` gets a sibling lookup for the
    new binary;
  - `tests/bare-host.d/*`, which gain helper checks;
  - CLAUDE.md, README and `openspec/config.yaml`'s stack line.
- **Deleted:** `share/assertion.py`, `share/uefi-vars.py`,
  `share/vm-qmp.py`.
