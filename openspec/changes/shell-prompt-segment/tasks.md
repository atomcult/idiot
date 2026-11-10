# Tasks

## 1. Store name in the environment

- [ ] 1.1 `cmd/store/use`: also `emit_export IDIOT_ACTIVE_STORE "${name}"`; `cmd/store/unset`: also `emit_unset IDIOT_ACTIVE_STORE` (if `add-env-profiles` has landed, do this in its shared store emitters instead); verify through the wrapper in bash and fish that `idiot store use staging` sets both variables and `idiot store unset` clears both
- [ ] 1.2 Update the store group help if it lists environment variables; run `make check` and commit (`feat(store): export the active store's name`)

## 2. `idiot prompt`

- [ ] 2.1 Add `cmd/prompt` (header, `main()`, help with the fields and options): build the segment from the fields in `--format` (default `store,arch,cred`; also `lp`, `profile`), join non-empty ones with `--separator` (default `·`), print with a newline only when non-empty; unknown field → `die` naming it and the known fields; verify every scenario in the prompt spec's "Prompt segment" and "Segment format" requirements, including `idiot prompt | wc -c` printing `0` with nothing set
- [ ] 2.2 `idiot`: dispatch `prompt` next to `status` (before the `IDIOT_SHELL` check, no `_increment_count`) and list it in the top-level help under Shell; verify `env -u IDIOT_SHELL ./idiot prompt` works and that 100 calls leave `invoke-count` unchanged
- [ ] 2.3 Under the snap (`snapcraft pack`, `snap install --dangerous`), verify `idiot prompt` runs confined (no handoff line in the eval file) and prints the same as from source
- [ ] 2.4 STYLE.md: add `prompt` to the commands that work without shell initialisation; run `make check` and commit (`feat(prompt): idiot prompt segment`)

## 3. `IDIOT_PROMPT` in the wrapper

- [ ] 3.1 `cmd/init`: add the forkless segment function to the bash/zsh and fish integrations, call it once at load and at the end of `idiot()` (save the return status first, return it after); comment each copy with where the others live; verify `eval "$(./idiot init bash)"`, `zsh -c`, and `./idiot init fish | source` all define it and that the "exit status preserved" scenario in shell-integration still holds (a command exiting 3 returns 3)
- [ ] 3.2 Consistency check: a script under the scratch dir that, for each of bash, zsh and fish, sets every combination of store id/name, arch, named/unnamed credential, and nothing, runs a no-op `idiot` call, and compares `$IDIOT_PROMPT` with `idiot prompt`; all must match and `IDIOT_PROMPT` must be unset in the empty case
- [ ] 3.3 Verify "No process per prompt": `strace -f -e trace=execve bash -i` with the bash snippet in PS1 shows no execve when pressing Enter on an empty line
- [ ] 3.4 README: a "Prompt" section with the starship (`env_var` and the `custom` fallback with its measured cost), bash, zsh and fish snippets from design.md; try each snippet in its shell (and starship if installed) and check it shows `staging·arm64·work` after the matching commands
- [ ] 3.5 Run `make check` and commit (`feat(init): keep IDIOT_PROMPT for shell prompts`)

## 4. Integration

- [ ] 4.1 Walk every scenario in the change's prompt, store, shell-integration and status specs once more from a source install and from the snap, then run `make check`
