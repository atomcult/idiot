# Tasks

## 1. Wrapper

- [ ] 1.1 `cmd/init`: eval file directory baked in at generation time (`$XDG_RUNTIME_DIR/snap.<instance>`, created 0700, under `$SNAP`; today's otherwise); read → delete → evaluate; return status rule; verify for bash, zsh and fish with a stub command that exports a value, exits 3, and hands off a command exiting 4 (expect 3 and 4 respectively), and that Ctrl-C during a handed-off `sleep` leaves no eval file
- [ ] 1.2 Verify `env | sort` is identical before and after loading the integration in each shell; commit (`feat(init): evaluate after deleting, return handoff status`)

## 2. Handoff

- [ ] 2.1 `share/common.sh`: `IDIOT_DATA_HOME` default (snap common dir / handed-off / XDG) and every `share/*.sh` default derived from it; `emit_handoff`; verify a source install still reads `~/.local/share/idiot` and `SNAP=… SNAP_USER_COMMON=$tmp` points every directory at `$tmp`
- [ ] 2.2 Dispatcher: hand off marked commands when `$SNAP` is set and `IDIOT_HOST` isn't, except help; verify with `SNAP=/snap/idiot/x1 SNAP_INSTANCE_NAME=idiot` that `idiot vm --arch arm64` writes exactly one quoted `env … '/snap/idiot/current/lib/idiot/idiot' 'vm' '--arch' 'arm64'` line, that `idiot vm --help` writes nothing, and that an argument containing `'` and `$` round-trips in bash and fish
- [ ] 2.3 Add `# idiot: host` to each host command in design.md's table; verify `grep -L` over `cmd/` leaves only confined commands, and grep confined commands for tool, network or file-argument use (none but fzf)
- [ ] 2.4 `snap_app` host-side lookup via `IDIOT_SNAP` (snap, then component), per-process setup only; `cmd/run --shell` PATH-only child shell; verify `env` inside `idiot run --shell` vs outside differ only in PATH
- [ ] 2.5 Update CLAUDE.md and STYLE.md (the rule, the marker, never touch PATH/LD_LIBRARY_PATH in the user's shell, host commands can't export); run `make check` and commit (`feat: hand host commands to the user's shell`)
