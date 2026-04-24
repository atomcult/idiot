# 🌱 idiot

**Interactive Devtools for IoT** — a shell environment manager for Snap Store
development. Switch credentials, stores, and architectures with a single
command; everything takes effect in your current shell session.

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

```sh
make install                   # installs to ~/.local by default
make install PREFIX=/usr/local # optional: override install prefix
```

Add shell integration to your rc file (run once):

```sh
eval "$(idiot init bash)"   # or zsh
idiot init fish | source    # fish
```

The `iot` shorthand is also available.

---

## How it works

`idiot` commands output shell code to stdout. The shell wrapper eval's that
output so environment changes land in your current shell, not a subprocess.
All user-visible output goes to stderr and is never eval'd.

---

## Development

```sh
make check    # shfmt format-check + shellcheck
make fmt      # auto-format all shell files
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
