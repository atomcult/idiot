# Proposal

## Why

idiot's whole job is to change the current shell's store, credential and
arch, but nothing shows that state until the user runs `idiot status`, so
it is easy to push to the wrong store or as the wrong account. A compact
segment in the shell prompt (`staging·arm64·work`) makes the active state
visible all the time. Prompts redraw on every command, so the segment has
to cost next to nothing: under the snap a single `idiot` invocation takes
about 120 ms (snap-confine), far too slow to run per prompt.

## What Changes

- New `idiot prompt [--format <fields>] [--separator <sep>]`: prints the
  active store, arch and credential (and optionally the Launchpad user and
  active profile) joined by `·` to stdout, followed by a newline, or prints
  nothing when nothing is active. It reads environment variables only (no
  network, no state files), works without the shell integration, and does
  not bump the usage counter. Runs confined.
- The shell wrapper from `idiot init` keeps an exported `IDIOT_PROMPT`
  variable equal to the default segment: computed from environment
  variables in the wrapper itself (no process started), at init and after
  every `idiot` call, and unset when the segment is empty. Prompts
  (starship's `env_var` module, bash `PS1`, zsh `PROMPT`, fish) read the
  variable instead of running anything.
- `idiot store use` also exports `IDIOT_ACTIVE_STORE` (the saved store's
  name), and `idiot store unset` unsets it, so the segment can show the
  name without reading the stores directory. Same pattern as
  `IDIOT_ACTIVE_CRED`.
- README: copy-paste snippets for starship, bash, zsh and fish.

stdout: `idiot prompt` is new and writes its segment to stdout (data).
The eval file: `store use` additionally queues
`export IDIOT_ACTIVE_STORE=…`, `store unset` an unset of it; the
integration printed by `idiot init` (stdout of `init`, which the user
evals) gains the `IDIOT_PROMPT` bookkeeping. No other command's output
changes. No new runtime dependencies; nothing new for the snap to stage.

## Capabilities

### New Capabilities
- `prompt`: the prompt segment, its `idiot prompt` command and format
  options, and the `IDIOT_PROMPT` variable the wrapper maintains.

### Modified Capabilities
- `store`: `store use`/`store unset` also set/clear `IDIOT_ACTIVE_STORE`.
- `shell-integration`: `prompt` joins the commands that run without the
  wrapper; the source-install integration may export `IDIOT_PROMPT`.
- `status`: `prompt` does not increment the usage counter.

## Impact

- New: `cmd/prompt`.
- Changed: `idiot` (dispatch `prompt` like `status`: before the
  `IDIOT_SHELL` check, no `_increment_count`; top-level help),
  `cmd/init` (prompt bookkeeping in the bash/zsh and fish wrappers),
  `cmd/store/use`, `cmd/store/unset`, README, STYLE.md (the list of
  commands that run without shell initialisation), CLAUDE.md if helpers
  change.
- Overlaps with the in-flight `add-env-profiles` change, which moves the
  `store use`/`unset` emit logic into `share/stores.sh` and adds
  `IDIOT_ACTIVE_PROFILE`: whichever lands second puts the
  `IDIOT_ACTIVE_STORE` export into the shared emitter. The `profile` field
  is empty until that change lands.
