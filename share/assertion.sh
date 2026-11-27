# shellcheck shell=sh
# share/assertion.sh - writing unsigned assertions (idiot assertion new/edit)
# Source this file; do not execute directly. Callers include creds.sh,
# arch.sh and models.sh first, and set work (a private mktemp -d).
#
# A draft is built as idiot-helper draft records in ${as_rec} (h/l header
# lines, s/v entries with their f fields, x raw values; see
# tools/idiot-helper/src/draft.rs) and turned into JSON by `draft write`.
# Values given with --set/--add/--snap are recorded first and never asked
# again; on a terminal everything else is asked on /dev/tty with its
# default in brackets.

# shellcheck disable=SC2154 # work, as_arch, cred, no_edit: set by the caller
AS_TAB="$(printf '\t')"

# Start an empty set of records.
as_init() {
    as_rec="${work}/records"
    : >"${as_rec}"
    as_given=' '
    as_tty=0
    if [ -t 0 ] && [ -t 2 ] && { : </dev/tty; } 2>/dev/null; then
        as_tty=1
    fi
    as_account_done=0
    as_account=''
}

# Append record $1 with fields $2 [$3]; dies on a tab or newline in them.
as_put() {
    case "${2}${3:-}" in
    *"${AS_TAB}"* | *'
'*) die "values can't hold a tab or a newline: ${2}" ;;
    esac
    if [ $# -ge 3 ]; then
        printf '%s\t%s\t%s\n' "${1}" "${2}" "${3}" >>"${as_rec}"
    else
        printf '%s\t%s\n' "${1}" "${2}" >>"${as_rec}"
    fi
}

# Print header $1's value; fails when it isn't set.
as_get() {
    awk -F '\t' -v k="${1}" '$1 == "h" && $2 == k { v = $3; f = 1 }
        END { if (f) print v; exit !f }' "${as_rec}"
}

# Succeed when header $1 has a value or list items.
as_has() {
    awk -F '\t' -v k="${1}" '($1 == "h" || $1 == "l" || $1 == "x") && $2 == k { f = 1 }
        END { exit !f }' "${as_rec}"
}

as_set() { as_put h "${1}" "${2}"; }
as_add() { as_put l "${1}" "${2}"; }

# Print list header $1's items, one per line.
as_list() {
    awk -F '\t' -v k="${1}" '$1 == "l" && $2 == k { print $3 }' "${as_rec}"
}

# Forget header $1 (scalar, list or raw).
as_unset() {
    awk -F '\t' -v k="${1}" '!(($1 == "h" || $1 == "l" || $1 == "x") && $2 == k)' \
        "${as_rec}" >"${as_rec}.new" && mv "${as_rec}.new" "${as_rec}"
}

# Mark header $1 as given on the command line (never asked).
as_mark_given() { as_given="${as_given}${1} "; }
as_is_given() {
    case "${as_given}" in
    *" ${1} "*) return 0 ;;
    esac
    return 1
}

# The snaps in the records, one name per line.
as_snap_names() {
    awk -F '\t' '$1 == "s" { print $2 }' "${as_rec}"
}

# Print field $2 of snap $1.
as_snap_field() {
    awk -F '\t' -v n="${1}" -v f="${2}" '
        $1 == "s" || $1 == "v" { cur = ($1 == "s" && $2 == n) }
        $1 == "h" || $1 == "l" { cur = 0 }
        cur && $1 == "f" && $2 == f { print $3; exit }' "${as_rec}"
}

# Remove snap $1's entry (its s line, f fields and x values).
as_drop_snap() {
    awk -F '\t' -v n="${1}" '
        $1 == "s" || $1 == "v" { cur = ($1 == "s" && $2 == n) }
        $1 == "h" || $1 == "l" { cur = 0 }
        $1 == "x" && index($2, "snaps[" n "].") == 1 { next }
        !(cur && ($1 == "s" || $1 == "f"))' "${as_rec}" >"${as_rec}.new" &&
        mv "${as_rec}.new" "${as_rec}"
}

# Parse one command-line answer: --set h=v, --add h=v. Dies on a bad one.
as_option() {
    case "${2}" in
    *=*) ;;
    *) die "${1} takes <header>=<value>, not '${2}'" ;;
    esac
    _ao_h="${2%%=*}"
    _ao_v="${2#*=}"
    [ -n "${_ao_h}" ] || die "${1} takes <header>=<value>, not '${2}'"
    case "${_ao_h}" in
    type | snaps) die "${_ao_h} can't be set with ${1}" ;;
    esac
    if [ "${1}" = --set ]; then
        as_set "${_ao_h}" "${_ao_v}"
    else
        as_add "${_ao_h}" "${_ao_v}"
    fi
    as_mark_given "${_ao_h}"
}

# Ask on the terminal for $1 with default $2 (shown in brackets; Enter
# takes it). Sets answer. End of input (Ctrl-D) cancels: exits zero,
# writing nothing.
as_ask() {
    if [ -n "${2:-}" ]; then
        printf '%s [%s]: ' "${1}" "${2}" >&2
    else
        printf '%s: ' "${1}" >&2
    fi
    if ! IFS= read -r answer </dev/tty; then
        echo >&2
        as_cancel
    fi
    answer="${answer:-${2:-}}"
}

