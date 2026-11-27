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

# Save store ID $2 as store $1: a valid, unused name, or die. Used by
# store add and store import, so both follow the same rules.
store_save() {
    validate_store_name "${1}"
    mkdir -p "${IDIOT_STORES_DIR}"
    require_no_store "${1}"
    printf '%s' "${2}" >"${IDIOT_STORES_DIR}/${1}"
}

# Print a store name made from display name $1: lowercased, each run of
# characters other than a-z and 0-9 replaced by '-', no '-' at either end.
# Empty when nothing is left (the caller falls back).
store_default_name() {
    printf '%s' "${1}" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9][^a-z0-9]*/-/g; s/^-//; s/-$//'
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

# Print store ID $1 for messages: the saved store's name where one matches,
# else the ID; "the global store" when $1 is empty.
store_label() {
    if [ -z "${1}" ]; then
        printf 'the global store'
    elif _sl="$(store_name_for_id "${1}")"; then
        printf "store '%s%s%s'" "${BOLD}" "${_sl}" "${RESET}"
    else
        printf 'store %s' "${1}"
    fi
}

# Queue the export that activates saved store $1; die if it doesn't exist.
emit_store_use() {
    _esu="$(store_id "${1}")" || exit 1
    emit_export UBUNTU_STORE_ID "${_esu}"
    emit_export IDIOT_ACTIVE_STORE "${1}"
    unset _esu
}

# Queue the unset that returns to the global store.
emit_store_unset() {
    emit_unset UBUNTU_STORE_ID
    emit_unset IDIOT_ACTIVE_STORE
}
