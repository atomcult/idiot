# Tasks

## 1. The model group again

- [ ] 1.1 Move the commands back:
  - `git mv cmd/assertion/pick cmd/model/pick`
  - `git mv` each file in `cmd/assertion/repos/` (`list`, `add`, `remove`, `update`, `help`) to `cmd/model/`

  Then remove `cmd/assertion/repos`. Fix each file's `common.sh` depth and its `idiot assertion repos …` / `idiot assertion pick` strings. List `pick` in `model/help` again.
- [ ] 1.2 In `cmd/assertion/help`:
  - drop `pick` and `repos`;
  - add a line naming `idiot model` for models kept in repositories.
- [ ] 1.3 In `share/aliases.sh`, replace the `model pick` and `model` lines with `assertion pick|model pick` and `assertion repos|model`.
- [ ] 1.4 In `idiot_help`, list `model` under "Assertions & inspection".
- [ ] 1.5 Change the hints back in `cmd/` and `share/models.sh`. Verify with `grep -rn "assertion \(pick\|repos\)" cmd share idiot`: only the alias table matches.
- [ ] 1.6 Verify from the source tree:
  - `idiot model help` lists `pick`;
  - `idiot model list`;
  - `idiot assertion repos list` prints the note, then the same output;
  - `idiot assertion pick --help` prints the note, then `model pick`'s help;
  - `idiot assertion repos` prints the note, then the model help;
  - `idiot assertion help` lists neither `pick` nor `repos`.
- [ ] 1.7 Update `README.md`, `skills/using-idiot/` (SKILL.md and its references) and `CLAUDE.md`. Run `make check`. Commit: `refactor(model): model repos and pick are idiot model again`.

## 2. Verification

- [ ] 2.1 In `tests/bare-host.d/aliases`:
  - the alias rows become `assertion pick|model pick`, `assertion repos list|model list` and `assertion repos update|model update`;
  - the stdout check compares `assertion repos list` with `model list`;
  - the stale-help check flags `pick`/`repos` in `assertion help`, not `model` in the main help.

  Run `make check`. Commit: `test(bare-host): model again, assertion repos/pick as aliases`.
- [ ] 2.2 Run `openspec validate model-group --strict`.
- [ ] 2.3 Run `snapcraft pack` and `tests/bare-host --image local:8257342f834e <snap> <comps>` on `local:`. All checks pass. This needs a shell where snaps can run, which the Claude app session can't currently do.
