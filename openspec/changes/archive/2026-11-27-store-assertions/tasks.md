# Tasks

## 1. Response shapes

- [x] 1.1 Record public, unauthenticated responses as fixtures under `tools/idiot-store/src/testdata/assertions/`: `account/canonical`, both Canonical account-keys (`root`, `store`), hello-world's `snap-declaration` and r29 `snap-revision`, `model/16/canonical/ubuntu-core-24-amd64`, and the JSON error bodies for `not-found`, `invalid-request` (key shape) and an unknown type. Confirm the `/v2/snaps/refresh` `download` action for a revision outside the channel map (hello-world, anonymously, for amd64 and for another arch) and record its answer. Verify: the fixtures hold no `Macaroon`, no `@` and no account ID other than Canonical's.

## 2. idiot-store: `assertion`

- [x] 2.1 The assertion types table from snapd's `asserts.go` (primary keys, optional trailing key, sequence-forming) and header-word parsing into a request path: `series` default, `validation-set` `?sequence=latest`, hex `snap-sha3-384` to base64url, per-type `max-format`, unknown type and missing/unknown header errors (exit 1, naming the key shape). Verify with unit tests that each fixture's assertion maps to the path it was fetched from, and on each error.
- [x] 2.2 The fetch: `GET /v2/assertions/...` with `Accept: application/x.ubuntu.assertion`, `Snap-Device-Store` from `--store`, the credential from `--file`/`UBUNTU_STORE_AUTH` or none, through the existing send-and-refresh path; `not-found` and hidden lookups exit 4, other store errors 3. The `snap-name=` lookups through `/v2/snaps/info` (snap-id; a revision's `download.sha3-384`), falling back to the refresh `download` action as confirmed in 1.1. Verify with unit tests on the fixtures (404 body → exit 4, 400 → 3 with the store's message, channel map → hash for r29).
- [x] 2.3 `--chain`: a reader for top-level headers and a `snap-developer`'s `developer-id`s, the prerequisites in design.md, depth-first with dedup, stopping at Canonical's account and root key; output only once all are fetched, bodies separated by one blank line. Verify with a unit test whose fake fetcher serves the 1.1 fixtures: hello-world r29 gives store key, declaration, revision in that order and nothing else; a cycle or repeat is fetched once.
- [x] 2.4 Document the subcommand and exit 4 in `main.rs`'s header and help. Run `cargo test` and `make check`. Commit: `feat(idiot-store): fetch assertions and their prerequisites`.

## 3. sh: `idiot store assertion`

- [x] 3.1 `cmd/store/assertion` (`# idiot: host`): the shortcuts mapped to header words, the generic `header=value` form, `--cred` via `query_cred`, `--store`/`--arch` passed on, `store` with no ID and no active store failing offline; default output through `cmd/inspect/assertion` (with `--no-verify` passed through and a `--chain` hint when a key is unverified), `--raw`, `-o <file>` written atomically with the count on stderr, `--raw` with `-o` refused; exit statuses mapped to the messages in design.md. Verify from the source tree: `idiot store assertion account canonical` (summary, `verified`), `--raw account-key <store key> | head -1`, `--chain -o /tmp/x.assert snap-revision hello-world 29` then `idiot inspect assertion /tmp/x.assert` (three assertions, all `verified` offline), `account no-such-account` (not-found message, exit 1, existing `-o` file unchanged), `model canonical` (key-shape error, nothing contacted).
- [x] 3.2 `cmd/store/help`, the command's `--help`, README (Stores section: fetching assertions, side-loading with `--chain` and `snap ack`), `skills/using-idiot/references/commands.md`. Extend `tests/bare-host.d/store`: usage, unknown type, missing key, `--raw` with `-o`, `--cred nope`, `store` with no store active (all offline); and, when the container reaches api.snapcraft.io, an anonymous `--chain --raw snap-revision hello-world 29` that `idiot inspect assertion -` reports `verified` (skipped with a note otherwise). Run `make check`. Commit: `feat(store): fetch assertions from the store`.

## 4. Verification

- [x] 4.1 Run `openspec validate store-assertions --strict`.
- [ ] 4.2 Pack the snap (`snapcraft clean idiot`, pack into a gitignored directory inside the tree) and run `tests/bare-host` on `local:`; all checks pass.
- [ ] 4.3 With the user, under the snap: `idiot store assertion --chain snap-revision ce-dummy <rev>` with `ce-team-test` active (a brand-store snap, with their credential), `store` for `ce-team-test`, and a brand model by `model <brand>/<model>`; the summaries verify, `--chain --raw … | sudo snap ack /dev/stdin` is accepted on a scratch machine or VM, and nothing printed contains a credential.

Archived 2026-11-27 with 4.2 and 4.3 open. Done of 4.3, from the source
tree with the user's credential and `ce-team-test` active: the `store`
assertion and its key (verified); `--chain snap-revision ce-dummy 2`
(store key, publisher account, declaration, revision, all verified),
which found and fixed a doubled Snap-Device-Store header; the same
revision fetched anonymously too (brand-store snap assertions need no
credential). Not done: a brand model by `model <brand>/<model>`, and
`snap ack` of a `--chain --raw` stream on a scratch machine. 4.2: the
first bare-host run passed 231 of 232 (a 503 from the network
component's HTTPS check); the rerun with the fix was still running.
