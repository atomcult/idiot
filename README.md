# 🌱 idiot

**Interactive Devtools for IoT** — a shell environment manager for Snap Store
development. Switch credentials, stores, and architectures with a single
command; everything takes effect in your current shell session.

```
$ idiot status
  auth    logged in  my-cred · you@example.com
  store   my-store   abc123xyz
  arch    arm64
  lp      yourname
```

---

## Install

```sh
make install          # installs to ~/.local by default
make install PREFIX=/usr/local
```

Add shell integration to your rc file (run once):

```sh
eval "$(idiot init bash)"   # or zsh
idiot init fish | source    # fish
```

The `iot` shorthand is also available.

---

## How it works

`idiot` commands print `export`/`unset` statements to stdout. The shell
wrapper eval's that output so environment changes land in your current shell,
not a subprocess. All user-visible output goes to stderr and is never eval'd.

---

## Development

```sh
make check    # shfmt format-check + shellcheck
make fmt      # auto-format all shell files
```

---

## License

MIT © Lauren Brock
