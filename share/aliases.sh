#!/usr/bin/env sh
# share/aliases.sh - old command names and the commands they now are
#
# Renamed commands keep working under their old names: the dispatcher
# rewrites the words before dispatch (so counting, help and the snap's
# host handoff all see the new command) and says what the new name is.
# One rename per line, "old words|new words"; a two-word match wins over
# a one-word one. Removing an alias is deleting its line.

# shellcheck disable=SC2034 # read by alias_resolve
IDIOT_ALIASES=''

# Print "<n>|<new words>" when the first one or two words ($1, $2) are an
# old name, n being how many of them it is; fail otherwise.
alias_resolve() {
    _ar_two="${1:-} ${2:-}"
    _ar_one="${1:-}"
    _ar_hit=''
    while IFS='|' read -r _ar_old _ar_new; do
        [ -n "${_ar_old}" ] || continue
        if [ "${_ar_old}" = "${_ar_two}" ]; then
            _ar_hit="2|${_ar_new}"
            break
        fi
        [ "${_ar_old}" != "${_ar_one}" ] || [ -n "${_ar_hit}" ] ||
            _ar_hit="1|${_ar_new}"
    done <<EOF
${IDIOT_ALIASES}
EOF
    [ -n "${_ar_hit}" ] || return 1
    printf '%s\n' "${_ar_hit}"
}

# Say that `idiot <old>` is now `idiot <new>` (stderr, one line).
alias_note() {
    say "${CYAN}note:${RESET} 'idiot ${1}' is now 'idiot ${2}'"
}
