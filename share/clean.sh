# shellcheck shell=sh
# share/clean.sh - listing, confirmation and removal for `idiot clean`
# Source this file; do not execute directly.

# Print the names of instances in snapcraft's LXD project, one per line.
# Callers check `require lxc` first: a die in here would only exit the
# command substitution it runs in.
clean_list_containers() {
    lxc list --project snapcraft --format csv --columns n 2>/dev/null
}

# Print the names of snaps installed from local files (sideloaded, x<N>
# revisions), one per line. Callers check `require snap` first.
clean_list_snaps() {
    snap list 2>/dev/null | awk 'NR>1 && $3 ~ /^x[0-9]+$/ {print $1}'
}

# Print heading $1, then each newline-separated name in $2, indented.
clean_show() {
    [ -n "${2}" ] || return 0
    say "${1}"
    while IFS= read -r _cs_name; do
        say "  ${BOLD}${_cs_name}${RESET}"
    done <<EOF
${2}
EOF
}

# Ask "$1 [y/N]" on the terminal; succeed only on an explicit yes. Reads
# /dev/tty, not stdin, and dies when there is no terminal to ask on.
clean_confirm() {
    # In a subshell: a failed redirection on a special builtin like `:`
    # would make dash exit outright, silently, with status 2.
    (: </dev/tty) 2>/dev/null ||
        die "no terminal to confirm on — re-run with --yes"
    printf '%s [y/N] ' "${1}" >&2
    read -r _cc_answer </dev/tty || _cc_answer=''
    case "${_cc_answer}" in
    y | Y | yes | Yes | YES) return 0 ;;
    esac
    return 1
}

# Delete each container named in $1, showing lxc's errors. Returns 1 if
# any deletion failed.
clean_remove_containers() {
    _crc_failed=0
    while IFS= read -r _crc_name; do
        [ -n "${_crc_name}" ] || continue
        say "Deleting ${BOLD}${_crc_name}${RESET}..."
        lxc delete --project snapcraft --force "${_crc_name}" \
            </dev/null >&2 || _crc_failed=1
    done <<EOF
${1}
EOF
    return "${_crc_failed}"
}

# Remove each snap named in $1, showing snapd's errors (e.g. a refusal
# without root). Returns 1 if any removal failed.
clean_remove_snaps() {
    _crs_failed=0
    while IFS= read -r _crs_name; do
        [ -n "${_crs_name}" ] || continue
        say "Removing ${BOLD}${_crs_name}${RESET}..."
        snap remove "${_crs_name}" </dev/null >&2 || _crs_failed=1
    done <<EOF
${1}
EOF
    return "${_crs_failed}"
}