# Header $1: the given value, else on a terminal the answer to prompt $2
# (default $3), else the default. Required ($4 = 1) headers can't be empty.
as_header() {
    if as_is_given "${1}"; then
        return 0
    fi
    # a value the records already hold (edit: from the file) is the default
    _ah_def="$(as_get "${1}" || printf '%s' "${3:-}")"
    if [ "${as_tty}" -eq 1 ]; then
        while :; do
            as_ask "${2}" "${_ah_def}"
            [ -n "${answer}" ] || [ "${4:-0}" -eq 0 ] || {
                say "${YELLOW}${1} is required${RESET}"
                continue
            }
            break
        done
    else
        answer="${_ah_def}"
    fi
    as_unset "${1}"
    [ -z "${answer}" ] || as_set "${1}" "${answer}"
}

# Die naming header $1 when it has no value and there is no terminal to
# ask; $2 says where a default would have come from.
as_require() {
    [ "${as_tty}" -eq 0 ] || return 0
    as_has "${1}" && return 0
    die "${1} is required${2:+ (${2})} — pass --set ${1}=<value>"
}

# The account ID behind the credential (query_cred), once; empty without
# one. Never prints the credential.
as_account_id() {
    if [ "${as_account_done}" -eq 0 ]; then
        as_account_done=1
        if query_cred "${cred:-}"; then
            _aa_h="$(store_helper)" || exit 1
            # shellcheck disable=SC2154 # query_file is set by query_cred
            as_account="$("${_aa_h}" whoami ${query_file:+--file "${query_file}"} 2>/dev/null |
                sed -n 's/^id: //p')" || as_account=''
            [ -n "${as_account}" ] ||
                say "${YELLOW}!${RESET} couldn't read the account behind the credential — asking for it"
        fi
    fi
    printf '%s' "${as_account}"
}

# Succeed when there is a credential to take account defaults from.
as_have_cred() {
    [ -n "${cred:-}" ] || [ -n "${UBUNTU_STORE_AUTH:-}" ]
}

# The current UTC time, RFC 3339.
as_now() {
    date -u +%Y-%m-%dT%H:%M:%SZ
}

# RFC 3339 time $1 one year later (29 February becomes 28 February).
as_year_later() {
    _ay="${1%%-*}"
    _ay_rest="${1#*-}"
    case "${_ay_rest}" in
    02-29*) _ay_rest="02-28${_ay_rest#02-29}" ;;
    esac
    printf '%s-%s' "$((_ay + 1))" "${_ay_rest}"
}

