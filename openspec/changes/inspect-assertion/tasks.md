# Tasks

## 1. stdout colors (`feat(common): add stdout_colors`)

- [ ] 1.1 `share/common.sh`: `stdout_colors` setting `OUT_*` from `[ -t 1 ]` and `NO_COLOR` only, documented in the CLAUDE.md helper list; skip this group if `inspect-image-diff` already added it; verify the variables are empty when stdout is piped and set on a terminal
- [ ] 1.2 Run `make check` and commit

## 2. Parser and summary (`feat(inspect): summarise assertion streams`)

- [ ] 2.1 Collect test inputs in the scratch directory (never in the repo): `snap known account-key`, `snap known model`, the canonical clone's `ubuntu-core-24-amd64.model`, a validation-set and a snap-declaration/snap-revision (`snap known --remote`), a system-user assertion with a password hash (signed with a throwaway `snap create-key` key), a `.json` unsigned model, README.md
- [ ] 2.2 `share/assertion.py` (stdlib only, header like `uefi-vars.py`): stream parser (headers with indented lists/maps, `body-length` bodies, signature, signed-content bytes), per-type summaries from the spec, `OUT_*` colors from the environment, `--facts`, exit 2 on unreadable or unparsable input naming the assertion's position; verify each input from 2.1 prints the spec's fields, the system-user output contains no substring of its hash (`grep -F`), and a truncated stream fails naming the position
- [ ] 2.3 `cmd/inspect/assertion` (`# idiot: host`): help, `<file|->`, missing-file and JSON errors before python, `--no-verify` passed through; verify the A signed model, From stdin, Unsigned model and Not an assertion scenarios, and that `| cat` output has no escape bytes
- [ ] 2.4 `cmd/inspect/help` lists `assertion`; README's inspect section mentions it; verify `idiot inspect` shows it
- [ ] 2.5 Run `make check` and commit

## 3. Signature verification (`feat(inspect): verify assertion signatures`)

- [ ] 3.1 `share/assertions/canonical.account-keys`: `canonical/root` and `canonical/models` from `snap known --remote`, their `public-key-sha3-384` checked against the computed digest; installed by `make install-sh` (it copies `share/`) and so in the snap; verify `make install` puts it under `~/.local/lib/idiot/share/assertions/`
- [ ] 3.2 `assertion.py` verification: OpenPGP v4 public-key and signature packets (RSA, SHA-512), full PKCS#1 v1.5 block comparison, key SHA3-384 check, `account-id` = `authority-id`, `since`/`until`, lookup order stream → `snap known` → bundle → `snap known --remote` (only with `--remote`), chain depth limit 4, results `verified (<account>/<name>)`/`unverified: …`/`invalid: …`, exit 1 when any is invalid; `cmd/inspect/assertion` passes `--snap` when `snap` is on PATH and says on stderr when it isn't
- [ ] 3.3 Verify every Assertion signatures scenario: UC24 model offline on this classic host → verified via bundled `canonical/models` with no network (run under `unshare -n` or with networking off); a brand model (e.g. a customer or `generic` one not in the local DB) → unverified naming the key, then verified with `--remote`; a header edited → invalid and exit 1 with later assertions still printed; a signature byte flipped → invalid; a key presented with the wrong `account-id` and a timestamp before `since` → invalid; `--no-verify` runs no `snap` (PATH shim); `snap` hidden from PATH → bundled keys only and the stderr note; `snap known` output of the local database unchanged before/after (`snap known account-key | sha256sum`)
- [ ] 3.4 Run `make check` and commit

## 4. Cross-checks (`feat(inspect): check models against idiot's stores and saved models`)

- [ ] 4.1 `cmd/inspect/assertion`: from `--facts`, the store check (`UBUNTU_STORE_ID` vs `store`, naming the saved store by `grep -lx` over `IDIOT_STORES_DIR`), the saved-model scan of every `*.model` under the `IDIOT_MODELS_DIR` clones (`find`) (same brand and model: identical via `cmp`, else revisions), and the authority ≠ brand note, all on stderr without changing the exit status; verify the Wrong store active, Saved model is newer (a scratch model repo clone with a bumped-revision copy, `IDIOT_MODELS_DIR` pointed at scratch) and Matches a saved model scenarios, with scratch `IDIOT_DATA_HOME`
- [ ] 4.2 Run `make check` and commit

## 5. Under the snap

- [ ] 5.1 With a locally built snap and `idiot+core`: `idiot inspect assertion pc.model`, `cat pc.model | idiot inspect assertion -` (stdin reaches the handed-off host command) and `--remote` work, stdout stays clean through `| cat`, and the store/saved-model warnings use the snap's `IDIOT_DATA_HOME`; run `openspec validate inspect-assertion`
