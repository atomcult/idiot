# Design

## Context

See proposal.md for motivation. What a prompt hook would cost today,
measured on an amd64 laptop (dash as `/bin/sh`, 100 runs each):

| Invocation | Per call |
|---|---|
| `sh -c :` (a bare fork + exec) | 0.9 ms |
| `sh -c '. share/common.sh'` | 2.4 ms |
| `./idiot version --short` (source checkout) | 7.3 ms |
| `./idiot status` (source checkout) | 13 ms |
| `/snap/bin/idiot version --short` (snap) | 119 ms |

Where the source-install time goes: the dispatcher sources `common.sh`,
which forks for `cat invoke-count` and, on a terminal, for eleven
`$(printf '\033[…m')` colour variables; then it `exec`s the leaf command,
which sources `common.sh` again (and any `include`d modules). Under the
snap almost all of the 119 ms is `snap run` and snap-confine setting up
the sandbox before idiot's code starts. That makes any design that runs
`idiot` per prompt a non-starter for snap users, whatever idiot itself
does.

The values a segment needs are all environment variables already
(`UBUNTU_STORE_ID`, `UBUNTU_STORE_ARCH`, `IDIOT_ACTIVE_CRED`,
`UBUNTU_STORE_AUTH`/`SNAPCRAFT_STORE_CREDENTIALS`, `IDIOT_LP_USER`), except
the store's *name*: `idiot status` finds it by scanning
`IDIOT_STORES_DIR` for the matching ID. Every change to those variables
made through idiot passes through the wrapper function `idiot init`
prints, which evaluates the eval file after each call.

## Goals / Non-Goals

**Goals:**
- A prompt segment that costs zero process spawns per prompt in bash, zsh,
  fish and starship.
- One definition of the segment, with `idiot prompt` as the reference that
  the wrapper's copy is checked against.

