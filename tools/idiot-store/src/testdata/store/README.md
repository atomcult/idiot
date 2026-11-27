# Store response fixtures

Synthetic responses with the shapes the Snap Store returned on 2026-11-26
(`account.json`, `info.json`: recorded live, then rewritten with made-up
stores, snaps, accounts, IDs and URLs) or documents (`store-snaps.json`,
`store-snaps-403.json`, `store-members.json`: from the dashboard's v2
stores reference; the members' names and addresses are made up). No field
holds real account data.

`channel-map.json`, `channel-map-base.json` (v2 info with `channel-maps`'
fields, shaped like the live anonymous answer for `hello` and `core22`)
and `releases.json` (the dashboard's `/api/v2/snaps/<name>/releases`, from
its v2 snaps reference and snapcraft's model; it mixes both spellings of
`created_at`, `build_url` and the hash on purpose) are synthetic too, until
real answers are recorded with the user.
