# idiot

A shell environment manager for working with the Snap Store. Manages credentials, store overrides, target architecture, and model assertions across shell sessions.

## How it works

`idiot` outputs eval-able shell code to stdout. A shell wrapper function evals that output, so environment changes (exports, unsets) take effect directly in your current shell.

## Installation

```sh
make install          # installs to ~/.local by default
make install PREFIX=/usr/local
```

Add shell integration to your rc file (run once):

```sh
# bash
eval "$(idiot init bash)"

# zsh
eval "$(idiot init zsh)"

# fish
idiot init fish | source
```

## Commands

### `idiot env auth` — credentials

```
login     Activate a credential
logout    Log out
status    Show login status

add       Create a new credential
remove    Remove a saved credential
list      List saved credentials
import    Import a credential from a file
export    Output a credential
```

### `idiot env store` — Dedicated Snap Stores

```
use       Activate a store
unset     Clear the active store
status    Show the active store

add       Save a store
remove    Remove a saved store
list      List saved stores
```

### `idiot env arch` — target architecture

```
set       Set the target architecture
unset     Clear the target architecture
status    Show the active architecture
```

### `idiot models` — model assertions

```
pick      Interactively select a model assertion
update    Fetch the latest model assertions
```

### `idiot inspect` — inspect snap store objects

```
changes   Browse recent snap changes
snap      Print the snapcraft.yaml or snap.yaml from a snap file
```

### `idiot prune` — remove development leftovers

```
all         Remove all snapcraft build containers and unasserted snaps
containers  Remove all snapcraft build containers
snaps       Remove all unasserted snaps
```

All `prune` subcommands accept `-n` / `--dry-run` to preview what would be removed.

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `IDIOT_AUTH_DIR` | `$XDG_DATA_HOME/idiot/auth` | Credentials directory |
| `IDIOT_STORES_DIR` | `$XDG_DATA_HOME/idiot/stores` | Stores directory |
| `IDIOT_MODELS_DIR` | `$XDG_DATA_HOME/idiot/models` | Models repository |
| `IDIOT_CACHE_DIR` | `$XDG_CACHE_HOME/idiot` | Cache directory |
| `UBUNTU_STORE_ARCH` | — | Target architecture override |

## Development

```sh
make check    # run shellcheck on all scripts
make install  # install to $PREFIX (default: ~/.local)
```

Requires: `shellcheck`, `fzf`, `snapcraft`, `lxd`.