# The snap's channel with a track: a bare risk gets latest/.
as_track() {
    case "${1}" in
    */*) printf '%s' "${1}" ;;
    *) printf 'latest/%s' "${1}" ;;
    esac
}

# Look snap $1 up in the active store, as the credential sees it, into
# ${work}/info.$1. Returns 0 found, 4 when the store doesn't show it;
# dies on another error.
as_lookup() {
    _al_f="${work}/info.${1}"
    # looked up already: found, or the store doesn't show it
    [ ! -s "${_al_f}" ] || return 0
    [ ! -f "${_al_f}.none" ] || return 4
    _al_h="$(store_helper)" || exit 1
    query_cred "${cred:-}" || :
    _al_rc=0
    # shellcheck disable=SC2154 # query_file is set by query_cred
    "${_al_h}" info ${query_file:+--file "${query_file}"} \
        ${UBUNTU_STORE_ID:+--store "${UBUNTU_STORE_ID}"} "${1}" \
        >"${_al_f}" 2>"${work}/info.err" || _al_rc=$?
    [ "${_al_rc}" -eq 0 ] && return 0
    rm -f "${_al_f}"
    if grep -q 'not found in this store' "${work}/info.err"; then
        : >"${_al_f}.none"
        return 4
    fi
    die "can't look ${1} up in the store: $(sed 's/^idiot-store: //' "${work}/info.err" | head -n 1)"
}

# Print snap $1's channels with a revision for ${as_arch}, $2 first.
as_channels() {
    awk -F '\t' -v a="${as_arch}" '$1 == "channel" && $3 == a { print $2 }' \
        "${work}/info.${1}" | awk -v d="${2:-}" '$0 == d { f = 1; next } { l[++n] = $0 }
            END { if (f) print d; for (i = 1; i <= n; i++) print l[i] }'
}

# Say nothing was written and exit zero (a cancelled prompt or picker).
as_cancel() {
    say "${DIM}Cancelled: nothing written.${RESET}"
    exit 0
}

# Pick snap $1's channel on the terminal, default $2 first. Prints it;
# fails when the picker is cancelled.
as_pick_channel() {
    _apc="$(as_channels "${1}" "${2}")"
    [ -n "${_apc}" ] || die "${1} has no ${as_arch} revision in any channel"
    printf '%s\n' "${_apc}" | fzf --no-sort --prompt="${1} channel> " \
        --header="channels with an ${as_arch} revision"
}

# The channel line ($2 = field) of snap $1 in channel $2 for ${as_arch}.
as_channel_field() {
    awk -F '\t' -v c="${2}" -v a="${as_arch}" -v f="${3}" \
        '$1 == "channel" && $2 == c && $3 == a { print $f; exit }' "${work}/info.${1}"
}

# Die saying snap $1 has no ${as_arch} revision in channel $2.
as_not_released() {
    _anr="$(as_channels "${1}" | paste -sd ' ' - | sed 's/ /, /g')"
    die "${1} has no ${as_arch} revision in ${2}${_anr:+ — it has one in: ${_anr}}"
}

# Ask for a snap name on the terminal, offering the snaps the credential
# sees in the active store; prints the name, or nothing when the user is
# done ($1: the prompt).
as_pick_snap() {
    if [ ! -f "${work}/store-snaps" ]; then
        "${IDIOT_ROOT}/cmd/store/snaps" --tsv ${cred:+--cred "${cred}"} \
            2>/dev/null | cut -f 1 >"${work}/store-snaps" || :
    fi
    _aps="$(fzf --print-query --prompt="${1}> " \
        --header='type a name or pick one; Enter on an empty line: done' \
        <"${work}/store-snaps")" || :
    _aps_q="$(printf '%s\n' "${_aps}" | sed -n 1p)"
    _aps_p="$(printf '%s\n' "${_aps}" | sed -n 2p)"
    printf '%s' "${_aps_p:-${_aps_q}}"
}

# Add snap $1 to a model, in channel $2 (empty: pick on a terminal, else
# latest/stable). Fills its id, type and default-channel from the store;
# essential snaps get no presence, others presence required. $3 is the
# type to give a snap the store doesn't show (an essential snap's role).
as_model_snap() {
    _ams_grade="$(as_get grade || printf signed)"
    _ams_base="$(as_get base || :)"
    _ams_rc=0
    as_lookup "${1}" || _ams_rc=$?
    as_drop_snap "${1}"
    if [ "${_ams_rc}" -eq 4 ]; then
        [ "${_ams_grade}" = dangerous ] ||
            die "the store doesn't show ${1} — find out why: idiot store check ${1} (or use a dangerous model)"
        say "${YELLOW}!${RESET} ${1}: no id — the store doesn't show this snap (allowed in a dangerous model)"
        as_put s "${1}"
        _ams_type="${3:-}"
        [ "${1}" != "${_ams_base}" ] || _ams_type=base
        [ -z "${_ams_type}" ] || as_put f type "${_ams_type}"
        as_put f default-channel "$(as_track "${2:-latest/stable}")"
        case "${_ams_type}" in
        snapd | kernel | gadget | base) ;;
        *) as_put f presence required ;;
        esac
        return 0
    fi
    _ams_ch="${2:-}"
    if [ -z "${_ams_ch}" ]; then
        if [ "${as_tty}" -eq 1 ]; then
            _ams_ch="$(as_pick_channel "${1}" latest/stable)" || as_cancel
        else
            _ams_ch=latest/stable
        fi
    fi
    _ams_ch="$(as_track "${_ams_ch}")"
    _ams_rev="$(as_channel_field "${1}" "${_ams_ch}" 4)"
    [ -n "${_ams_rev}" ] || as_not_released "${1}" "${_ams_ch}"
    _ams_type="$(as_channel_field "${1}" "${_ams_ch}" 7)"
    [ -n "${_ams_type}" ] || _ams_type=app
    # snapd's model calls a classic core "core"; the store says "os"
    [ "${_ams_type}" != os ] || _ams_type=core
    as_put s "${1}"
    as_put f id "$(awk -F '\t' '$1 == "snap" { print $3; exit }' "${work}/info.${1}")"
    as_put f type "${_ams_type}"
    as_put f default-channel "${_ams_ch}"
    case "${_ams_type}" in
    snapd | kernel | gadget) ;;
    *) [ "${1}" = "${_ams_base}" ] || as_put f presence required ;;
    esac
}

# Add snap $1 to a validation set, at the ${as_arch} revision in channel
# $2 (empty: pick on a terminal, else latest/stable), or as invalid
# ($2 = invalid). Warns when another architecture has another revision
# there.
as_vs_snap() {
    _avs_rc=0
    as_lookup "${1}" || _avs_rc=$?
    [ "${_avs_rc}" -eq 0 ] ||
        die "the store doesn't show ${1} — find out why: idiot store check ${1}"
    as_drop_snap "${1}"
    _avs_id="$(awk -F '\t' '$1 == "snap" { print $3; exit }' "${work}/info.${1}")"
    if [ "${2:-}" = invalid ]; then
        as_put s "${1}"
        as_put f id "${_avs_id}"
        as_put f presence invalid
        return 0
    fi
    _avs_ch="${2:-}"
    if [ -z "${_avs_ch}" ]; then
        if [ "${as_tty}" -eq 1 ]; then
            _avs_ch="$(as_pick_channel "${1}" latest/stable)" || as_cancel
        else
            _avs_ch=latest/stable
        fi
    fi
    _avs_ch="$(as_track "${_avs_ch}")"
    _avs_rev="$(as_channel_field "${1}" "${_avs_ch}" 4)"
    [ -n "${_avs_rev}" ] || as_not_released "${1}" "${_avs_ch}"
    _avs_other="$(awk -F '\t' -v c="${_avs_ch}" -v a="${as_arch}" -v r="${_avs_rev}" \
        '$1 == "channel" && $2 == c && $3 != a && $4 != r { printf "%s%s %s", s, $3, $4; s = ", " }' \
        "${work}/info.${1}")"
    [ -z "${_avs_other}" ] ||
        say "${YELLOW}!${RESET} ${1}: revision ${_avs_rev} is ${as_arch}'s in ${_avs_ch}; other architectures have ${_avs_other} — a validation set's revision holds for all of them"
    as_put s "${1}"
    as_put f id "${_avs_id}"
    as_put f presence required
    as_put f revision "${_avs_rev}"
}

# The essential snaps for base $1 on ${as_arch}: "role name channel" lines
# (no name: ask, no default).
as_essentials() {
    _ae_track="${1#core}"
    case "${_ae_track}" in
    '' | *[!0-9]*) _ae_track='' ;;
    esac
    printf 'snapd snapd latest/stable\n'
    printf 'base %s latest/stable\n' "${1}"
    case "${as_arch}" in
    amd64) _ae_g=pc _ae_k=pc-kernel ;;
    arm64 | armhf) _ae_g=pi _ae_k=pi-kernel ;;
    *) _ae_g='' _ae_k='' ;;
    esac
    if [ -n "${_ae_g}" ] && [ -n "${_ae_track}" ]; then
        printf 'gadget %s %s/stable\nkernel %s %s/stable\n' \
            "${_ae_g}" "${_ae_track}" "${_ae_k}" "${_ae_track}"
    else
        printf 'gadget\nkernel\n'
    fi
}

# The --snap answers (name[=channel] lines in ${as_snaps_given}), each
# looked up and added with function $1 (as_model_snap or as_vs_snap).
as_given_snaps() {
    [ -n "${as_snaps_given:-}" ] || return 0
    while IFS= read -r _ags; do
        [ -n "${_ags}" ] || continue
        _ags_n="${_ags%%=*}"
        _ags_c=''
        [ "${_ags_n}" = "${_ags}" ] || _ags_c="${_ags#*=}"
        # edit: a snap the file has is updated in place, keeping its fields
        if [ "${as_mode:-new}" = edit ] && as_snap_names | grep -qx -- "${_ags_n}"; then
            as_refresh_snap "${_ags_n}" "${_ags_c}"
        else
            "${1}" "${_ags_n}" "${_ags_c}"
        fi
    done <<EOF
${as_snaps_given}
EOF
}

# The model's snaps: the --snap ones, then each essential snap not yet
# there (proposed on a terminal, where it can be replaced), then on a
# terminal more snaps until an empty answer.
as_model_snaps() {
    as_given_snaps as_model_snap
    _ams_base="$(as_get base)"
    as_essentials "${_ams_base}" >"${work}/essentials"
    while read -r _role _name _chan; do
        # already there: by name for snapd and the base, by type otherwise
        case "${_role}" in
        snapd | base)
            as_snap_names | grep -qx -- "${_name}" && continue
            ;;
        *)
            awk -F '\t' -v t="${_role}" '$1 == "f" && $2 == "type" && $3 == t { f = 1 } END { exit !f }' \
                "${as_rec}" && continue
            ;;
        esac
        if [ "${as_tty}" -eq 1 ]; then
            while :; do
                as_ask "${_role} snap (name or name=channel)" "${_name:+${_name}=${_chan}}"
                [ -n "${answer}" ] && break
                say "${YELLOW}a model needs a ${_role}${RESET}"
            done
            case "${answer}" in
            *=*) as_model_snap "${answer%%=*}" "${answer#*=}" "${_role}" ;;
            *) as_model_snap "${answer}" "$([ "${answer}" = "${_name}" ] && printf '%s' "${_chan}")" "${_role}" ;;
            esac
        elif [ -n "${_name}" ]; then
            as_model_snap "${_name}" "${_chan}" "${_role}"
        else
            die "no default ${_role} for ${as_arch} — pass --snap <${_role}>=<channel>"
        fi
    done <"${work}/essentials"
    [ "${as_tty}" -eq 1 ] || return 0
    while :; do
        _amsn="$(as_pick_snap 'add a snap')"
        [ -n "${_amsn}" ] || break
        as_model_snap "${_amsn}" ''
    done
}

# The validation set's snaps: the --snap ones, then on a terminal more
# until an empty answer.
as_vs_snaps() {
    as_given_snaps as_vs_snap
    [ "${as_tty}" -eq 1 ] || return 0
    while :; do
        _avsn="$(as_pick_snap 'add a snap')"
        [ -n "${_avsn}" ] || break
        as_vs_snap "${_avsn}" ''
    done
}

# The latest sequence the store has for validation set $1/$2: prints it,
# 0 when the store has none; fails (printing nothing) when it can't be
# asked.
as_latest_sequence() {
    _als_h="$(store_helper)" || return 1
    _als_rc=0
    "${_als_h}" assertion validation-set "account-id=${1}" "name=${2}" \
        >"${work}/vs.latest" 2>/dev/null || _als_rc=$?
    case "${_als_rc}" in
    0) awk '/^$/ { exit } /^sequence: / { print $2; exit }' "${work}/vs.latest" ;;
    4) printf '0' ;;
    *) return 1 ;;
    esac
}

# Print the problems `draft check` finds in draft $1 on stderr; fails when
# there is an error.
as_check() {
    _ac_h="$(helper idiot-helper)" || exit 1
    _ac_rc=0
    "${_ac_h}" draft check "${1}" >"${work}/problems" || _ac_rc=$?
    while IFS="${AS_TAB}" read -r _ac_level _ac_at _ac_msg; do
        case "${_ac_level}" in
        error)
            [ "${_ac_at}" != - ] || _ac_at=''
            say "  ${RED}✗${RESET} ${_ac_at:+${BOLD}${_ac_at}${RESET} }${_ac_msg}"
            ;;
        warning) say "  ${YELLOW}!${RESET} ${BOLD}${_ac_at}${RESET} ${_ac_msg}" ;;
        esac
    done <"${work}/problems"
    return "${_ac_rc}"
}

# The editor to run: VISUAL, EDITOR, editor, then vi.
as_editor() {
    if [ -n "${VISUAL:-}" ]; then
        printf '%s' "${VISUAL}"
    elif [ -n "${EDITOR:-}" ]; then
        printf '%s' "${EDITOR}"
    elif command -v editor >/dev/null 2>&1; then
        printf editor
    else
        printf vi
    fi
}

# Turn the records into the draft ${work}/draft.json (type $1), then,
# unless no_edit is 1, open it in the editor until it passes the check or
# the user gives up (exit 1, nothing written).
as_build() {
    _ab_h="$(helper idiot-helper)" || exit 1
    "${_ab_h}" draft write "${1}" <"${as_rec}" >"${work}/draft.json" 2>"${work}/write.err" ||
        die "$(sed 's/^idiot-helper draft: //' "${work}/write.err")"
    if [ "${no_edit:-0}" -eq 1 ] || [ "${as_tty}" -eq 0 ]; then
        as_check "${work}/draft.json" ||
            die "the draft has errors — nothing written"
        return 0
    fi
    _ab_ed="$(as_editor)"
    while :; do
        # shellcheck disable=SC2016 # $1 is the editor's shell's
        sh -c "${_ab_ed}"' "$1"' sh "${work}/draft.json" </dev/tty >/dev/tty ||
            say "${YELLOW}!${RESET} the editor exited with an error"
        as_check "${work}/draft.json" && return 0
        as_ask "The draft has errors (above). Edit again? [Y/n]" ''
        case "${answer}" in
        n* | N*) die "nothing written" ;;
        esac
    done
}

# Write draft $1 to $2 (- for stdout): through a temporary file beside it,
# refusing to replace an existing file unless $3 is 1. Says where.
as_write() {
    if [ "${2}" = - ]; then
        cat "${1}"
        return 0
    fi
    if [ "${3:-0}" -ne 1 ] && [ -e "${2}" ]; then
        die "${2} already exists — pick another with -o <file>"
    fi
    _aw_dir="$(dirname "${2}")"
    _aw_tmp="$(mktemp "${_aw_dir}/.idiot-assertion.XXXXXX")" ||
        die "can't write in ${_aw_dir}"
    # mktemp makes it 0600: give it the mode the umask gives a new file
    chmod "$(printf '%o' "$((0666 & ~$(umask)))")" "${_aw_tmp}" 2>/dev/null || :
    cat "${1}" >"${_aw_tmp}" || {
        rm -f "${_aw_tmp}"
        die "can't write ${2}"
    }
    # checked again just before: no clobbering a file made meanwhile
    if [ "${3:-0}" -ne 1 ] && [ -e "${2}" ]; then
        rm -f "${_aw_tmp}"
        die "${2} already exists — pick another with -o <file>"
    fi
    mv -f "${_aw_tmp}" "${2}" || {
        rm -f "${_aw_tmp}"
        die "can't write ${2}"
    }
    say "Wrote ${BOLD}${2}${RESET}"
}

# The model names saved in model repositories for brand $1, one per line.
as_saved_models() {
    [ -d "${IDIOT_MODELS_DIR}" ] || return 0
    find "${IDIOT_MODELS_DIR}" -name '*.model' -type f 2>/dev/null |
        while IFS= read -r _asm; do
            awk -v b="${1}" '/^$/ { exit } /^brand-id: / { br = $2 } /^model: / { m = $2 }
                END { if (br == b && m != "") print m }' "${_asm}"
        done | sort -u
}

# Ask twice, without echo, for a password; hash it with openssl's SHA-512
# crypt and record only the hash. The password is never in argv, the
# environment or a file.
as_password() {
    command -v openssl >/dev/null 2>&1 ||
        die "openssl is needed to hash a password — install it (apt install openssl), or use SSH keys instead"
    _ap_n=0
    while :; do
        as_noecho=1
        stty -echo </dev/tty
        printf 'Password: ' >&2
        IFS= read -r _ap_pw </dev/tty || _ap_pw=''
        printf '\nAgain: ' >&2
        IFS= read -r _ap_pw2 </dev/tty || _ap_pw2=''
        stty echo </dev/tty
        as_noecho=0
        echo >&2
        if [ -n "${_ap_pw}" ] && [ "${_ap_pw}" = "${_ap_pw2}" ]; then
            break
        fi
        _ap_pw='' _ap_pw2=''
        _ap_n=$((_ap_n + 1))
        [ "${_ap_n}" -lt 3 ] || die "the passwords didn't match — nothing written"
        say "${YELLOW}they didn't match (or were empty)${RESET} — again"
    done
    # printf is a builtin: the password reaches openssl's stdin only
    _ap_hash="$(printf '%s\n' "${_ap_pw}" | openssl passwd -6 -stdin)" || _ap_hash=''
    _ap_pw='' _ap_pw2=''
    unset _ap_pw _ap_pw2
    [ -n "${_ap_hash}" ] || die "openssl couldn't hash the password"
    as_set password "${_ap_hash}"
    _ap_hash=''
}

# Restore the terminal's echo if a password prompt was interrupted.
as_restore_tty() {
    [ "${as_noecho:-0}" -eq 0 ] || stty echo </dev/tty 2>/dev/null || :
}

# ---- headers per type (new and edit) --------------------------------------
# as_mode is new or edit: edit takes the file's values as defaults (as_header
# does that for any header already in the records) and moves timestamps on.

# Set field $2 of snap $1 to $3, in place (added after its s line when
# missing).
as_set_snap_field() {
    awk -F '\t' -v n="${1}" -v f="${2}" -v v="${3}" '
        function flush() { if (cur && !done) print "f\t" f "\t" v; done = 1 }
        $1 == "s" || $1 == "v" || $1 == "h" || $1 == "l" || $1 == "x" { if (cur) flush(); cur = 0 }
        $1 == "s" && $2 == n { cur = 1; done = 0; print; next }
        cur && $1 == "f" && $2 == f { if (!done) print "f\t" f "\t" v; done = 1; next }
        { print }
        END { if (cur) flush() }' "${as_rec}" >"${as_rec}.new" &&
        mv "${as_rec}.new" "${as_rec}"
}

# Remove field $2 of snap $1.
as_unset_snap_field() {
    awk -F '\t' -v n="${1}" -v f="${2}" '
        $1 == "s" || $1 == "v" || $1 == "h" || $1 == "l" { cur = ($1 == "s" && $2 == n) }
        !(cur && $1 == "f" && $2 == f)' "${as_rec}" >"${as_rec}.new" &&
        mv "${as_rec}.new" "${as_rec}"
}

# Look snap $1 (already in the records) up again and update it in place,
# keeping the fields idiot doesn't set: a model snap's id and type (and
# default-channel when channel $2 is given); a validation-set snap's id
# and, from channel $2 (asked on a terminal), its revision, or with
# $2 = invalid its presence. Without a channel off a terminal a
# validation-set snap keeps its revision.
as_refresh_snap() {
    _ars_rc=0
    as_lookup "${1}" || _ars_rc=$?
    _ars_type="$(as_get type)"
    if [ "${_ars_rc}" -eq 4 ]; then
        if [ "${_ars_type}" = model ] && [ "$(as_get grade || :)" = dangerous ]; then
            say "${YELLOW}!${RESET} ${1}: the store doesn't show this snap — kept as it is"
            return 0
        fi
        die "the store doesn't show ${1} — find out why: idiot store check ${1}"
    fi
    as_set_snap_field "${1}" id "$(awk -F '\t' '$1 == "snap" { print $3; exit }' "${work}/info.${1}")"
    if [ "${_ars_type}" = model ]; then
        _ars_ch="$(as_snap_field "${1}" default-channel)"
        if [ -n "${2:-}" ]; then
            _ars_ch="$(as_track "${2}")"
            [ -n "$(as_channel_field "${1}" "${_ars_ch}" 4)" ] || as_not_released "${1}" "${_ars_ch}"
            as_set_snap_field "${1}" default-channel "${_ars_ch}"
        fi
        _ars_t="$(as_channel_field "${1}" "${_ars_ch:-latest/stable}" 7)"
        [ "${_ars_t}" != os ] || _ars_t=core
        if [ -n "${_ars_t}" ]; then
            as_set_snap_field "${1}" type "${_ars_t}"
        else
            say "${YELLOW}!${RESET} ${1}: no ${as_arch} revision in ${_ars_ch:-latest/stable} — its type is kept"
        fi
        return 0
    fi
    _ars_ch="${2:-}"
    if [ "${_ars_ch}" = invalid ]; then
        as_set_snap_field "${1}" presence invalid
        as_unset_snap_field "${1}" revision
        return 0
    fi
    if [ -z "${_ars_ch}" ] && [ "${as_tty}" -eq 1 ]; then
        _ars_ch="$(as_pick_channel "${1}" latest/stable)" || as_cancel
    fi
    [ -n "${_ars_ch}" ] || return 0
    _ars_ch="$(as_track "${_ars_ch}")"
    _ars_rev="$(as_channel_field "${1}" "${_ars_ch}" 4)"
    [ -n "${_ars_rev}" ] || as_not_released "${1}" "${_ars_ch}"
    [ "$(as_snap_field "${1}" presence)" != invalid ] || as_set_snap_field "${1}" presence required
    as_set_snap_field "${1}" revision "${_ars_rev}"
}

# brand-id/account-id ($1, prompt $2) and authority-id, defaulting to the
# account behind the credential.
as_account_headers() {
    _ah_acct=''
    as_is_given "${1}" || as_has "${1}" || _ah_acct="$(as_account_id)"
    as_header "${1}" "${2}" "${_ah_acct}" 1
    as_require "${1}" "no credential to take it from"
    as_header authority-id "Signed by (authority-id)" "$(as_get "${1}")" 1
}

# A timestamp header $1: now, unless given (new: unless given; edit: moved
# on to now).
as_timestamp() {
    as_is_given "${1}" || as_set "${1}" "$(as_now)"
}

as_model_headers() {
    as_is_given series || as_has series || as_set series 16
    if [ -n "${arch:-}" ] && ! as_is_given architecture; then
        as_unset architecture
        as_set architecture "${arch}"
        as_mark_given architecture
    fi
    as_header model "Model name" '' 1
    as_header display-name "Display name (optional)" ''
    as_account_headers brand-id "Brand (account ID)"
    as_header architecture "Architecture" "${as_arch}" 1
    as_arch="$(as_get architecture)"
    as_header base "Base" core24 1
    as_header grade "Grade (dangerous, signed or secured)" signed 1
    # the global store has no store header
    if [ -n "${UBUNTU_STORE_ID:-}" ] || as_has store; then
        as_header store "Store" "${UBUNTU_STORE_ID:-}"
    fi
    if [ "${as_mode}" = edit ] && ! as_is_given revision; then
        _amh_rev="$(as_get revision || printf 0)"
        case "${_amh_rev}" in
        '' | *[!0-9]*) _amh_rev=0 ;;
        esac
        _amh_yes=y
        if [ "${as_tty}" -eq 1 ]; then
            as_ask "Raise the model's revision to $((_amh_rev + 1))? [Y/n]" ''
            _amh_yes="${answer}"
        fi
        case "${_amh_yes}" in
        n* | N*) ;;
        *)
            as_unset revision
            as_set revision "$((_amh_rev + 1))"
            ;;
        esac
    fi
    as_timestamp timestamp
}

as_system_user_headers() {
    as_header email "Email" '' 1
    as_header username "Username" '' 1
    as_header name "Name (optional)" ''
    as_account_headers brand-id "Brand (account ID)"
    as_is_given series || as_has series || as_add series 16
    if ! as_is_given models && ! as_has models && [ "${as_tty}" -eq 1 ]; then
        _ash_saved="$(as_saved_models "$(as_get brand-id)" | paste -sd ' ' - | sed 's/ /, /g')"
        [ -z "${_ash_saved}" ] || say "${DIM}Saved models for $(as_get brand-id): ${_ash_saved}${RESET}"
        as_ask "Models it is for (comma-separated; empty: any of the brand's models, for at most a year)" ''
        printf '%s\n' "${answer}" | tr ',' '\n' | sed 's/^ *//; s/ *$//; /^$/d' >"${work}/models"
        while IFS= read -r _ash_m; do as_add models "${_ash_m}"; done <"${work}/models"
    fi
    if ! as_is_given ssh-keys && ! as_has ssh-keys && [ "${as_tty}" -eq 1 ]; then
        for _ash_k in "${HOME}"/.ssh/*.pub; do
            [ -f "${_ash_k}" ] || continue
            as_ask "Include ${_ash_k##*/} ($(cut -d ' ' -f 3- "${_ash_k}"))? [Y/n]" ''
            case "${answer}" in
            n* | N*) ;;
            *) as_add ssh-keys "$(head -n 1 "${_ash_k}")" ;;
            esac
        done
    fi
    if ! as_is_given password && [ "${as_tty}" -eq 1 ]; then
        if as_has password; then
            as_ask "Set a new password? [y/N]" ''
        else
            as_ask "Set a password too? SSH keys are the usual way in [y/N]" ''
        fi
        case "${answer}" in
        y* | Y*)
            as_unset password
            as_password
            ;;
        esac
    fi
    as_header since "Valid from" "$(as_now)" 1
    as_header until "Valid until" "$(as_year_later "$(as_get since)")" 1
    if ! as_is_given format; then
        _ash_f="$(as_get format || printf 0)"
        if as_has user-presence && [ "${_ash_f}" -lt 2 ] 2>/dev/null; then
            as_unset format
            as_set format 2
        elif as_has serials && [ "${_ash_f}" -lt 1 ] 2>/dev/null; then
            as_unset format
            as_set format 1
        fi
    fi
}

