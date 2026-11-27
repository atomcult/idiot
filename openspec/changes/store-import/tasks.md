# Tasks

## 1. Shared helpers

- [ ] 1.1 Add `store_default_name` and `store_save` to `share/stores.sh`, and switch `cmd/store/add` to `store_save` (its messages and failures unchanged). Verify in a scratch data directory: `store add` still adds, refuses `../x` and duplicates as the spec's scenarios say; `store_default_name` turns `Example Team Store` into `example-team-store`, `  Fancy__Store!! ` into `fancy-store`, and `!!!` into an empty string (the caller's fallback).

## 2. store import

- [ ] 2.1 `cmd/store/import` (host): `--all`, `--cred`, the terminal check before any network call, skipping the global store and saved IDs, the fzf multi-select picker, name prompts on `/dev/tty` with defaults and re-asking, collision suffixes, per-store and summary messages on stderr. Add it to `cmd/store/help`. Verify from the source tree: `--all` against the user's account (with their go-ahead) saves the unsaved stores and skips `ce-team-test`; a re-run saves nothing; with a fake `account` response (a stub `idiot-store` on PATH in a copied tree), the collision and empty-name cases give the spec's names; without a terminal and without `--all` it fails before calling the helper.
- [ ] 2.2 Extend `tests/bare-host.d/store`: usage, the non-terminal failure, and no credential. Update the README (Checking a store) and the using-idiot skill reference. Run `make check`. Commit: `feat(store): import the account's stores`.

## 3. Verification

- [ ] 3.1 Run `openspec validate store-import --strict`.
- [ ] 3.2 Pack the snap (`snapcraft clean idiot`, pack into a gitignored directory inside the tree) and run `tests/bare-host` on `local:`; all checks pass.
- [ ] 3.3 With the user, on a terminal: `idiot store import` shows only unsaved stores, the name prompt accepts the default and an edited name, and `idiot store roles` afterwards shows the imported stores by their saved names.
