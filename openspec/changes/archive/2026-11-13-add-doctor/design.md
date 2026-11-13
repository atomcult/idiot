# Design

## Context

See proposal.md for motivation. The facts `doctor` needs already have
helpers, each used by one command and dying on failure:

- integration: `require_wrapper` (`IDIOT_EVAL_FILE` set, a writable file),
  the `IDIOT_SHELL` check in `idiot`/`dispatch_subcmd`, and `common.sh`'s
  refusal to run snap files host-side without `IDIOT_SNAP`;
- tools: `tool_path` (host first, then `snap_tool` over
  `snap_tool_dirs`), `tool_component` (the `share/components/<name>`
  indexes), `missing_tool` (the hint text);
- components: `snap_component_dirs`, and `cmd/run`'s private
  `_list_components` (installed vs available, from the indexes);
- firmware: `share/vm-firmware.sh`'s table (`vm_firmware`,
  `vm_firmware_package`, `vm_firmware_secboot`) and `cmd/vm`'s private
  `_find_qemu`/`_find_firmware`, which decide whether the host's or the
  `vm` component's files are searched depending on which qemu runs;
- KVM: `cmd/vm` only tests `[ -e /dev/kvm ]` for the host arch;
- credentials: `share/creds.sh` (`IDIOT_AUTH_DIR`, `store_helper`), modes
  enforced by `ensure_auth_dir` and `install -m 600`;
- state paths: `common.sh` (`IDIOT_DATA_HOME`, `IDIOT_CACHE_DIR`), the
  eval-file directory chosen in `cmd/init`.

Confined, the snap sees core24 as `/`, not the host's `/usr/share`,
`PATH`, `~/.local` or (with no interfaces) `/dev/kvm`. Host-side commands
are reached through `emit_handoff`, which needs the eval file; the
host-side process itself runs *after* the wrapper has read and deleted
the eval file, so it never has a usable `IDIOT_EVAL_FILE`.

## Goals / Non-Goals

**Goals:**
- One read-only, offline command covering the failure causes the specs
  already describe, reusing the same lookup code the commands use so the
  report cannot disagree with what the command would do.
- A useful answer even when the integration itself is what's broken.

**Non-Goals:**
- Fixing anything (`--fix`). Every hint is a command the user runs. A
  later change can add fixes for the safe ones (chmod, cleaning eval
  files).
- Network checks (store reachability, `whoami`): slow, and
  `idiot auth status` already does it.
- Machine-readable output (`--json`); can be added later.

## Decisions

### A host command, with a confined fallback
`cmd/doctor` carries `# idiot: host`: the checks that matter most
(`/dev/kvm`, host tools on `PATH`, `/usr/share/OVMF`, `~/.local/share/idiot`)
are invisible from inside confinement. `idiot` dispatches `doctor` before
its `IDIOT_SHELL` check:

- integration loaded and current → the normal path (`_increment_count`,
  `run_cmd`, i.e. a handoff under the snap, direct from source);
- otherwise → `exec cmd/doctor` directly. From source that is already on
  the host, so the report is complete apart from the integration being
  `✗`. Under the snap it runs confined, reports the integration `✗`,
  prints the install section from what it can see, and marks every
  host-only section `! skipped — load the shell integration first`.

How `cmd/doctor` judges the integration:
- `SNAP` set (confined): it only gets here when the dispatcher could not
  hand off, so `IDIOT_SHELL` missing → "not loaded", else "out of date".
- `IDIOT_SNAP` set, `SNAP` unset (host side of the snap): reaching here
  took a handoff, which needs a current wrapper → `✓`. (Running
  `/snap/idiot/current/lib/idiot/idiot doctor` by hand would also say `✓`;
  accepted.)
- source install: `IDIOT_SHELL` and the `require_wrapper` test, without
  dying.

Alternative: a confined command. It would report `/dev/kvm` missing and
every host tool missing on every machine; useless. Alternative: two
commands (confined `doctor` + host `doctor --host`): more surface for the
same result as the handoff.

### Reuse, not reimplementation
- Move `_list_components` from `cmd/run` to `common.sh` as
  `snap_component_table` (same tab-separated output); `run --list` prints
  it, doctor formats it. `run --list` output is unchanged.
- Move `_find_qemu`'s lookup and `_find_firmware` from `cmd/vm` into
  `share/vm-firmware.sh` (`vm_find_qemu <arch>` setting the same
  variables without dying, `vm_find_firmware <arch> <sb> <code|vars>`),
  so doctor searches exactly the roots `idiot vm` would. `cmd/vm` keeps
  its error messages (`_no_firmware`, `missing_tool`).
- Non-dying variants where a helper dies today: doctor calls `tool_path`
  and `tool_component` (both just fail), and builds hints with the same
  wording as `missing_tool`; `store_helper` gets a sibling
  `store_helper_path` that fails instead of dying (and `store_helper` uses
  it).

### The tool table
One table in `cmd/doctor`, `command|tool|host package hint`, taken from
the commands' current `require`/`tool_path` calls:

