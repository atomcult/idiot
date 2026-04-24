# shellcheck shell=sh
# share/creds.sh - credential storage helpers
# Source this file; do not execute directly.

IDIOT_AUTH_DIR="${IDIOT_AUTH_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/auth}"

# Die unless a credential named $1 exists.
require_cred() {
    [ -f "${IDIOT_AUTH_DIR}/${1}" ] || die "credentials '${1}' not found"
}

# Die if a credential named $1 already exists.
require_no_cred() {
    [ -f "${IDIOT_AUTH_DIR}/${1}" ] && die "credentials '${1}' already exist — remove it first"
}

# Print the path to the whoami cache file for credential $1.
whoami_cache_path() {
    printf '%s/whoami/%s' "${IDIOT_CACHE_DIR}" "${1}"
}
