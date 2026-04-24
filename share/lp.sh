# shellcheck shell=sh
# share/lp.sh - Launchpad user helpers
# Source this file; do not execute directly.

IDIOT_LP_USER_FILE="${IDIOT_LP_USER_FILE:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/lp-user}"

# Print the active LP username (from env or saved file), or empty if not set.
lp_user_value() {
    if [ -n "${IDIOT_LP_USER:-}" ]; then
        printf '%s' "${IDIOT_LP_USER}"
    elif [ -f "${IDIOT_LP_USER_FILE}" ]; then
        cat "${IDIOT_LP_USER_FILE}"
    fi
}

# Print the active LP username; die if not set.
require_lp_user() {
    _rlu="$(lp_user_value)"
    [ -n "${_rlu}" ] || die "no Launchpad username set — run: eval \"\$(idiot lp set <username>)\""
    printf '%s' "${_rlu}"
}
