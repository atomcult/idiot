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

# Strip the base64 padding snapd can't read from saved credential $1's
# macaroons (lossless; older idiot-store wrote it), saying so when it did.
# Warns and carries on without idiot-store; never fails, never prints the
# credential (idiot-store prints only ok/repaired).
repair_cred() {
    if ! _rpc_h="$(store_helper_path)"; then
        say "${YELLOW}warning:${RESET} could not check credentials '${1}' for snapd (idiot-store not found)"
        return 0
    fi
    case "$("${_rpc_h}" repair "${IDIOT_AUTH_DIR}/${1}" 2>/dev/null)" in
    repaired) say "Repaired credentials '${BOLD}${1}${RESET}' ${DIM}(snapd couldn't use them)${RESET}" ;;
    esac
    return 0
}

# Queue the exports that activate credential $1 (repaired first); die if it
# doesn't exist.
emit_cred_login() {
    require_cred "${1}"
    repair_cred "${1}"
    _ecl="$(cat "${IDIOT_AUTH_DIR}/${1}")"
    emit_export UBUNTU_STORE_AUTH "${_ecl}"
    emit_export SNAPCRAFT_STORE_CREDENTIALS "${_ecl}"
    emit_export IDIOT_ACTIVE_CRED "${1}"
    if _ecl="$(read_expiry "${1}")"; then
        emit_export IDIOT_CRED_EXPIRES "${_ecl}"
    else
        emit_unset IDIOT_CRED_EXPIRES
    fi
    unset _ecl
}

# Queue the unsets that deactivate the active credential.
emit_cred_logout() {
    emit_unset SNAPCRAFT_STORE_CREDENTIALS
    emit_unset UBUNTU_STORE_AUTH
    emit_unset IDIOT_ACTIVE_CRED
    emit_unset IDIOT_CRED_EXPIRES
}

# The pairing record's value for a credential that belongs to every store
# (a store ID can't be *). Unpaired credentials are treated the same.
CRED_ANY_STORE='*'

# Print the path of the record of which store credential $1 belongs to.
# Hidden (credential names can't start with '.'), so `auth list` and the
# pickers skip it; it holds a store ID, never credential contents.
cred_store_path() {
    printf '%s/.store.%s' "${IDIOT_AUTH_DIR}" "${1}"
}

# Record that credential $1 belongs to store ID $2 (empty: the global store;
# CRED_ANY_STORE: any store).
record_cred_store() {
    ensure_auth_dir
    printf 'store: %s\n' "${2}" >"$(cred_store_path "${1}")"
}

# Print the store ID credential $1 belongs to (empty: global;
# CRED_ANY_STORE: any store); fail when it has no recorded store.
cred_store_id() {
    _csi="$(cred_store_path "${1}")"
    [ -f "${_csi}" ] || return 1
    sed -n 's/^store:[[:space:]]*//p' "${_csi}"
}

# Print a short name for the store credential $1 belongs to, for listings:
# the saved store's name (else its ID), "global", or "any store" (marked so,
# or never paired). Needs stores.sh.
cred_store_short() {
    _css="$(cred_store_id "${1}")" || _css="${CRED_ANY_STORE}"
    case "${_css}" in
    '') printf 'global' ;;
    "${CRED_ANY_STORE}") printf 'any store' ;;
    *) store_name_for_id "${_css}" || printf '%s' "${_css}" ;;
    esac
}

