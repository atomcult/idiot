# Proposal

## Why

The shell wrapper evaluates everything a command writes to stdout, so stdout
cannot carry ordinary output. That breaks commands whose job is to print
something, and in one case is dangerous: `idiot run <tool>` evaluates the
tool's output as shell code, so `idiot run debugfs -R 'cat …' image.img`
executes file contents from an untrusted image in the user's shell. `idiot
auth export` without `-f` evaluates the credential, which bash then echoes
back in a "command not found" error. `idiot run --shell` captures the whole
interactive session. `model pick -o -` and `version --short` work around it by
writing to stderr even though they are documented as stdout.

## What Changes

- **BREAKING (shell integration):** Commands hand shell code to the wrapper
  through a private per-invocation file instead of stdout. Users must reload
  the integration (`eval "$(idiot init bash)"` / `idiot init fish | source`,
  which new shells do automatically); a stale wrapper is detected and the
  command refuses to run with that instruction.
- stdout becomes an ordinary output stream, passed through to the terminal
  or pipe and never evaluated.
- `idiot run <tool>` and `idiot run --shell` pass the tool's stdout through
  unmodified.
- `idiot auth export` (no `-f`), `idiot model pick -o -` and
  `idiot version --short` write to stdout as documented, so they work in
  pipes and redirections.
- This change alters what commands write to stdout: env-setting commands
  stop writing to it; printing commands start.
- No new runtime dependencies (uses `mktemp`, already in coreutils).

## Capabilities

### New Capabilities

### Modified Capabilities
- `shell-integration`: the wrapper's contract moves from "eval stdout" to
  "eval a private file"; stdout becomes plain output; outdated wrappers are
  rejected.

## Impact

- `cmd/init` (generated wrapper for bash/zsh and fish), `share/common.sh`
  (export/unset emitters, wrapper check), every command that emits exports
  (`auth login/logout`, `store use/unset`, `arch set/unset`, `lp set/unset`),
  `idiot` and `dispatch_subcmd` (wrapper check).
- `cmd/run`, `cmd/auth/export`, `cmd/model/pick`, `cmd/version`: write to
  stdout.
- CLAUDE.md and STYLE.md "Output discipline" / "Eval-able output" sections.
- Users with an open shell must re-run `idiot init` once.
- `native-store-login` builds on this contract.
