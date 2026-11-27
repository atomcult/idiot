# Tasks

## 1. idiot-store: snap ID and type from info

- [x] 1.1 Add `type` to `INFO_FIELDS`, and make `idiot-store info` print a first `snap\t<name>\t<snap-id>` line and a seventh `type` field on each `channel` line. Record an anonymous fixture of `/v2/snaps/info/pc-kernel?fields=…,type` under `tools/idiot-store/src/testdata/store/` (a public snap: no account data in it). Verify: unit tests parse the ID and the types from it; `idiot store channels pc-kernel --tsv` and `idiot store check pc-kernel` print the same as before the change.
- [x] 1.2 Run `cargo test` and `make check`. Commit: `feat(idiot-store): snap ID and type in info`.

## 2. idiot-helper: drafts

- [x] 2.1 `idiot-helper draft write <type>` and `draft read <file|->`. Implement the record format in design.md (`h`, `l`, `s`, `v`, `f`, `x`, `t`), with every scalar written as a string, the fixed header order and a 2-space indent. Refuse values containing a tab or newline, and refuse signed input. Verify with unit tests: round trips of Canonical's `ubuntu-core-24-amd64.json` and `ubuntu-core-24-pi-arm64.json` (from canonical/models, as test data) give the same JSON object; quotes, backslashes and UTF-8 in `display-name` survive; and a signed `.model` is refused.
- [x] 2.2 `idiot-helper draft check <file|->`: snapd's rules for model, system-user and validation-set (design.md Context), printing `error`/`warning` lines with a `<where>` and exiting 1 on an error. Verify with unit tests built from snapd's `asserts/*_test.go` vectors (one passing and one failing draft per rule), covering every scenario of the spec's "Checking an assertion" requirement, including the 365-day system-user and the format needed by `serials` and `user-presence`.
- [x] 2.3 Run `cargo test` and `make check`, and grep the new code for socket or HTTP use (none). Commit: `feat(idiot-helper): write, read and check assertion drafts`.

  Notes (implementation): the rule tests are snapd's vectors rewritten as JSON drafts in Rust (`draft.rs` tests), one passing and one failing draft per rule, rather than a port of the Go test files. `check` also errors on JSON null and warns when a system-user has neither SSH keys nor a password. The test data from canonical/models is GPL-3.0 (noted in `src/testdata/models/README.md`).

## 3. sh: idiot assertion new and check

- [x] 3.1 `share/assertion.sh`, covering:
  - the per-type header lists;
  - the defaults (account ID from `idiot-store whoami` through `query_cred`, architecture, store, times, model essentials per base and architecture);
  - the `--set`, `--add` and `--snap` parsing, and the terminal check before any network call;
  - prompts on `/dev/tty` with bracketed defaults;
  - the snap loop (fzf `--print-query` over `idiot store snaps --tsv`, the channel picker over `info` lines for the target architecture, filling `id`, `type`, `default-channel` or `revision`, the multi-arch warning, and the dangerous-grade and not-found cases);
  - the editor loop with `draft check` and the no-clobber write.

  Verify from the source tree, anonymously, with `--no-edit` and no terminal: the spec's "Model without prompts" command gives the live IDs, and "Not released for the target" and "Required value missing" fail as specified.
- [x] 3.2 `cmd/assertion/help`, `cmd/assertion/new` and `cmd/assertion/check` (host commands; usage, `-o`, default file names, refusing to overwrite). Verify:
  - each scenario of "Writing an assertion", "Defaults from the environment", "Snaps from the store" and "Model essentials" that needs no account, from the source tree;
  - `check` on the failing drafts from 2.2 exits 1 naming the header;
  - on a terminal, a UC24 amd64 model written with all the defaults passes `check`.
- [x] 3.3 Add `tests/bare-host.d/assertion-writer`: usage errors, the no-terminal failure, `check` on a good and a bad draft (offline), and an anonymous `new model --no-edit -o -` for public snaps. Update the README (a "Writing assertions" section) and the using-idiot skill reference (`skills/using-idiot/references/commands.md`). Run `make check`. Commit: `feat(assertion): write and check unsigned assertions`.

## 4. sh: system users and validation sets

- [x] 4.1 System-user prompts:
  - models from saved model repositories with a matching `brand-id`, and SSH keys from `~/.ssh/*.pub`;
  - the optional password: read twice with echo off (echo restored by a trap), hashed with `openssl passwd -6 -stdin`, the variable unset after;
  - `format` from `serials` and `user-presence`;
  - `umask 077` for drafts and outputs.

  Verify:
  - "Password hashed": the file starts `$6$`, its mode is 0600, `grep` finds no typed password in it, and a `ps` loop during hashing never shows it;
  - "Plain text refused";
  - "Serials need format 1";
  - without openssl on PATH, the hint is shown.
