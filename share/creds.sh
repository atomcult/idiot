# shellcheck shell=sh
# share/creds.sh - credential storage helpers
# Source this file; do not execute directly.

IDIOT_AUTH_DIR="${IDIOT_AUTH_DIR:-${IDIOT_DATA_HOME}/auth}"

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

# Queue the exports that activate credential $1; die if it doesn't exist.
emit_cred_login() {
    require_cred "${1}"
    _ecl="$(cat "${IDIOT_AUTH_DIR}/${1}")"
    emit_export UBUNTU_STORE_AUTH "${_ecl}"
    emit_export SNAPCRAFT_STORE_CREDENTIALS "${_ecl}"
    emit_export IDIOT_ACTIVE_CRED "${1}"
    unset _ecl
}

# Queue the unsets that deactivate the active credential.
emit_cred_logout() {
    emit_unset SNAPCRAFT_STORE_CREDENTIALS
    emit_unset UBUNTU_STORE_AUTH
    emit_unset IDIOT_ACTIVE_CRED
}

# Print the path to the whoami cache file for credential $1.
whoami_cache_path() {
    printf '%s/whoami/%s' "${IDIOT_CACHE_DIR}" "${1}"
}

# Print the path of the idiot-store helper: the installed one (make install,
# or the snap, both at $IDIOT_ROOT/libexec), else a development build in
# this checkout, else one on PATH. Dies when there's none. Use as
# `h="$(store_helper)" || exit 1`.
store_helper() {
    for _sh in "${IDIOT_ROOT}/libexec/idiot-store" \
        "${IDIOT_ROOT}/tools/idiot-store/target/release/idiot-store"; do
        if [ -x "${_sh}" ]; then
            printf '%s' "${_sh}"
            return 0
        fi
    done
    command -v idiot-store 2>/dev/null ||
        die "idiot-store not found — build it with: make install"
}

# Store permissions idiot knows: name, then what it lets a credential do.
# The first seven (package_*) are what `snapcraft export-login` grants by
# default, and what `idiot auth add` grants unless told otherwise.
STORE_PERMISSIONS='package_access	Access packages
package_manage	Manage collaborators
package_metrics	View package metrics
package_push	Push packages
package_register	Register snap names
package_release	Publish releases
package_update	Update package metadata
store_admin	Administer store
account_admin	Administer account
openid	OpenID authentication
edit_account	Edit account
create_store	Create stores'

# shellcheck disable=SC2034 # used by cmd/auth/add
DEFAULT_PERMISSIONS='package_access,package_manage,package_metrics,package_push,package_register,package_release,package_update'

# Print what permission $1 lets a credential do (the name itself if unknown).
map_permission() {
    _mp="$(printf '%s\n' "${STORE_PERMISSIONS}" | awk -F '\t' -v p="${1}" '$1 == p { print $2 }')"
    printf '%s' "${_mp:-${1}}"
}

# Succeed if $1 is a permission idiot knows.
is_store_permission() {
    printf '%s\n' "${STORE_PERMISSIONS}" | cut -f1 | grep -qx -- "${1}"
}
