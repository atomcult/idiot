# shellcheck shell=sh
# share/creds.sh - credential storage helpers
# Source this file; do not execute directly.

IDIOT_AUTH_DIR="${IDIOT_AUTH_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/auth}"

# Die unless $1 is a safe credential name: letters, digits, '.', '-', '_',
# not starting with '.' (so no '/', '..' or hidden files).
validate_cred_name() {
    case "${1}" in
    '' | .* | *[!A-Za-z0-9._-]*)
        die "invalid credential name '${1}': use only letters, digits, '.', '-' and '_', not starting with '.'"
        ;;
    esac
}

# Create IDIOT_AUTH_DIR (or tighten an existing one) so only its owner can
# enter it, and make files created from here on private.
ensure_auth_dir() {
    umask 077
    mkdir -p "${IDIOT_AUTH_DIR}"
    chmod 700 "${IDIOT_AUTH_DIR}"
}

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