**Non-Goals:**
- Colours or icons in the segment: the prompt framework styles it
  (starship `style`, the user's own escapes).
- Credential expiry, VM state or anything needing files or the network in
  the segment (expiry lands with `credential-expiry`, which can add a field
  later if it exports an env var).
- Installing anything into the user's prompt configuration automatically.

## Decisions

### The wrapper computes `IDIOT_PROMPT`; `idiot prompt` is the reference
`idiot init` adds a small function to the integration (`_idiot_prompt`
in bash/zsh, `__idiot_prompt` in fish) that builds the default segment
from environment variables with builtins only (`[`/`test`, parameter
expansion, `set -q`) and exports `IDIOT_PROMPT`, or unsets it when the
segment is empty. The integration calls it once when loaded and at the end
of every `idiot` call, after evaluating the eval file. The `idiot` function
keeps its current return-status rule (the command's status if non-zero,
else the evaluation's), saved before the update and returned after it.

Prompts then read a variable: bash `PS1`, zsh `PROMPT` (with
`PROMPT_SUBST`), fish `$IDIOT_PROMPT`, starship's `env_var` module. No
process per prompt, under the snap or not. It is exported so starship (a
child process) can see it.

Alternatives considered:
- *Run `idiot prompt` per prompt* (e.g. a starship `custom` module): 7 ms
  per redraw from source, ~120 ms under the snap. Kept as the documented
  fallback for prompt systems that can only run commands, with the cost
  stated in the README.
- *Recompute on every prompt* (bash `PROMPT_COMMAND`, zsh `precmd`, a fish
  `fish_prompt` event): also forkless and would catch variables changed by
  hand immediately, but it means editing the user's prompt hooks, which
  starship and other frameworks also manage. Recomputing after each
  `idiot` call covers every change idiot makes; a change made by hand
  shows up after the next `idiot` call (or right away via `idiot prompt`).
- *Have commands queue an `IDIOT_PROMPT` export in the eval file*: only
  env-changing commands would refresh it, every such command (and the
  shared emitters `add-env-profiles` introduces) would have to remember
  to, and manual changes would never be picked up. The wrapper sees every
  call.

The cost is that the segment logic exists three times: `cmd/prompt` (POSIX
sh), the bash/zsh wrapper and the fish wrapper. It is about ten lines
each, and a task checks all three against each other.

### `IDIOT_ACTIVE_STORE` for the store name
`store use` exports the saved store's name as `IDIOT_ACTIVE_STORE` next to
`UBUNTU_STORE_ID`; `store unset` unsets both. This mirrors
`IDIOT_ACTIVE_CRED` and keeps the segment free of file reads. The name is
shown only while `UBUNTU_STORE_ID` is also set; otherwise the raw ID (or
nothing). If the user sets `UBUNTU_STORE_ID` by hand to another store's ID,
the stale name is shown, the same trade-off `IDIOT_ACTIVE_CRED` already
makes. `idiot status` keeps its own lookup through `IDIOT_STORES_DIR`.

### `idiot prompt` dispatch
`idiot` dispatches `prompt` like `status`: before the `IDIOT_SHELL` check
and without `_increment_count` (a prompt hook would otherwise level the
user up thousands of times a day). It is not a host command, so under the
snap it runs confined; it needs no state. The leaf follows the usual
`cmd/` structure (sources `common.sh`); trimming `common.sh`'s start-up
cost is not worth breaking that convention, since the recommended path
does not run idiot at all.

### Fields and format
`--format` takes a comma-separated field list rather than a printf-style
template, because the main need is dropping empty fields cleanly
(`staging·work`, not `staging··work`); a template can be added later if
asked for. Fields: `store`, `arch`, `cred` (default, in that order), plus
`lp` and `profile`. `arch` appears only when `UBUNTU_STORE_ARCH` is set: the
host default is noise in a prompt. `lp` reads only `IDIOT_LP_USER`, not the
saved `lp-user` file, so it stays file-free (and a saved default is
persistent, not per-shell state). `profile` reads `IDIOT_ACTIVE_PROFILE`
from `add-env-profiles` and is simply empty until that lands. Separator
`·` (U+00B7), overridable with `--separator`. Output is plain text, one
line, newline-terminated; nothing at all when empty, so
`[ -n "$(idiot prompt)" ]` works.

### Snippets (for the README)

starship (`~/.config/starship.toml`); the default `format` includes
`$env_var`, otherwise add `${env_var.IDIOT_PROMPT}`:

```toml
[env_var.IDIOT_PROMPT]
format = "[$env_value]($style) "
style = "bold yellow"
```

starship, command-based fallback (runs idiot per prompt: ~7 ms from
source, ~120 ms under the snap):

```toml
[custom.idiot]
command = "idiot prompt"
when = true
format = "[$output]($style) "
style = "bold yellow"
```

bash (`~/.bashrc`, after the `idiot init` line; single quotes so it
expands at each prompt):

```sh
PS1='${IDIOT_PROMPT:+[$IDIOT_PROMPT] }'"${PS1}"
```

zsh (`~/.zshrc`):

```sh
setopt PROMPT_SUBST
PROMPT='${IDIOT_PROMPT:+[$IDIOT_PROMPT] }'"${PROMPT}"
```

fish (`~/.config/fish/functions/fish_right_prompt.fish`):

```fish
function fish_right_prompt
    set -q IDIOT_PROMPT; and echo -n "[$IDIOT_PROMPT]"
end
```

## Risks / Trade-offs

- [Three copies of the segment logic drift] → a task compares
  `idiot prompt` with `$IDIOT_PROMPT` in bash, zsh and fish over a matrix
  of set/unset variables; keep each copy next to a comment naming the
  others.
- [`IDIOT_PROMPT` stale after a variable is changed by hand] → documented;
  refreshed by the next `idiot` call; `idiot prompt` is always live.
- [`·` garbled in a non-UTF-8 terminal] → `--separator` for `idiot prompt`;
  users of `$IDIOT_PROMPT` can substitute it in their prompt. Not worth a
  config variable until someone asks.
- [Shells that loaded the old integration] → no `IDIOT_PROMPT` until they
  re-run `idiot init`; nothing breaks (the snippets use `${…:+…}`).
- [Conflicts with `add-env-profiles`] → both touch `store use`/`unset`;
  whichever lands second moves the `IDIOT_ACTIVE_STORE` export into the
  shared emitter.

## Migration Plan

Additive. Existing shells keep working; `IDIOT_PROMPT` appears after
reloading the integration. Rollback is removing the command and the
wrapper function; an exported `IDIOT_ACTIVE_STORE` left behind is inert.

## Assumptions

- `--format` is a field list, not a template (see Decisions).
- Default fields are `store,arch,cred`; `lp` and `profile` are opt-in.
- The host-default arch is not shown.
- `IDIOT_PROMPT` always uses the default fields and `·`; customising it
  means using `idiot prompt` instead.
- Exporting `IDIOT_PROMPT` and `IDIOT_ACTIVE_STORE` (non-secret names) is
  acceptable; nothing about credentials beyond the credential's name
  reaches the segment.
