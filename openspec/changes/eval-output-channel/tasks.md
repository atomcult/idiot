# Tasks

## 1. Eval file plumbing

- [x] 1.1 Add `emit_export` and `require_wrapper` to `share/common.sh` and make `emit_unset` append to `"${IDIOT_EVAL_FILE}"`; verify with a sh snippet that sources common.sh, sets `IDIOT_EVAL_FILE` to a temp file, and checks the file holds `export X='a b'\''c'` and `unset Y`
- [x] 1.2 Rewrite the bash/zsh and fish wrappers in `cmd/init` to create the file with `mktemp`, pass it in `IDIOT_EVAL_FILE`, source it, delete it on every path, and return the command's status; verify in bash and fish that `idiot arch set arm64` sets the variable, `idiot arch set bogus` returns 1, and no `idiot` temp files remain afterwards
- [x] 1.3 Run `make check` and commit (`feat(init)!: evaluate a private file instead of stdout`)

## 2. Env-setting commands

- [x] 2.1 Switch `auth login/logout`, `store use/unset`, `arch set/unset` and `lp set/unset` to `require_wrapper` + `emit_export`/`emit_unset`; verify each through the new wrapper in bash and fish, and verify that with `IDIOT_SHELL` set but `IDIOT_EVAL_FILE` unset `idiot store use <name>` fails with the re-run-init hint and changes nothing
- [x] 2.2 Grep `cmd/` and `share/` for remaining `printf 'export` and confirm none is left
- [x] 2.3 Run `make check` and commit (`refactor: write exports to the eval file`)

## 3. Printing commands use stdout

- [x] 3.1 `cmd/run` needs no code change (the tool already inherits stdout; only the wrapper used to capture it); verify `idiot run cat <file containing "touch /tmp/pwned">` prints the line and does not create the file, and `idiot run --shell` is interactive
- [x] 3.2 `cmd/auth/export` (no `-f`), `cmd/model/pick -o -` and `cmd/version --short` write to stdout; verify `idiot auth export <name> | wc -c` matches the credential's size and prints nothing else, and `idiot version --short > v && cat v` shows the version
- [x] 3.3 Run `make check` and commit (`fix: print data output on stdout`)

## 4. Documentation

- [ ] 4.1 Update CLAUDE.md ("How it works", output-discipline table) and STYLE.md ("Output discipline", "Eval-able output", helpers reference) to describe the eval file and `emit_export`/`require_wrapper`; update README "How it works"; verify no doc still says stdout is eval'd (`grep -rn "stdout" CLAUDE.md STYLE.md README.md`)
- [ ] 4.2 Run `make check` and commit (`docs: describe the eval file contract`)
