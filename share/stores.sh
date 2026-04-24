# shellcheck shell=sh
# share/stores.sh - store configuration helpers
# Source this file; do not execute directly.

IDIOT_STORES_DIR="${IDIOT_STORES_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/stores}"

# Die unless a store named $1 exists.
require_store() {
    [ -f "${IDIOT_STORES_DIR}/${1}" ] || die "store '${1}' not found"
}

# Die if a store named $1 already exists.
require_no_store() {
    [ -f "${IDIOT_STORES_DIR}/${1}" ] && die "store '${1}' already exists — use 'idiot store remove ${1}' first"
}