# The store's revision of validation set $1/$2 sequence $3 (0 when it
# has none or no revision header); fails when it can't be asked.
as_store_vs_revision() {
    _asr_h="$(store_helper)" || return 1
    _asr_rc=0
    "${_asr_h}" assertion validation-set "account-id=${1}" "name=${2}" "sequence=${3}" \
        >"${work}/vs.seq" 2>/dev/null || _asr_rc=$?
    case "${_asr_rc}" in
    0) awk '/^$/ { exit } /^revision: / { r = $2 } END { print r + 0 }' "${work}/vs.seq" ;;
    4) printf '0' ;;
    *) return 1 ;;
    esac
}

as_vs_headers() {
    as_is_given series || as_has series || as_set series 16
    as_account_headers account-id "Account ID"
    as_header name "Name" '' 1
    _avh_acct="$(as_get account-id)"
    _avh_name="$(as_get name)"
    if [ "${as_mode}" = new ] && ! as_is_given sequence; then
        if _avh_latest="$(as_latest_sequence "${_avh_acct}" "${_avh_name}")" &&
            [ -n "${_avh_latest}" ]; then
            _avh_seq=$((_avh_latest + 1))
        else
            say "${YELLOW}!${RESET} couldn't ask the store for the latest sequence of ${_avh_acct}/${_avh_name} — defaulting to 1"
            _avh_seq=1
        fi
        as_header sequence "Sequence" "${_avh_seq}" 1
    elif [ "${as_mode}" = edit ] && ! as_is_given sequence && ! as_is_given revision; then
        _avh_seq="$(as_get sequence || printf 1)"
        _avh_rev="$(as_get revision || printf 0)"
        _avh_store="$(as_store_vs_revision "${_avh_acct}" "${_avh_name}" "${_avh_seq}")" || _avh_store=0
        [ "${_avh_store}" -le "${_avh_rev}" ] 2>/dev/null || _avh_rev="${_avh_store}"
        _avh_latest="$(as_latest_sequence "${_avh_acct}" "${_avh_name}")" || _avh_latest=0
        [ "${_avh_latest}" -ge "${_avh_seq}" ] 2>/dev/null || _avh_latest="${_avh_seq}"
        _avh_pick=r
        if [ "${as_tty}" -eq 1 ]; then
            as_ask "New revision $((_avh_rev + 1)) of sequence ${_avh_seq} [r], or new sequence $((_avh_latest + 1)) [s]?" r
            _avh_pick="${answer}"
        fi
        case "${_avh_pick}" in
        s* | S*)
            as_unset sequence
            as_set sequence "$((_avh_latest + 1))"
            as_unset revision
            ;;
        *)
            as_unset revision
            as_set revision "$((_avh_rev + 1))"
            ;;
        esac
    fi
    as_timestamp timestamp
}