# Warn on stderr when credential $1 belongs to a store other than store ID
# $2 (empty: global), the store active once the caller's change is made.
# Silent without a credential name or a recorded store, or for one marked
# for any store; never fails, never dies, so it can't change a command's
# outcome. Callers include stores.sh.
warn_store_mismatch() {
    [ -n "${1}" ] || return 0
    _wsm_own="$(cred_store_id "${1}")" || return 0
    [ "${_wsm_own}" != "${CRED_ANY_STORE}" ] || return 0
    [ "${_wsm_own}" != "${2}" ] || return 0
    if [ -z "${_wsm_own}" ]; then
        _wsm_fix='idiot store unset'
    elif _wsm_name="$(store_name_for_id "${_wsm_own}")"; then
        _wsm_fix="idiot store use ${_wsm_name}"
    else
        _wsm_fix="idiot store add <name> ${_wsm_own}"
    fi
    say "${YELLOW}warning:${RESET} credentials '${BOLD}${1}${RESET}' belong to $(store_label "${_wsm_own}"), but $(store_label "${2}") is active"
    say "         ${DIM}switch: ${_wsm_fix} · re-pair: idiot auth bind ${1} · any store: idiot auth bind --any ${1}${RESET}"
    return 0
}

# Print the path of the record of when credential $1 expires: one line, Unix
# time. Hidden like the store record; never holds credential contents.
expiry_record_path() {
    printf '%s/.expires.%s' "${IDIOT_AUTH_DIR}" "${1}"
}

# Print the path of the stamp holding the day (UTC) credential $1's expiry
# was last warned about.
expiry_stamp_path() {
    printf '%s/expiry-warned/%s' "${IDIOT_CACHE_DIR}" "${1}"
}

# Print the earliest Unix time among idiot-store expiry's `<label> <time>`
# lines on stdin; fail when there's none.
_earliest_expiry() {
    _ee=''
    while read -r _ee_label _ee_time; do
        _ee_t="$(date -u -d "${_ee_time}" +%s 2>/dev/null)" || continue
        if [ -z "${_ee}" ] || [ "${_ee_t}" -lt "${_ee}" ]; then
            _ee="${_ee_t}"
        fi
    done
    [ -n "${_ee}" ] || return 1
    printf '%s' "${_ee}"
}

# Record when saved credential $1 expires: the earliest expiry caveat in the
# credential itself (read offline by idiot-store, from the file: never via
# argv or stdout), else fallback time $2 (anything `date -d` reads), if
# given. Fails, writing nothing, when neither gives a time. The record is
# private (ensure_auth_dir's umask).
record_expiry() {
    _rx=''
    if _rx_h="$(store_helper 2>/dev/null)"; then
        _rx="$("${_rx_h}" expiry --file "${IDIOT_AUTH_DIR}/${1}" 2>/dev/null | _earliest_expiry)" || _rx=''
    fi
    if [ -z "${_rx}" ] && [ -n "${2:-}" ]; then
        _rx="$(date -u -d "${2}" +%s 2>/dev/null)" || _rx=''
    fi
    [ -n "${_rx}" ] || return 1
    ensure_auth_dir
    printf '%s\n' "${_rx}" >"$(expiry_record_path "${1}")"
}

# Print credential $1's recorded expiry (Unix time); fail without a record.
read_expiry() {
    _rdx="$(expiry_record_path "${1}")"
    [ -f "${_rdx}" ] || return 1
    IFS= read -r _rdx <"${_rdx}" || [ -n "${_rdx}" ] || return 1
    case "${_rdx}" in '' | *[!0-9]*) return 1 ;; esac
    printf '%s' "${_rdx}"
}

# Print the expiry warning window in seconds: IDIOT_CRED_WARN_DAYS days,
# 7 when it isn't a non-negative integer (0: no early warning).
cred_warn_window() {
    _cww="${IDIOT_CRED_WARN_DAYS:-}"
    case "${_cww}" in '' | *[!0-9]*) _cww=7 ;; esac
    while case "${_cww}" in 0?*) true ;; *) false ;; esac do
        _cww="${_cww#0}" # leading zeros would read as octal
    done
    [ "${#_cww}" -le 6 ] || _cww=999999
    printf '%s' "$((_cww * 86400))"
}

