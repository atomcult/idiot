# Tasks

## 1. Aliases and main help

- [x] 1.1 Add `share/aliases.sh`, holding the rename table from design.md and an `alias_note <old> <new>` helper. In `idiot` main, resolve the longest match on the first two words, then on the first word, print the note, rewrite the arguments, then dispatch. `_counted` sees the new words. Verify from the source tree:
  - `idiot store roles --help` prints the note, then the help of the not-yet-moved command;
  - `IDIOT_SHELL` unset still shows help through an alias;
  - an unknown word still fails with `unknown command`.
- [x] 1.2 Rewrite `idiot_help` with the command-layout headings ("Store & environment", "Assertions & inspection", "VM & device", "Get sources", "Setup & tools"), with `version` listed and no `model`. Verify that every entry in `cmd/` appears exactly once in `idiot help`.
- [x] 1.3 Run `make check`. Commit: `feat(dispatch): aliases for renamed commands; regroup main help`.

## 2. One assertion group

- [x] 2.1 `git mv` the commands into `assertion`:
  - `cmd/store/assertion` → `cmd/assertion/fetch`
  - `cmd/inspect/assertion` → `cmd/assertion/show`
  - `cmd/model/pick` → `cmd/assertion/pick`
  - `cmd/model/{list,add,remove,update}` → `cmd/assertion/repos/`
  - `cmd/model/help` → `cmd/assertion/repos/help`

  Fix each moved file's `common.sh` depth and its usage strings. Remove `cmd/model`. Add the `model`, `model pick`, `store assertion` and `inspect assertion` aliases.
- [x] 2.2 Update the group help files:
  - `assertion/help` lists `fetch`, `show`, `pick`, `repos`, `new`, `edit`, `check`, `sign`;
  - `store/help` drops `assertion`;
  - `inspect/help` drops `assertion`.
- [x] 2.3 Rename the hints that name the moved commands in `cmd/`, `share/` (`models.sh`, `assertion.sh`) and `tools/idiot-helper/src/assertion.rs`. Run `cargo test` for idiot-helper. Verify with `grep -rnE "(store|inspect) assertion|model (pick|list|add|remove|update)" cmd share tools/*/src`: the only matches left are the alias table.
- [x] 2.4 Verify from the source tree:
  - `idiot assertion repos list`;
  - `idiot assertion show tests/bare-host.d/brand.model --no-verify`;
  - `idiot model list` (note, then the same output);
  - `idiot assertion pick --help`.
- [x] 2.5 Add `snap-revision <snap> [<channel>]` lookups, per the design:
  - in idiot-store, `snap-channel=` resolves through the info reply's channel map, using the default track and following a closed risk down, and prints the channel and revision it used on stderr;
  - in `cmd/assertion/fetch`, map the words and add the form to the help's Types list.

  Add `cargo test` cases with a recorded channel map: a risk alone, `<track>/<risk>`, a closed risk following down, a missing architecture, and the default track. Verify against the global store:
  - `idiot assertion fetch snap-revision hello-world` and `… hello-world stable` fetch the same revision;
  - `… hello-world 29` still works;
  - `… hello-world latest/stable --arch riscv64` fails naming `idiot store channels`.
- [x] 2.6 Update `README.md`, `skills/using-idiot/SKILL.md`, its `references/` and `CLAUDE.md`. Run `make check`. Commit: `feat(assertion): fetch, show, pick and repos under idiot assertion; snap-revision by channel`.

## 3. Verbs

- [x] 3.1 `git mv cmd/auth/login cmd/auth/use` and `cmd/auth/logout cmd/auth/unset`. Update the usage strings, `auth/help` and `_counted`'s `auth use` check, and add the `auth login` and `auth logout` aliases. Rename the hints in `cmd/` and `share/` (`doctor`, `store check`, `auth bind`, `profile save`, creds.sh). Verify that `grep -rn "auth log\(in\|out\)" cmd share` matches only the alias table.
- [x] 3.2 Add `cmd/profile/unset`, per the profile delta ("Clearing the active profile"), and list it in `profile/help`. Verify through the wrapper from the source tree:
  - after `profile use` of a full profile, every recorded variable and `IDIOT_ACTIVE_PROFILE` are unset;
  - a partial profile leaves `UBUNTU_STORE_ARCH` alone;
  - with none active, the command says so and exits 0.
- [x] 3.3 Update `README.md`, the skill and `CLAUDE.md`. Run `make check`. Commit: `feat(auth,profile): use and unset in every group`.

## 4. Explicit forms and folds

- [x] 4.1 In `cmd/vm`:
  - add `boot` (same as the bare form) and `logs --console [--previous] [-f] [--name <name>]`, which refuses a snap name, `-n`, and `--previous` without `--console`;
  - keep a hidden `console-log` case that prints `alias_note` and maps the positional name;
  - help leads with `vm boot`, and `console-log` is no longer listed.

  Verify `idiot vm boot --help`, `idiot vm logs --console --help`, and the refusals.
- [x] 4.2 In `cmd/example`, add `clone <repo> [options]` and have help lead with it. Verify that `idiot example clone pc-gadget --bare --dest <tmp>` extracts the same files as the bare form.
- [x] 4.3 Merge `cmd/clean/{all,containers,snaps}` into one `cmd/clean` file taking `[containers|snaps] [-n] [-y]`, and add the `clean all` alias. Verify that `idiot clean -n`, `idiot clean containers -n` and `idiot clean all -n` list without removing anything, and that `idiot clean bogus` fails naming `bogus`.
- [x] 4.4 Move `cmd/store/roles`'s code into `cmd/store/list` behind `--account` (with `--tsv` and `--cred`, both refused without `--account`). Under the snap, `--account` hands off with `emit_handoff`. Remove `cmd/store/roles`, add the `store roles` alias, and update `store/help`. Add the confined-or-host pattern to CLAUDE.md. Verify that `idiot store list` still prints on stderr, and that `--tsv` alone fails.
- [x] 4.5 Update `README.md` and the skill. Run `make check`. Commit: `feat(vm,example,clean,store): explicit forms and folded duplicates`.

## 5. Verification

- [ ] 5.1 Move `tests/bare-host.d/*` and `tests/bare-host` to the new names. Add `tests/bare-host.d/aliases`, which checks for each alias:
  - the note is on stderr and names the new form;
  - stdout matches the new command (`store roles --tsv` against `store list --account --tsv`, `model list`, `inspect assertion --no-verify` of `brand.model`);
  - under the snap, the eval file's handoff line carries the new words.

  Also add a help check that every command under `cmd/` appears once in `idiot help`, and that no old name appears in any group's help.
- [ ] 5.2 Run `openspec validate command-reorg --strict`.
- [ ] 5.3 Run `snapcraft clean idiot` and `snapcraft pack`, then `tests/bare-host --image local:8257342f834e <snap> <comps>` on `local:`. All checks pass.