# The default file name for draft $1: <model>.json, <username>.json or
# <name>-<sequence>.json.
as_default_name() {
    "$(helper idiot-helper)" draft read "${1}" | awk -F '\t' '
        $1 == "t" { t = $2 }
        $1 == "h" { h[$2] = $3 }
        END {
            if (t == "model") print h["model"] ".json"
            else if (t == "system-user") print h["username"] ".json"
            else print h["name"] "-" h["sequence"] ".json"
        }'
}

# Fetch assertion $1 (model or validation-set) by key $2 from the active
# store, as idiot store assertion does (the active or --cred credential),
# into ${work}/source. Exits with its message when the store doesn't
# serve it.
as_source_fetch() {
    case "${1}" in
    model | validation-set) ;;
    *) die "only models and validation sets can be fetched from the store, not ${1}" ;;
    esac
    _asf_rc=0
    "${IDIOT_ROOT}/cmd/store/assertion" --raw ${cred:+--cred "${cred}"} "${1}" "${2}" \
        >"${work}/source" || _asf_rc=$?
    [ "${_asf_rc}" -eq 0 ] || exit "${_asf_rc}"
    as_src_from="the store"
}

# Copy source $1 (a file, or - for stdin) to ${work}/source.
as_source_file() {
    if [ "${1}" = - ]; then
        cat >"${work}/source" || die "can't read stdin"
        as_src_from=stdin
    else
        [ -f "${1}" ] || die "file not found: ${1}"
        cp "${1}" "${work}/source" || die "can't read ${1}"
        as_src_from="${1}"
    fi
}