# Print how far Unix time $1 is from now ($2, Unix time): `expires in <N>d`
# (whole days), `expires in <N>h`, `expires in <1h`, or `expired`.
expiry_phrase() {
    _xp=$((${1} - ${2}))
    if [ "${_xp}" -le 0 ]; then
        printf 'expired'
    elif [ "${_xp}" -ge 86400 ]; then
        printf 'expires in %sd' "$((_xp / 86400))"
    elif [ "${_xp}" -ge 3600 ]; then
        printf 'expires in %sh' "$((_xp / 3600))"
    else
        printf 'expires in <1h'
    fi
}

# Print the colour for an expiry phrase ($1 expiry, $2 now): red when
# expired, yellow within the warning window, else dim.
expiry_colour() {
    if [ "${1}" -le "${2}" ]; then
        printf '%s' "${RED}"
    elif [ $((${1} - ${2})) -lt "$(cred_warn_window)" ]; then
        printf '%s' "${YELLOW}"
    else
        printf '%s' "${DIM}"
    fi
}

# Say on stderr that credential $1 (expiring at $2) expires soon or has
# expired, as of $3 (Unix time); fail, saying nothing, when neither.
cred_expiry_notice() {
    _cn_left=$((${2} - ${3}))
    if [ "${_cn_left}" -le 0 ]; then
        _cn_ago=$((-_cn_left))
        if [ "${_cn_ago}" -ge 86400 ]; then
            _cn_ago="$((_cn_ago / 86400))d ago"
        elif [ "${_cn_ago}" -ge 3600 ]; then
            _cn_ago="$((_cn_ago / 3600))h ago"
        else
            _cn_ago='just now'
        fi
        say "${RED}idiot: credential '${1}' expired ${_cn_ago} — create a new one with: idiot auth add${RESET}"
    elif [ "${_cn_left}" -lt "$(cred_warn_window)" ]; then
        say "${YELLOW}idiot: credential '${1}' $(expiry_phrase "${2}" "${3}") — renew with: idiot auth add${RESET}"
    else
        return 1
    fi
}

# Write today's date (UTC, $1) to credential $2's warning stamp.
expiry_stamp() {
    _es="$(expiry_stamp_path "${2}")"
    mkdir -p "${_es%/*}" 2>/dev/null && printf '%s\n' "${1}" >"${_es}" 2>/dev/null
    return 0
}

# The dispatcher's once-a-day warning: when the active credential
# (IDIOT_ACTIVE_CRED, expiring at IDIOT_CRED_EXPIRES) has expired or
# expires within the window, and it wasn't warned about today in any shell,
# warn on stderr. Costs nothing without an active credential with a known
# expiry; never fails, never touches the network.
cred_expiry_warn() {
    case "${IDIOT_ACTIVE_CRED:-}" in '' | .* | */*) return 0 ;; esac
    case "${IDIOT_CRED_EXPIRES:-}" in '' | *[!0-9]*) return 0 ;; esac
    _cw_now="$(date -u '+%s %Y-%m-%d')" || return 0
    _cw_day="${_cw_now#* }"
    _cw_now="${_cw_now%% *}"
    _cw_stamp=''
    # the path inline (not expiry_stamp_path): no subshell on this path
    _cw_f="${IDIOT_CACHE_DIR}/expiry-warned/${IDIOT_ACTIVE_CRED}"
    [ ! -f "${_cw_f}" ] || IFS= read -r _cw_stamp <"${_cw_f}" || :
    [ "${_cw_stamp}" != "${_cw_day}" ] || return 0
    cred_expiry_notice "${IDIOT_ACTIVE_CRED}" "${IDIOT_CRED_EXPIRES}" "${_cw_now}" || return 0
    expiry_stamp "${_cw_day}" "${IDIOT_ACTIVE_CRED}"
}

# Print the path to the whoami cache file for credential $1.
whoami_cache_path() {
    printf '%s/whoami/%s' "${IDIOT_CACHE_DIR}" "${1}"
}

# Print the path of the idiot-store helper (see helper_path), or die. Use as
# `h="$(store_helper)" || exit 1`.
store_helper() {
    helper idiot-store
}

# The idiot-store helper's path, as store_helper finds it, or fail quietly.
store_helper_path() {
    helper_path idiot-store
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
