# shellcheck shell=sh
# share/stores.sh - store configuration helpers
# Source this file; do not execute directly.

IDIOT_STORES_DIR="${IDIOT_STORES_DIR:-${IDIOT_DATA_HOME}/stores}"

# Die unless a store named $1 exists.
require_store() {
    [ -f "${IDIOT_STORES_DIR}/${1}" ] || die "store '${1}' not found"
}

# Die if a store named $1 already exists.
require_no_store() {
    [ -f "${IDIOT_STORES_DIR}/${1}" ] && die "store '${1}' already exists — use 'idiot store remove ${1}' first"
}

# Die unless $1 is a valid store name: letters, digits, '-' and '_'.
validate_store_name() {
    case "${1}" in
    '' | *[!A-Za-z0-9_-]*)
        die "invalid store name '${1}': use only letters, digits, hyphens, underscores"
        ;;
    esac
}

# Print the ID of saved store $1; die if it doesn't exist.
# Use as `id="$(store_id "${name}")" || exit 1`.
store_id() {
    require_store "${1}"
    cat "${IDIOT_STORES_DIR}/${1}"
}

# Print the name of the saved store whose ID is $1; fail if none matches.
store_name_for_id() {
    [ -n "${1}" ] && [ -d "${IDIOT_STORES_DIR}" ] || return 1
    for _snfi in "${IDIOT_STORES_DIR}"/*; do
        [ -f "${_snfi}" ] || continue
        if [ "$(cat "${_snfi}")" = "${1}" ]; then
            printf '%s' "${_snfi##*/}"
            return 0
        fi
    done
    return 1
}

# Queue the export that activates saved store $1; die if it doesn't exist.
emit_store_use() {
    _esu="$(store_id "${1}")" || exit 1
    emit_export UBUNTU_STORE_ID "${_esu}"
    unset _esu
}

# Queue the unset that returns to the global store.
emit_store_unset() {
    emit_unset UBUNTU_STORE_ID
}
