# Proposal

## Why

When an idiot command fails, the cause is usually environmental and
spread over several places: a shell that loaded an outdated integration,
a missing snap component, no access to `/dev/kvm`, firmware not
installed, a credential file with loose permissions, or data left in the
source install's directory after switching to the snap. Each has its own
error, shown only when the command that needs it runs. `idiot doctor`
checks all of them in one go and says what to do about each.

## What Changes

- New `idiot doctor [--quiet]`: a read-only, offline report on stderr,
  grouped into sections, one line per check marked `✓` (fine), `!`
  (degraded or worth knowing: an optional tool or component missing, slow
  VMs) or `✗` (broken: something idiot needs, or a security problem),
  each `!`/`✗` with a hint. Ends with a count. Exits 1 if anything is
  `✗`, else 0. `--quiet` prints only the `!` and `✗` lines (nothing when
  everything is fine).
- Checks:
  - shell integration: `IDIOT_SHELL`, a usable `IDIOT_EVAL_FILE` (the
    current wrapper), and under the snap `IDIOT_SNAP`/`IDIOT_DATA_HOME`;
  - install: snap (instance, revision, version) or source (root,
    launcher, version); both installed at once; the `idiot-store` helper;
  - tools and components: for the commands that need host tools (`vm`,
    `vm --tpm`, `inspect image`, `inspect snap`, `kernel`/`example`,
    `clean`), whether each tool resolves and to which copy (host or which
    component), and which components are installed or available;
  - VMs: `/dev/kvm` present and accessible; UEFI firmware (secure boot and
    plain) for the host architecture, and for any other architecture whose
    qemu is available, found the way `idiot vm` finds it;
  - credentials and stores: directory and file modes, the active
    credential still saved and matching, saved store files readable, and,
    once `credential-expiry` has landed, expired, expiring and
    unknown-expiry credentials;
  - data directories: source-install data not visible to the snap (or the
    reverse), old per-revision snap directories holding state, stale eval
    files.
- `idiot doctor` is a host command (it has to see the host's `/dev/kvm`,
  `PATH`, `/usr/share` firmware and `~/.local`). Without the shell
  integration (or with an outdated one) it still runs, directly or
  confined under the snap, reports the integration as broken and skips
  the checks that need the host.

stdout: nothing; `doctor` writes only to stderr and queues nothing in the
eval file beyond the usual host handoff under the snap. No other command's
output changes. No new runtime dependencies; nothing new for the snap to
stage.

## Capabilities

### New Capabilities
- `doctor`: the `idiot doctor` report, its checks, severities, exit status
  and quiet mode.

### Modified Capabilities
- `shell-integration`: `doctor` joins the commands that run without the
  shell integration (in its limited form).

## Impact

- New: `cmd/doctor`.
- Refactors shared with existing commands, behaviour unchanged:
  `cmd/run`'s component listing moves into `share/common.sh` (used by
  `run --list` and `doctor`); `cmd/vm`'s qemu and firmware lookup moves
  into `share/vm-firmware.sh` (used by `vm` and `doctor`).
- `idiot` (dispatch and top-level help), README, CLAUDE.md (layout,
  helpers), STYLE.md (commands that run without shell initialisation).
- Optional dependency on `credential-expiry` (expiry checks are added only
  when its expiry record exists). Both this change and
  `shell-prompt-segment` modify the shell-integration "Wrapper required for
  commands" requirement; the one archived second merges the exception list
  (prompt and doctor).
