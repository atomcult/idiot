# Tasks

## 1. idiot-store

- [ ] 1.1 `query::store_members` and `idiot-store store-members --store <id>` (`member` lines, `refused permission <p>` / `refused not-admin`), with a synthetic fixture shaped like the documented response (made-up names and addresses only). Verify with unit tests: members parsed and sorted, the account ID never printed, both refusals. Run `cargo test` and `make check`. Commit: `feat(idiot-store): list a store's members`.

## 2. store members

- [ ] 2.1 Move the admin pre-check from `cmd/store/snaps` into `share/stores.sh` (`store_admin_check`), with `store snaps` behaving as before. Verify: the store bare-host check and a local run of `store snaps` against the user's account (partial list and note unchanged).
- [ ] 2.2 `cmd/store/members` (host): `--role`, `--email`, `--tsv`, `--cred`, the pre-check, the global-store refusal; add it to `cmd/store/help`. Verify from the source tree with a stub idiot-store (copied tree) for the listed, filtered, email and refusal cases; emails absent without `--email`.
- [ ] 2.3 Extend `tests/bare-host.d/store` (usage, no credential, global store) and update the README and the using-idiot skill reference. Run `make check`. Commit: `feat(store): list a store's members`.

## 3. Verification

- [ ] 3.1 Run `openspec validate store-members --strict`.
- [ ] 3.2 Pack the snap (`snapcraft clean idiot`, pack into a gitignored directory inside the tree) and run `tests/bare-host` on `local:`; all checks pass.
- [ ] 3.3 With the user, once they have a `store_admin` credential: `idiot store members --cred <it>` in `ce-team-test` lists the members, confirming the response shape.
