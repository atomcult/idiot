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

`releases-ce-dummy.json`, `channel-map-ce-dummy.json` and
`releases-404.json` were recorded live on 2026-11-27 with the user's
go-ahead; the publisher's account ID, username and display name and the
snap ID were replaced with made-up values.

`info-hello-world.json` is the public channel map of `hello-world`, as
the store served it anonymously on 2026-11-27.

`info-pc-kernel.json` is a real, anonymous response for the public
`pc-kernel` snap (`/v2/snaps/info/pc-kernel?fields=download,revision,version,created-at,architectures,type`,
recorded 2026-11-27), trimmed to its `24` track. It holds no account data.