- [x] 4.2 Validation-set sequences: the default sequence from the latest validation set (`idiot-store assertion validation-set account-id=… name=…`, which asks for `sequence=latest`): one more than its sequence, 1 when the store has none (exit 4), and 1 with a note only when the store can't be reached (exit 3), `timestamp` filled, and `revision` left out. Verify with a stubbed fetch in a copied tree (latest sequence 3 → default 4; not found → 1), and live against a public validation set if one is known.
- [x] 4.3 Extend the bare-host check with a system-user draft (mode 0600, the password prompt skipped) and a validation-set draft for a public snap. Update the README and the skill reference. Run `make check`. Commit: `feat(assertion): system users and validation sets`.

## 5. sh: idiot assertion edit

- [x] 5.1 `cmd/assertion/edit`: defaults from `draft read`, the model `revision` bump, the validation-set choice between a new revision and a new sequence (the store's revision, from `idiot-store assertion validation-set … sequence=<n>`, when higher), the `timestamp` update, `--refresh` with `--snap` channels, writing back in place or to `-o`, and refusing signed input. Verify the spec's "Editing an assertion" scenarios from the source tree, and that `edit` on a file from `idiot model pick --unsigned` keeps the fields it doesn't model (`modes`, components) unchanged.
- [x] 5.2 Add `edit` to the group help, the bare-host check (bump and signed-input refusal, offline with `--no-edit`), the README and the skill reference. Run `make check`. Commit: `feat(assertion): edit unsigned assertions`.

  Notes (implementation): groups 3–5 landed in one commit (`feat(assertion): write, check and edit unsigned assertions`): `share/assertion.sh` holds the per-type headers for both `new` and `edit` (a header the records already hold is the prompt's default, so `edit` gets the file's values for free). Verified from the source tree without a credential (so with `--set brand-id=…`/`account-id=…`; the account default itself needs the user's credential, task 7.3): the spec's no-terminal scenarios with live IDs (core24 amd64 `latest/stable` 2307), on a terminal through tmux a UC24 amd64 model with all the defaults (passes `check`) and the editor loop (an error listed, reopened, written only once it passes), the password (hashed `$6$`, mode 0600, absent from the file, the terminal and every idiot or openssl process's argv in a `ps` loop), the openssl-missing hint, and sequences against a stubbed `idiot-store assertion` (latest 3 → 4, not found → 1). No public validation set was known for a live sequence check. A dangerous model's snap the store doesn't show gets its essential role as `type`. Drafts other than system-users get the umask's mode (system-users 0600).

## 6. sh: idiot assertion sign

- [x] 6.1 `cmd/assertion/sign`:
  - `check` first;
  - keys from `snap keys --json`, each key's account from `idiot-store assertion account-key public-key-sha3-384=<sha>` (anonymous; exit 4: not registered), the picker preselecting the `authority-id` match, and `default` without a terminal;
  - the no-keys message (`snap create-key`, `snapcraft register-key`);
  - warnings for an unregistered or foreign key and for a `timestamp` before the key's `since`;
  - `snap sign -k <name> [--chain]` on the terminal, the output written beside the input (`.model` / `.assert`) or to `-o`, no-clobber, 0600 for system-users;
  - the verdict from `idiot-helper assertion --remote` on stderr.

  Verify from the source tree with a throwaway key (`snap create-key` in a scratch `HOME`, never the user's keyring): a draft signs, `idiot inspect assertion` reads the result, and the "Invalid draft", "No keys" and "To stdout" scenarios hold. Also check that a stubbed account-key fetch naming another account gives the "Key for another account" warning.
- [x] 6.2 Extend the bare-host check: in the container, create a key without a passphrase, sign a dangerous model draft, check that the file starts `type: model`, that `snap sign` wasn't given anything but the key name and the file, and that stdout with `-o -` holds only the assertion. Update the README and the skill reference. Run `make check`. Commit: `feat(assertion): sign with snap sign`.

  Notes (implementation): `snap` ignores `HOME` for its keyring; the throwaway keys (source tree and bare-host) live in a private `SNAP_GNUPG_HOME`, made with `gpg --batch` as `snap create-key` makes them (RSA 4096, no passphrase), so the user's keyring was never used. The "Key for another account" warning was checked with a stubbed `idiot-store assertion account-key` in a copied tree. bare-host installs gnupg in the container when it has no gpg (`snap sign` needs it).

## 7. Verification

- [x] 7.1 Run `openspec validate assertion-writer --strict`.
- [x] 7.2 Pack the snap (`snapcraft clean idiot`, pack into a gitignored directory inside the tree) and run `tests/bare-host` on `local:`; all checks pass, including `assertion-writer`.

  Notes: packed from a clone (a worktree's build instance can't see its `.git`). At 9b2bbdf with `--boot download --kvm`: 280 passed, 3 failed (the container has no gpg, and gnupg was installed without `apt-get update`); fixed in 6565070 and rerun without `--boot`: 270 passed, 0 failed, all 20 assertion-writer checks included. 45d1f23 (a lookup-cache fix) was checked with the local copy of the check.
- [ ] 7.3 With the user, under the snap and on a terminal:
  - write a UC24 model for their brand in a brand store they belong to (a private snap's ID filled in with their credential);
  - sign it with their registered key (they type the passphrase);
  - confirm that `idiot inspect assertion --remote` reports it `verified`, and that no output contains any part of a credential or key.
