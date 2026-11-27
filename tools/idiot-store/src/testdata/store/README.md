# Store response fixtures

Synthetic responses with the shapes the Snap Store returned on 2026-11-26
(`account.json`, `info.json`: recorded live, then rewritten with made-up
stores, snaps, accounts, IDs and URLs) or documents (`store-snaps.json`,
`store-snaps-403.json`: from the dashboard's v2 stores reference). No
field holds real account data.

`info-pc-kernel.json` is a real, anonymous response for the public
`pc-kernel` snap (`/v2/snaps/info/pc-kernel?fields=download,revision,version,created-at,architectures,type`,
recorded 2026-11-27), trimmed to its `24` track. It holds no account data.
