# shellcheck shell=sh
# share/profiles.sh - environment profile helpers
# Source this file; do not execute directly. Callers also include creds.sh,
# stores.sh and arch.sh (profiles name credentials, stores and arches).
#
# A profile is one file of `key: value` lines, keys from PROFILE_FIELDS. A
# key that is absent is not part of the profile; a key with an empty value
# clears that setting. The file is parsed, never sourced.

IDIOT_PROFILES_DIR="${IDIOT_PROFILES_DIR:-${IDIOT_DATA_HOME}/profiles}"

PROFILE_FIELDS='auth store arch lp'

# Die unless $1 is a valid profile name: letters, digits, '-' and '_'.
validate_profile_name() {
    case "${1}" in
    '' | *[!A-Za-z0-9_-]*)
        die "invalid profile name '${1}': use only letters, digits, hyphens, underscores"
        ;;
    esac
}

# Die unless a profile named $1 exists.
require_profile() {
    [ -f "${IDIOT_PROFILES_DIR}/${1}" ] || die "profile '${1}' not found"
}

# Die if a profile named $1 already exists.
require_no_profile() {
    [ -f "${IDIOT_PROFILES_DIR}/${1}" ] &&
        die "profile '${1}' already exists — use --force, or 'idiot profile remove ${1}' first"
}

# Succeed if $1 is one of PROFILE_FIELDS.
is_profile_field() {
    case " ${PROFILE_FIELDS} " in
    *" ${1} "*) [ -n "${1}" ] ;;
    *) return 1 ;;
    esac
}

# Read profile $1 into PF_HAS_<FIELD> (1 when the profile records it, else
# 0) and PF_<FIELD> (its value). Blank lines are skipped. Fails, with the
# reason in PF_ERROR, on an unknown key or a line without ':'.
read_profile() {
    PF_HAS_AUTH=0 PF_HAS_STORE=0 PF_HAS_ARCH=0 PF_HAS_LP=0
    PF_AUTH='' PF_STORE='' PF_ARCH='' PF_LP='' PF_ERROR=''
    while IFS= read -r _rp_line || [ -n "${_rp_line}" ]; do
        case "${_rp_line}" in
        '') continue ;;
        *:*) ;;
        *)
            PF_ERROR="malformed line '${_rp_line}'"
            return 1
            ;;
        esac
        _rp_key="${_rp_line%%:*}"
        _rp_val="${_rp_line#*:}"
        _rp_val="${_rp_val#"${_rp_val%%[! ]*}"}"
        _rp_val="${_rp_val%"${_rp_val##*[! ]}"}"
        case "${_rp_key}" in
        auth) PF_HAS_AUTH=1 PF_AUTH="${_rp_val}" ;;
        store) PF_HAS_STORE=1 PF_STORE="${_rp_val}" ;;
        arch) PF_HAS_ARCH=1 PF_ARCH="${_rp_val}" ;;
        lp) PF_HAS_LP=1 PF_LP="${_rp_val}" ;;
        *)
            PF_ERROR="unknown field '${_rp_key}'"
            return 1
            ;;
        esac
    done <"${IDIOT_PROFILES_DIR}/${1}"
}

# Print what is wrong with profile $1 and succeed, or fail when it can be
# used: it parses, and every credential, store and arch it names is valid
# and exists. Leaves the profile's fields set (see read_profile).
profile_problem() {
    if ! read_profile "${1}"; then
        printf '%s' "${PF_ERROR}"
        return 0
    fi
    if [ -n "${PF_AUTH}" ]; then
        case "${PF_AUTH}" in
        .* | *[!A-Za-z0-9._-]*)
            printf "invalid credential name '%s'" "${PF_AUTH}"
            return 0
            ;;
        esac
        if [ ! -f "${IDIOT_AUTH_DIR}/${PF_AUTH}" ]; then
            printf "credentials '%s' not found" "${PF_AUTH}"
            return 0
        fi
    fi
    if [ -n "${PF_STORE}" ]; then
        case "${PF_STORE}" in
        *[!A-Za-z0-9_-]*)
            printf "invalid store name '%s'" "${PF_STORE}"
            return 0
            ;;
        esac
        if [ ! -f "${IDIOT_STORES_DIR}/${PF_STORE}" ]; then
            printf "store '%s' not found" "${PF_STORE}"
            return 0
        fi
    fi
    if [ -n "${PF_ARCH}" ] && ! is_store_arch "${PF_ARCH}"; then
        printf "invalid architecture '%s'" "${PF_ARCH}"
        return 0
    fi
    return 1
}

# Print the fields of the profile last read, as `auth ci · store staging`:
# left-out fields are omitted, cleared ones shown as a dim "none".
profile_summary() {
    _ps=''
    for _ps_f in ${PROFILE_FIELDS}; do
        case "${_ps_f}" in
        auth) _ps_has="${PF_HAS_AUTH}" _ps_val="${PF_AUTH}" ;;
        store) _ps_has="${PF_HAS_STORE}" _ps_val="${PF_STORE}" ;;
        arch) _ps_has="${PF_HAS_ARCH}" _ps_val="${PF_ARCH}" ;;
        lp) _ps_has="${PF_HAS_LP}" _ps_val="${PF_LP}" ;;
        esac
        [ "${_ps_has}" -eq 1 ] || continue
        [ -n "${_ps_val}" ] || _ps_val="${DIM}none${RESET}"
        _ps="${_ps:+${_ps} · }${_ps_f} ${_ps_val}"
    done
    printf '%s' "${_ps:-${DIM}nothing recorded${RESET}}"
}

# Print, one per line as `field<TAB>recorded<TAB>now`, each field the
# profile last read records whose setting in this shell now differs (empty
# values read "none"; stores by name). Prints nothing when none differ.
profile_drift() {
    if [ "${PF_HAS_AUTH}" -eq 1 ] && [ "${PF_AUTH}" != "${IDIOT_ACTIVE_CRED:-}" ]; then
        printf 'auth\t%s\t%s\n' "${PF_AUTH:-none}" "${IDIOT_ACTIVE_CRED:-none}"
    fi
    if [ "${PF_HAS_STORE}" -eq 1 ]; then
        _pd_want=''
        [ -z "${PF_STORE}" ] ||
            _pd_want="$(cat "${IDIOT_STORES_DIR}/${PF_STORE}" 2>/dev/null)" ||
            _pd_want="?"
        if [ "${_pd_want}" != "${UBUNTU_STORE_ID:-}" ]; then
            _pd_now="$(store_name_for_id "${UBUNTU_STORE_ID:-}")" ||
                _pd_now="${UBUNTU_STORE_ID:-none}"
            printf 'store\t%s\t%s\n' "${PF_STORE:-none}" "${_pd_now}"
        fi
    fi
    if [ "${PF_HAS_ARCH}" -eq 1 ] && [ "${PF_ARCH}" != "${UBUNTU_STORE_ARCH:-}" ]; then
        printf 'arch\t%s\t%s\n' "${PF_ARCH:-none}" "${UBUNTU_STORE_ARCH:-none}"
    fi
    if [ "${PF_HAS_LP}" -eq 1 ] && [ "${PF_LP}" != "${IDIOT_LP_USER:-}" ]; then
        printf 'lp\t%s\t%s\n' "${PF_LP:-none}" "${IDIOT_LP_USER:-none}"
    fi
}
