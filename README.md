# 🌱 idiot

**Interactive Devtools for IoT** — a shell environment manager for Snap Store
development. Switch credentials, stores, and architectures with a single
command; everything takes effect in your current shell session.

```
$ eval "$(idiot init bash)"   # or zsh / fish — do this once in your rc

$ idiot status
  auth    logged in  my-cred · you@example.com
  store   my-store   abc123xyz
  arch    arm64
  lp      yourname
```

---

## Install

```sh
git clone <this repo>
cd idiot
make install          # installs to ~/.local by default
make install PREFIX=/usr/local
```

Add shell integration to your rc file:

```sh
# bash / zsh
eval "$(idiot init bash)"    # or zsh

# fish
idiot init fish | source
```

The `iot` shorthand is included — `iot status` works just as well as `idiot status`.

---

## Commands

### Store & environment

| Command | Description |
|---|---|
| `idiot status` | Show active credential, store, architecture, and LP identity at a glance |
| `idiot auth` | Manage snap store credentials (add, remove, login, logout, import, export) |
| `idiot store` | Save and activate snap store environments |
| `idiot arch` | Set the target build architecture |
| `idiot lp` | Configure your Launchpad username |

### Development

| Command | Description |
|---|---|
| `idiot model` | Register, clone, update, and interactively pick model assertions |
| `idiot kernel` | Clone Ubuntu kernel trees from Launchpad |
| `idiot example` | Manage bundled snap example repositories |
| `idiot inspect` | Inspect snaps, UC images, and snapd change logs |

### VM & device

| Command | Description |
|---|---|
| `idiot vm <image>` | Launch a QEMU VM with UEFI secure boot (amd64, arm64, armhf, riscv64) |
| `idiot vm ssh` | SSH into the running VM |
| `idiot vm scp` | Copy files to/from the running VM (`vm:` is the guest alias) |
| `idiot vm vars` | Inspect UEFI secure boot variables from a saved state directory |
| `idiot vm tpm` | Attach swtpm to a saved TPM state for offline inspection |

### Shell utilities

| Command | Description |
|---|---|
| `idiot run <tool>` | Run a snap-bundled tool, falling back to PATH when not in a snap |
| `idiot run --shell` | Open a shell with snap tools on PATH |
| `idiot clean` | Remove development leftovers |
| `idiot init <shell>` | Print shell integration code for bash, zsh, or fish |
| `idiot version` | Show version and authorship |

---

## How it works

`idiot` commands print `export`/`unset` statements to stdout. The shell
wrapper installed by `idiot init` captures that output and `eval`s it, so
environment changes land in your current shell rather than a subprocess.
User-visible output (progress, errors) always goes to stderr and is never
eval'd.

---

## Nerd Fonts logo

If you use a [Nerd Fonts](https://www.nerdfonts.com/)-patched terminal font,
set `IDIOT_NERD=1` for a fancier logo with the sprout icon in green.

---

## Development

```sh
make check     # shfmt format-check + shellcheck
make fmt       # auto-format all shell files
make uninstall
make purge     # uninstall + delete all user data
```

All shell code is POSIX `sh` — no bashisms. `shfmt` and `shellcheck` are the
only dev dependencies.

---

## License

MIT © Lauren Brock
