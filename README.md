# 🌱 idiot

**Interactive Devtools for IoT** — a shell environment manager and introspection
tool for Snap and Core development. Switch credentials, stores, and architectures
with a single command; everything takes effect in your current shell session.

```
$ idiot store use staging
$ idiot auth login work-creds
$ idiot status
  auth    logged in  work-creds · you@example.com
  store   staging    abc123xyz
  arch    arm64
  lp      yourname
```

---

## Install

From source:

```sh
make install                   # installs to ~/.local by default
make install PREFIX=/usr/local # optional: override install prefix
```

This builds `idiot-store`, idiot's Snap Store client, so it needs a Rust
toolchain (`cargo`). `idiot auth` doesn't need snapcraft: `idiot auth add`
logs in itself and can scope the credential
(`--permissions`, `--snaps`, `--channels`, `--expires`).

Or as a snap (strictly confined, no interfaces to connect):

```sh
snap install idiot
snap install idiot+core   # optional: e2fsprogs, mtools, util-linux and tpm2
                          # tools for image inspection, mainly on Ubuntu Core
```

The snap keeps its state in `~/snap/idiot/common`, separate from a source
install's `~/.local/share/idiot`. To carry credentials, stores and models
over from a source install, copy them once:

```sh
cp -a ~/.local/share/idiot/. ~/snap/idiot/common/
```

Add shell integration to your rc file (run once):

```sh
eval "$(idiot init bash)"   # or zsh
idiot init fish | source    # fish
```

The `iot` shorthand is also available.

---

## How it works

The shell wrapper hands each `idiot` command a private temp file; commands
that change your environment write `export`/`unset` lines there, and the
wrapper reads, deletes and evaluates the file afterwards, so the changes land
in your current shell, not a subprocess. Nothing a command prints is ever
eval'd: stdout and stderr reach your terminal or pipe as-is
(`idiot auth export work | …`, `idiot run …`).

Under the snap, commands that only manage idiot's own state (`auth login`,
`store use`, `arch`, …) run confined. Commands that need host tools, the
network or your files (`vm`, `run`, `inspect`, `auth add`, …) are handed
back to your shell through the same file and run there, as you, from the
snap's files; that's why the snap needs no interfaces. Your `PATH` and
`LD_LIBRARY_PATH` are never changed: a tool bundled in the snap gets what it
needs in its own process only. After a snap refresh nothing needs reloading.

After upgrading from a version that eval'd stdout, reload the integration
once in already-open shells (`eval "$(idiot init bash)"`); idiot tells you
when it's needed.

---

## Development

```sh
make check    # shfmt + shellcheck, cargo fmt --check + clippy
make fmt      # auto-format shell and Rust files
cargo test --manifest-path tools/idiot-store/Cargo.toml   # helper unit tests (offline)
make purge    # uninstall + delete all user data
```

---

## Contributing

```sh
make hooks    # wire up git hooks (run once after cloning)
```

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/).
The `pre-commit` hook runs `make check`; `commit-msg` enforces the format.

---

## License

MIT © Lauren Brock
