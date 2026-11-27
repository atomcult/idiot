# Tasks

## 1. Offline check

- [x] 1.1 In `tools/idiot-helper/src/draft.rs` `check_model`, add the rules from design.md:
  - `distribution`: required and of os-release form in a classic model with `snaps`, an error in any other model;
  - a classic model with `snaps` lists a kernel only with a gadget;
  - an old-style classic model (no `snaps`) has no `kernel` or `base`.

  Add `model_rules` cases for each scenario in "Checking an assertion": classic without a distribution, distribution in Ubuntu Core, classic kernel without a gadget, classic without kernel or gadget passing, and old-style classic with a kernel. Run `cargo test`.
- [x] 1.2 In `check_validation_set`, check the components as design.md says (presence values, the string shortcut, the revision rules), naming `snaps[<snap>].components.<name>`. Add `validation_set_rules` cases. Run `cargo test`.
- [x] 1.3 Verify: the five drafts from the investigation give the results snapd gives (four errors, one pass) with `idiot assertion check`. Run `make check`. Commit: `feat(idiot-helper): check classic models and validation-set components as snapd does`.

## 2. idiot-store info

- [ ] 2.1 Add `confinement` to `INFO_FIELDS`, and have `info_line` print the channel's confinement as an eighth column. Update the `info_lines` test, and check that `store channels`, `store channels --tsv` and `store browse --tsv` print what they did. Run `make check`. Commit: `feat(idiot-store): confinement in info lines`.

## 3. Writer

- [ ] 3.1 In `cmd/assertion/new`, add `--classic` (as `--set classic=true`) to the options and help.
- [ ] 3.2 In `share/assertion.sh`:
  - `as_model_headers` settles `classic` (given, the draft's, asked on a terminal, else Ubuntu Core) and, for classic, asks for `distribution` (default `ubuntu`), refusing a value that isn't an os-release ID;
  - `as_essentials` and `as_model_snaps` treat a classic model's gadget and kernel as optional, as design.md says;
  - `as_model_snap` reads the confinement column, marking `classic: true` in a classic model and warning in an Ubuntu Core one.
- [ ] 3.3 Verify from the source tree, without a terminal, against the global store (anonymous):
  - the spec's "Classic without a terminal" draft passes `idiot assertion check`;
  - `--classic --snap pc=24/stable --snap pc-kernel=24/stable` lists both;
  - `--classic --snap pc-kernel=24/stable` (no gadget) fails `check`;
  - `--set distribution=Ubuntu` is refused;
  - a plain `new model` is unchanged;
  - `edit` of a classic draft keeps `classic` and `distribution`;
  - a classic-confined snap is marked in a classic model and warned about in an Ubuntu Core one.
- [ ] 3.4 Update README ("Writing assertions": a classic model) and `skills/using-idiot/references/commands.md`. Run `make check`. Commit: `feat(assertion): write classic models`.

## 4. Verification

- [ ] 4.1 Extend `tests/bare-host.d/assertion-writer` with offline check cases (classic without a distribution, distribution in Ubuntu Core, classic kernel without a gadget, a component revision rule) and, when the store is reachable, `new model --classic --no-edit -o -` passing `check`. Run `make check`. Commit: `test(bare-host): classic models and validation-set components`.
- [ ] 4.2 Run `openspec validate classic-models --strict`.
- [ ] 4.3 Run `snapcraft pack` and `tests/bare-host --image local:8257342f834e <snap> <comps>` on `local:`. All checks pass. This needs a shell where snaps can run.