# Read ${work}/source (a draft or a signed assertion) into records in
# ${work}/source.records (without the t and o records). Sets as_src_kind
# (signed or draft), as_src_type, as_src_keys and as_src_rev.
as_source_read() {
    _asr_h="$(helper idiot-helper)" || exit 1
    "${_asr_h}" draft read "${work}/source" >"${work}/source.read" 2>"${work}/source.err" ||
        die "${as_src_from}: $(sed 's/^idiot-helper draft: //' "${work}/source.err")"
    as_src_type="$(sed -n '1s/^t\t//p' "${work}/source.read")"
    _asr_o="$(sed -n '2s/^o\t//p' "${work}/source.read")"
    as_src_kind="$(printf '%s' "${_asr_o}" | cut -f 1)"
    as_src_keys="$(printf '%s' "${_asr_o}" | cut -f 3)"
    as_src_rev="$(printf '%s' "${_asr_o}" | cut -f 4)"
    sed '/^[to]\t/d' "${work}/source.read" >"${work}/source.records"
}

# Say on stderr which assertion the draft starts from.
as_source_say() {
    say "${DIM}Starting from ${as_src_type} ${as_src_keys} revision ${as_src_rev} (${as_src_kind}, from ${as_src_from})${RESET}"
}

# Forget what makes a template some other assertion (new --from): its
# identity, revision and times, which new then asks for or defaults.
as_template_strip() {
    case "${1}" in
    model) _ats='authority-id brand-id model display-name revision timestamp' ;;
    validation-set) _ats='authority-id account-id name sequence revision timestamp' ;;
    system-user) _ats='authority-id brand-id username email name password since until revision timestamp' ;;
    esac
    for _ats_h in ${_ats}; do as_unset "${_ats_h}"; done
}