| Command | Tools |
|---|---|
| `vm` | `qemu-system-<host qemu arch>`; `ssh` (`vm ssh`); `python3` (`vm vars`) |
| `vm --tpm` | `swtpm` |
| `inspect image` | `sfdisk`, `python3`, `debugfs`, `mcopy` |
| `inspect snap` | `unsquashfs` |
| `inspect changes` | `snap`, `fzf` |
| `kernel`, `example` | `git`, `fzf` |
| `clean` | `lxc`, `snap` |

Each tool is checked once (shared tools listed with every command that
needs them). A missing tool is `!`, never `✗`: each one disables a
feature, not idiot. qemu for other architectures is probed (no line when
absent) only to decide which other architectures' firmware to check.

### Severity rules
`✗` only for: no/outdated integration; `IDIOT_SNAP` unusable host-side;
missing or failing `idiot-store`; credentials directory or files
accessible by group/other (exposed secrets); `IDIOT_ACTIVE_CRED` naming a
removed credential; an expired *active* credential (with
`credential-expiry`). Everything else that is not fine is `!`. Exit 1 iff
any `✗`.

### Credential checks without exposure
Modes via `stat -c %a` on paths (no contents read). The "active credential
differs from its saved file" check compares in-shell
(`[ "${SNAPCRAFT_STORE_CREDENTIALS}" = "$(cat "${file}")" ]`): the
credential only passes through a command substitution, never argv, a
pipe to another tool, or output. Expiry checks read only the
`.expires.<name>` records from `credential-expiry`, via its
`read_expiry`/`expiry_phrase` helpers, and are skipped (no lines) when
`creds.sh` has no `read_expiry` (that change not landed). Doctor does not
write missing records (read-only); it points at `idiot auth status`.

### Data directories
- Snap host side: the source data home
  (`${XDG_DATA_HOME:-~/.local/share}/idiot`) holding a non-empty `auth`,
  `stores` or `models` → `!` with
  `cp -a ~/.local/share/idiot/. ~/snap/<instance>/common/`; a source
  install's files (`~/.local/bin/idiot`, `~/.local/lib/idiot`) → `!` "two
  installs" in the install section.
- Source: `~/snap/<instance>/common` (instance `idiot`) holding state → the
  mirror-image `!`; `/snap/bin/idiot` present → "two installs".
- `~/snap/<instance>/<revision>/` (not `common`, not `current`) holding
  `auth`, `stores` or `models` → `!` (state from an old per-revision
  layout).
- Eval files: `idiot.??????` owned by the user, older than a day, in the
  directory `cmd/init` uses (snap: `${XDG_RUNTIME_DIR}/snap.<instance>`;
  source: `${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}`) → `!` with the count and
  a `find … -delete` command.

### Output
Sections as `${BOLD}<Name>${RESET}` lines, checks indented two spaces:
`${GREEN}✓${RESET} <what>  ${DIM}<detail>${RESET}`,
`${YELLOW}!${RESET} <what> — <hint>`, `${RED}✗${RESET} <what> — <hint>`;
the symbols stay when colour is off. Closing line after a blank line:
`2 problems, 3 warnings` / `No problems found`. All via `say`, nothing on
stdout. `--quiet` buffers each section's lines and prints the heading
only if it has a `!` or `✗`; the count only if anything was printed.

### Counting
`doctor` counts like any dispatched command: the usage-counter rule
exempts only help, version, status, init (and prompt, once
`shell-prompt-segment` lands), and doctor is run by hand, not per prompt.
In the confined fallback (no usable integration) it is not counted, as
nothing else is in that state. With
`credential-expiry` landed it therefore also triggers the once-a-day
expiry warning, which is harmless next to its own expiry lines.

## Risks / Trade-offs

- [Tool table drifts from the commands' `require` calls] → a task greps
  `cmd/` for `require`/`tool_path` and compares with the table; a comment
  at the table points at the commands.
- [Refactoring `cmd/vm`/`cmd/run` breaks them] → the moved helpers keep
  their outputs; tasks re-run the vm and bundled-tools scenarios that
  cover them.
- [False alarms on unusual but intentional setups] (custom
  `IDIOT_DATA_HOME`, a store ID set by hand) → those are `!`, not `✗`, and
  never affect the exit status.
- [Host-side `✓` for the integration without a real check] → documented
  above; the only way to get there without a current wrapper is running
  the snap's internal script by hand.
- [Merge with `shell-prompt-segment`] → both modify "Wrapper required for
  commands"; the later archive lists both `prompt` and `doctor`.

## Migration Plan

Additive; the two refactors are behaviour-preserving. Rollback is
removing `cmd/doctor` and its dispatcher case; the moved helpers can stay.

## Assumptions

- Missing tools/components are warnings (`!`), never failures.
- Loose credential permissions are failures (`✗`), even though every
  command still works.
- Doctor is strictly read-only and offline; no `--fix`, `--json` or store
  checks in this change.
- Only the host architecture's firmware is always checked; other
  architectures only when their qemu is available.
- Stale eval files are those older than one day.
