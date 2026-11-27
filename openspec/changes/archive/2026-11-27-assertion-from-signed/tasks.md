# Tasks

## 1. idiot-helper: drafts from signed assertions

- [x] 1.1 In `tools/idiot-helper/src/draft.rs`, make `parse_draft` accept signed input through `assertion::decode_stream`:
  - pick the one model, system-user or validation-set in the stream, and fail listing the types seen when there are none or several;
  - convert `V` to JSON values in header order, dropping `sign-key-sha3-384` and `body-length`;
  - fail on a non-empty body;
  - emit the `o` record (`signed` or `draft`, type, primary keys, revision) after `t`, which `draft write` ignores.

  Verify with unit tests:
  - Canonical's signed `ubuntu-core-24-amd64.model` (testdata) gives the same object as its JSON draft, minus `sign-key-sha3-384`;
  - a `--chain`-style stream (account, account-key, model) picks the model;
  - account-only and two-model streams fail naming their types;
  - `read | write` of signed input gives a draft that `draft check` passes;
  - the old `signed_and_foreign_input` test is replaced (foreign types are still refused).
- [x] 1.2 Run `cargo test`, `make check`, and grep the change for socket or HTTP use (none). Commit: `feat(idiot-helper): read signed assertions as drafts`.

## 2. sh: edit from a signed file, stdin or the store

- [x] 2.1 In `cmd/assertion/edit`:
  - accept `-`, and two positionals `model|validation-set <key>`, which fetch through `"${IDIOT_ROOT}/cmd/store/assertion" --raw` (passing `--cred`) into `${work}/source`; a non-zero exit stops before any prompt, with its message;
  - read the `o` record, and say on stderr where the input came from;
  - for signed sources without `-o`, write with `as_write` to the default name, computed as `new` does from the edited values, and never overwrite;
  - update help and usage.

  Verify from the source tree, with `--no-edit` and no terminal:
  - the spec's "Signed input" and "Draft beside it already exists" scenarios, using a signed model built from testdata;
  - "A chain on stdin" with `idiot store assertion --chain --raw model canonical/ubuntu-core-24-amd64` (anonymous);
  - `edit model canonical/nope` fails with the not-found message and writes nothing;
  - `edit kiosk.json` on a draft still writes back.
- [x] 2.2 Update `README.md` ("Writing assertions": editing a published model) and `skills/using-idiot/references/commands.md`. Run `make check`. Commit: `feat(assertion): edit signed assertions and fetch them from the store`.

## 3. sh: templates for new assertions

- [x] 3.1 In `cmd/assertion/new` and `share/assertion.sh`:
  - add `--from <file|->` and `--from-store <key>` (refused together, and `--from-store` refused for system-user);
  - load the template's records, refuse a different type naming both, and drop the identity headers listed in design.md decision 7 before `--set`/`--add`/`--snap` and the prompts.

  Verify from the source tree, no terminal:
  - "A model from Canonical's" (anonymous `--from-store canonical/ubuntu-core-24-amd64`, with a fake credential's account as in the assertion-writer bare-host check, or `--set brand-id=… --set authority-id=…` when no credential);
  - "Template of another type";
  - "Identity still required without a terminal";
  - a system-user template from a signed one keeps `models` and `ssh-keys` but drops the username, email and password.
- [x] 3.2 Update `new`'s help, `README.md` and the skill reference. Run `make check`. Commit: `feat(assertion): start a new assertion from another`.

## 4. Verification

- [x] 4.1 Extend `tests/bare-host.d/assertion-writer`:
  - offline: a signed-looking model built in the script (headers, blank line, a dummy signature) edited with `--no-edit -o -`, a stream with no writable assertion, `new --from` of another type, and the no-overwrite case;
  - when the store is reachable: `edit model canonical/ubuntu-core-24-amd64 --no-edit -o -` gives a draft that `idiot assertion check` passes, apart from any errors the original already has.
- [x] 4.2 Run `openspec validate assertion-from-signed --strict`.
- [x] 4.3 Run `snapcraft clean idiot`, `snapcraft pack` (into the tree; the snap and components are gitignored), and `tests/bare-host --image local:8257342f834e <snap> <comps>` on `local:`. All checks pass.
- [ ] 4.4 With the user, on a terminal under the snap: `idiot assertion edit model <their-brand>/<their-model>` offers the next revision and writes `<model>.json`; `idiot assertion sign` on it gives a model that `idiot inspect assertion` verifies.

## Archive note (2026-11-27)

Archived at the user's request with 4.4 open: editing the user's own
published model on a terminal under the snap and signing it with their key.
Bare-host on local: passed 354 of 354, including the new checks.
