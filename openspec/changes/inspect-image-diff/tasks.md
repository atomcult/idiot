# Tasks

## 1. Extraction module (`refactor(inspect): move image extraction to share/image.sh`)

- [ ] 1.1 Capture today's `idiot inspect image` stderr (with `FORCE_COLOR=1` and with `NO_COLOR=1`) for a fresh and a first-booted Core 24 image, plus `--extract-state` output, into the scratch directory as the baseline
- [ ] 1.2 `share/image.sh`: `image_load` and its parts (`image_partitions` with one `sfdisk --json` per image and `partitions.tsv`, `image_read_model` with `model-headers.tsv` and `model-snaps.tsv` including `type` and pre-UC20 `gadget`/`kernel` headers, `image_read_seed`, `image_read_data` storing uncolored `first-boot`/`preseeded` tokens); the no-ubuntu-seed error names the image; verify with a shell that sources the module that a fresh image's work directory holds the expected files and that sfdisk runs once (`strace -f -e execve` or a PATH shim counting calls)
- [ ] 1.3 `cmd/inspect/image`: use `image_load` into a `mktemp -d` directory, printers read it and color the tokens; verify every captured baseline matches byte for byte (`cmp`) and the existing inspect spec scenarios (not a Core image, encrypted data, `--extract-state` cases) still hold
- [ ] 1.4 Run `make check` and commit

## 2. stdout colors (`feat(common): add stdout_colors`)

- [ ] 2.1 `share/common.sh`: `stdout_colors` setting `OUT_*` from `[ -t 1 ]` and `NO_COLOR` only (not `FORCE_COLOR`), documented in the CLAUDE.md helper list; skip this group if `inspect-assertion` already added it; verify `sh -c '. share/common.sh; stdout_colors; printf %s "$OUT_RED"' | od -c` is empty and non-empty when run on a terminal
- [ ] 2.2 Run `make check` and commit

## 3. `idiot inspect diff` (`feat(inspect): compare two Core images`)

- [ ] 3.1 `cmd/inspect/diff` (`# idiot: host`): options `--state`, `--tsv`, `-h`; exactly two images, else the usage error; both loaded with `image_load`; section normalisers (python3 for `installed`/`connection` from `state.json`) and the `_diff_section` awk differ producing the TSV stream; a missing-state message on stderr per image under `--state`; verify against the spec scenarios: identical image (copy of itself) → empty stdout and exit 0, `--tsv` row shape, one image → usage error, README.md as the second image → error naming it
- [ ] 3.2 Human printer: `---`/`+++` lines, section headings, `-`/`+`/`~` lines with `OUT_*` colors from `stdout_colors`; verify `| cat` output has no escape bytes (`grep -c "$(printf '\033')"` is 0) and a terminal run is colored
- [ ] 3.3 Real comparisons: build or download two Core 24 images that differ (e.g. `ubuntu-core-24-amd64` and its `-dangerous` model, or the same model with `--snap` for a newer pc-kernel via `ubuntu-image`), and a fresh vs first-booted image with `--state`; verify the Different models, New kernel revision, State compared and State unreadable scenarios
- [ ] 3.4 `cmd/inspect/help` lists `diff`; README's inspect section mentions it if it lists subcommands; verify `idiot inspect` and `idiot inspect diff --help` show it
- [ ] 3.5 Under the snap (locally built, `idiot+core` installed, host without e2fsprogs/mtools in a bare LXD container or with them hidden from PATH): `idiot inspect diff --tsv a.img b.img | cat` works through the host handoff and stdout stays clean
- [ ] 3.6 Run `make check` and `openspec validate inspect-image-diff`, and commit
