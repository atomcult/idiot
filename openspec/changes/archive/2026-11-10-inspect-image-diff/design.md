# Design

## Context

`cmd/inspect/image` (~330 lines) does all its work through globals
(`image`, `tmpdir`, `seed_offset`, `assertion`, `model`, `first_boot`,
`preseeded`) and helpers that read them: `partition_sectors` (runs
`sfdisk --json` and picks one partition; called once for ubuntu-seed and
again for ubuntu-data), `find_seed_offset`, `read_model_assertion` (mtools:
first `systems/<label>/model`), `inspect_ubuntu_data` (LUKS check, modeenv
and `state.json` via debugfs), `collect_seeded_snaps` (mtools listing of
`snaps/` → `tmpdir/seeded`) and `parse_assertion_snaps` (awk over the
model's `snaps:` list → `tmpdir/channels`, `tmpdir/absent_optional`).
`first_boot` already carries color escapes, so it can't be compared or
printed anywhere but stderr. Nothing extracts whole partitions or images:
each image costs one sfdisk run per partition lookup, a few mtools reads
and two debugfs reads. See proposal.md for why a comparison is wanted, and
the inspect spec delta for what it prints.

## Goals / Non-Goals

**Goals:**
- One extraction path, used by `inspect image` and `inspect diff`, with no
  globals and no colors in what it records.
- Each image read once: one `sfdisk --json` per image, each file in it
  read once, nothing extracted that the command won't print.
- `inspect image`'s output byte-for-byte unchanged.

**Non-Goals:**
- Comparing snapd's changes, task logs, or anything inside the seeded
  snaps (their snap.yaml, files). `inspect snap` on each is the tool.
- Comparing two model assertion files without images, or images that
  aren't Ubuntu Core (no ubuntu-seed).
- Decrypting ubuntu-data.
- Verifying the model's signature (the `inspect-assertion` change).

## Decisions

### A separate `idiot inspect diff`, not `inspect image --diff`
- `inspect image` takes exactly one image after its options, and its spec
  says an argument after the image is a usage error; `--diff <a> <b>`
  would mean a flag taking two values in an options-before-operand
  parser, and `--extract-state` would have no meaning with it.
- The two commands write to different channels: `inspect image` reports
  on stderr (specified), the comparison goes to stdout. One leaf command
  per behavior keeps each help and spec simple and follows the
  one-file-per-leaf layout.
- The name leaves room for comparing other things later (two model
  assertions) without a new verb; for now it accepts only images.
Alternative considered: `inspect image --diff`. Rejected for the above.

### `share/image.sh`: extraction into a work directory
New module (4-space indent, `_im_`-style local prefixes like the other
modules), with `image_` functions that take the image and a work
directory and record what they find as files in it, so two images can be
loaded side by side (`${tmp}/a`, `${tmp}/b`):

| Function | Writes in `<dir>` |
|---|---|
| `image_load <img> <dir>` | everything below except the state section; dies (naming the image) on no ubuntu-seed |
| `image_partitions` | `ptable.json` (the one sfdisk run), `partitions.tsv`: name, start, size, type GUID per partition (`#<n>` for an unnamed one), and `offset_<name>` lookups |
| `image_read_model` | `label`, `model` (the assertion), `model-headers.tsv` (header → value, including derived `gadget`/`kernel`), `model-snaps.tsv` (name, type, default channel, presence) |
| `image_read_seed` | `seeded` (name revision, as today), `absent_optional` |
| `image_read_data` | `first-boot` (`complete`, `not-complete[:<mode>]`, `encrypted`), `preseeded`, `state.json` when readable |

`partition_sectors` becomes a lookup in `ptable.json`, so the ubuntu-data
lookup no longer reruns sfdisk. `image_read_data` keeps today's single
modeenv read and single `state.json` dump: `inspect image` needs
`state.json` for "Pre-seeded" anyway, so the diff loads it unconditionally
too and `--state` only controls whether it is compared. Raw tokens are
stored; the printers color them (`first-boot` → `complete` green, etc.),
which is what keeps `inspect image`'s output identical.

`parse_assertion_snaps`' awk grows `type:` (to derive gadget and kernel)
and keeps its behavior for `channels`/optional. A pre-UC20 model has no
`snaps:` list but `gadget:`/`kernel:` headers: both feed
`model-headers.tsv`'s `gadget`/`kernel`.

The printers (`print_seeded_snaps`, `print_installed_snaps`,
`print_connections`, `print_changes`) stay in `cmd/inspect/image`; they
read `<dir>/…` instead of `tmpdir/…`.

### Comparison: normalised TSV, one awk differ
Each section is normalised to `item<TAB>field<TAB>value` lines:
`model` (from `model-headers.tsv`, the compared headers only), `system`
(`system-label`, `preseeded`, `first-boot`), `seed` (`revision`,
`channel`), `partition` (`start`, `size`, `type`), and with `--state`,
`installed` (`revision`, `channel`) and `connection` (`connected` = `yes`)
from `state.json` via a short python3 snippet (the same keys
`print_installed_snaps`/`print_connections` read). One private
`_diff_section <section> <a.tsv> <b.tsv>` awk function loads both sides
and prints `section item field a b` for every field whose values differ,
an absent item giving empty values. That stream *is* `--tsv` output; the
human printer reads it and groups by section and item, choosing `-`, `+`
or `~` from which side is empty. Values never contain tabs or newlines
(headers are single-line; the compared multi-line `snaps` header is
represented through the seed/model-snaps rows instead).

Alternative considered: one python3 script comparing everything. Rejected:
the sh/awk extraction already exists and python3 is used here only for
JSON, as elsewhere in the project.

### stdout colors: `stdout_colors` in `share/common.sh`
`common.sh`'s color variables follow stderr (`-t 2` or `FORCE_COLOR`).
`stdout_colors` sets `OUT_BOLD`, `OUT_DIM`, `OUT_RED`, `OUT_GREEN`,
`OUT_YELLOW`, `OUT_CYAN` and `OUT_RESET` from `[ -t 1 ]` and `NO_COLOR`,
and `--tsv` skips calling it. `FORCE_COLOR` deliberately does not apply:
it exists to carry stderr's terminal-ness across the snap handoff, and a
handed-off command's stdout is the user's real stdout, which `-t 1` sees
directly. The `inspect-assertion` change uses the same helper.

### Human format
```
--- a.img
+++ b.img
Seed snaps:
  ~ pc-kernel  revision 1400 → 1500
  + htop       revision 4521, channel latest/stable
Partitions:
  ~ ubuntu-data  size 1048576 → 2097152
```
Only sections with a difference are printed; "images don't differ" goes
to stderr when there is none.

### Exit status 0 whether or not images differ
Errors exit 1 through `die`; diff(1)'s "1 means different" would collide
with that. A script can test `[ -z "$(idiot inspect diff --tsv a b)" ]`.

### Host command
`# idiot: host`: it reads arbitrary user files and runs sfdisk, debugfs,
mtools and python3 (host or `core` component), like `inspect image`.

## Risks / Trade-offs

- [Refactor changes `inspect image` output] → Before/after captures on
  the fresh and first-booted Core 24 images, compared with `cmp`.
- [Revision and channel of a snap that's in one image only show as two
  rows] → Intentional for `--tsv` (one row per field); the human form
  shows it on one line.
- [Partition matching by name misses a renamed partition] → It shows as
  one removed and one added, which is what happened.
- [A model with no recovery system or no assertion] → `model-headers.tsv`
  is empty; every header the other image has shows as `+`/`-`.
- [The two images are the same file] → Read twice; correct, just slower.

## Assumptions

- Exit 0 on differences (not diff(1)'s 1) is acceptable.
- The diff compares only the first recovery system, as `inspect image`
  shows only the first.
- Partition `start` is worth comparing (a moved partition matters for
  gadget changes) even though it's noisy when sizes change.
- `inspect-assertion` may land first; whichever lands second reuses
  `stdout_colors` rather than adding it again.
