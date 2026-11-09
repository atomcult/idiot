# Tasks

## 1. Wrapper

- [ ] 1.1 `cmd/init`: eval file directory baked in at generation time (`$XDG_RUNTIME_DIR/snap.<instance>`, created 0700, under `$SNAP`; today's otherwise); read → delete → evaluate; return status rule; verify for bash, zsh and fish with a stub command that exports a value, exits 3, and hands off a command exiting 4 (expect 3 and 4 respectively), and that Ctrl-C during a handed-off `sleep` leaves no eval file
- [ ] 1.2 `cmd/init` under `$SNAP`: export `IDIOT_SNAP`, `IDIOT_DATA_HOME`, `IDIOT_CACHE_DIR` (never `SNAP`, PATH or LD_LIBRARY_PATH); verify in each shell that, after loading, `env` differs from before only by `IDIOT_*` variables (snap) or `IDIOT_SHELL` (source); commit (`feat(init): evaluate after deleting, return handoff status`)

## 2. Handoff

- [ ] 2.1 `share/common.sh`: `IDIOT_DATA_HOME` default (snap common dir / init's export / XDG) and every `share/*.sh` default derived from it; `emit_handoff`; verify a source install still reads `~/.local/share/idiot` and `SNAP=… SNAP_USER_COMMON=$tmp` points every directory at `$tmp`
- [ ] 2.2 Dispatcher: hand off marked commands when `$SNAP` is set, except help; host side refuses without `IDIOT_SNAP`; verify with `SNAP=/snap/idiot/x1 SNAP_INSTANCE_NAME=idiot` that `idiot vm --arch arm64` writes exactly the line `'/snap/idiot/current/lib/idiot/idiot' 'vm' '--arch' 'arm64'`, that `idiot vm --help` writes nothing, and that an argument containing `'` and `$` round-trips in bash and fish
- [ ] 2.3 Add `# idiot: host` to each host command in design.md's table; verify `grep -L` over `cmd/` leaves only confined commands, and grep confined commands for tool, network or file-argument use (none but fzf)
- [ ] 2.4 `snap_app` host-side lookup via `IDIOT_SNAP` (snap, then component), PATH/LD_LIBRARY_PATH set as needed for that process only; `cmd/run --shell` child shell; verify the calling shell's PATH and LD_LIBRARY_PATH are unchanged afterwards, and a bundled fzf's preview `sh` sees no LD_LIBRARY_PATH
- [ ] 2.5 Update CLAUDE.md and STYLE.md (the rule, the marker, what `init` exports, PATH/LD_LIBRARY_PATH only per process, host commands can't export); run `make check` and commit (`feat: hand host commands to the user's shell`)
