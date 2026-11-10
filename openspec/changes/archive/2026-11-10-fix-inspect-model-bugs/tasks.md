# Tasks

## 1. Model repository names

- [x] 1.1 `share/models.sh`: `validate_model_repo_name` (letters, digits, `-`, `_`), used by `cmd/model/add` instead of its own check and by `remove` and `update` before anything is read or deleted; verify with scratch data dirs that `remove ../victim` and `update ../auth/fake` fail with the invalid-name error, the victim file survives and the fake marker is never printed, and add/remove/update of valid names still work

## 2. inspect image --extract-state

- [x] 2.1 `cmd/inspect/image`: the value after `--extract-state` only when another argument follows, else `./state.json`; `--extract-state=` fails before inspecting; with no readable state, fail after the summary; verify each spec scenario against the fresh and first-booted Core 24 images

## 3. Finish

- [x] 3.1 Run `make check`, re-run the inspect and model baselines' scenarios, and commit (`fix: model repo names and inspect --extract-state`)
