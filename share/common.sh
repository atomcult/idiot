#!/usr/bin/env sh
# share/common.sh - shared helpers for idiot subcommands
# Source this file; do not execute directly.

# Exit normally on Ctrl-C. A command killed by SIGINT makes bash, zsh and fish
# abort the calling wrapper function before it cleans up its eval file.
trap 'exit 130' INT

# The snap's instance name (parallel installs: idiot_foo) and its
# refresh-proof directory. Only meaningful inside the snap ($SNAP set).
snap_instance() {
    printf '%s' "${SNAP_INSTANCE_NAME:-${SNAP_NAME:-idiot}}"
}
snap_current() {
    printf '/snap/%s/current' "$(snap_instance)"
}

# Where idiot keeps its state. Confined in the snap: the snap's common
# directory (per-revision $SNAP_USER_DATA would multiply credentials across
# refreshes). Host side of the snap: what `idiot init` exported. Source
# install: XDG.
if [ -n "${SNAP:-}" ]; then
    IDIOT_DATA_HOME="${IDIOT_DATA_HOME:-${SNAP_USER_COMMON}}"
    IDIOT_CACHE_DIR="${IDIOT_CACHE_DIR:-${SNAP_USER_COMMON}/cache}"
else
    IDIOT_DATA_HOME="${IDIOT_DATA_HOME:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot}"
    IDIOT_CACHE_DIR="${IDIOT_CACHE_DIR:-${XDG_CACHE_HOME:-${HOME}/.cache}/idiot}"
fi

# The snap's files run on the host only through a shell whose integration
# came from the snap (it exports IDIOT_SNAP and the state directories).
# Without it this copy would read the wrong state, so refuse.
if [ -z "${SNAP:-}" ] && [ -z "${IDIOT_SNAP:-}" ]; then
    case "${IDIOT_ROOT:-}" in
    /snap/*)
        # shellcheck disable=SC2016
        printf 'error: shell integration is out of date — run: eval "$(idiot init bash)"  or: idiot init fish | source\n' >&2
        exit 1
        ;;
    esac
fi

# Bundled data directory (examples, kernels, etc.)
IDIOT_DATA_DIR="${IDIOT_DATA_DIR:-${IDIOT_ROOT}/share}"

# Use colors only when stderr is a TTY and NO_COLOR is unset
# shellcheck disable=SC2034
if { [ -t 2 ] || [ -n "${FORCE_COLOR:-}" ]; } && [ -z "${NO_COLOR:-}" ]; then
    BOLD="$(printf '\033[1m')"
    DIM="$(printf '\033[2m')"
    BLACK="$(printf '\033[30m')"
    RED="$(printf '\033[31m')"
    GREEN="$(printf '\033[32m')"
    YELLOW="$(printf '\033[33m')"
    BLUE="$(printf '\033[34m')"
    MAGENTA="$(printf '\033[35m')"
    CYAN="$(printf '\033[36m')"
    WHITE="$(printf '\033[37m')"
    RESET="$(printf '\033[0m')"
else
    BOLD=''
    DIM=''
    BLACK=''
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    MAGENTA=''
    CYAN=''
    WHITE=''
    RESET=''
fi

# Resolve IDIOT_LOGO from invoke count.
_idiot_count=0
_idiot_count_file="${IDIOT_CACHE_DIR}/invoke-count"
if [ -f "${_idiot_count_file}" ]; then
    _idiot_count="$(cat "${_idiot_count_file}" 2>/dev/null || :)"
    case "${_idiot_count}" in
    '' | *[!0-9]*) _idiot_count=0 ;;
    esac
fi
# shellcheck disable=SC2034
if [ "${_idiot_count}" -ge 10000 ]; then
    IDIOT_LOGO='🌌'
elif [ "${_idiot_count}" -ge 5000 ]; then
    IDIOT_LOGO='🪐'
elif [ "${_idiot_count}" -ge 2500 ]; then
    IDIOT_LOGO='☀️'
elif [ "${_idiot_count}" -ge 1000 ]; then
    IDIOT_LOGO='🌍'
elif [ "${_idiot_count}" -ge 500 ]; then
    IDIOT_LOGO='🌲'
elif [ "${_idiot_count}" -ge 200 ]; then
    IDIOT_LOGO='🌳'
elif [ "${_idiot_count}" -ge 75 ]; then
    IDIOT_LOGO='🌿'
elif [ "${_idiot_count}" -ge 25 ]; then
    IDIOT_LOGO='🌱'
elif [ "${_idiot_count}" -ge 10 ]; then
    IDIOT_LOGO='🥚'
else
    IDIOT_LOGO='💀'
fi
unset _idiot_count _idiot_count_file

# Display output goes to stderr — it bypasses eval in the shell wrapper and
# reaches the terminal directly in all shells. Only env-modifying output
# (export / unset statements) goes to stdout to be eval'd by the wrapper.

# Print a message to the user
say() {
    printf '%s\n' "$*" >&2
}

# Print an error to stderr and exit
die() {
    printf '%s\n' "${RED}idiot: error:${RESET} $*" >&2
    exit 1
}

# Die unless the given command is found in PATH.
# $1: command name, $2: optional install hint appended after " — "
# Make sure a tool can be run by name. On the host side of the snap, a tool
# the host lacks but the snap bundles becomes a function running the
# bundled copy through snap_app (so the user's PATH is never changed).
require() {
    command -v "${1}" >/dev/null 2>&1 && return 0
    if [ -z "${SNAP:-}" ] && [ -n "${IDIOT_SNAP:-}" ] && snap_tool "${1}" >/dev/null; then
        case "${1}" in
        *[!A-Za-z0-9_]*) ;;
        *)
            eval "${1}() { snap_app ${1} \"\$@\"; }"
            return 0
            ;;
        esac
    fi
    die "${1} is required but not found${2:+ — ${2}}"
}

# POSIX-safe single-quote escaping
shell_quote() {
    printf "'"
    printf '%s' "${1}" | sed "s/'/'\\\\''/g"
    printf "'"
}

# Help formatting helpers — produce consistent output across all help scripts.
#
# help_heading "Subcommands:"
# help_heading "Usage:" "idiot env auth <subcommand>"
# help_command "login"           "Activate a credential"
# help_option  "--dry-run"       "Print what would be removed"
# help_env     "IDIOT_AUTH_DIR"  "Credentials directory"

# Print a bold/blue section heading with optional inline text.
help_heading() {
    say "${BOLD}${BLUE}${1}${RESET}${2:+ ${2}}"
}

# Print a subcommand/command row: cyan name padded to 14 chars, then description.
help_command() {
    gap=$((14 - ${#1}))
    [ "${gap}" -lt 2 ] && gap=2
    say "  ${CYAN}${1}${RESET}$(printf '%*s' "${gap}" '')${2}"
}

# Print an option row: bold name padded to 22 chars, then description.
help_option() {
    gap=$((22 - ${#1}))
    [ "${gap}" -lt 2 ] && gap=2
    say "  ${BOLD}${1}${RESET}$(printf '%*s' "${gap}" '')${2}"
}

# Print an environment variable row: bold name padded to 22 chars, then description.
help_env() {
    gap=$((22 - ${#1}))
    [ "${gap}" -lt 2 ] && gap=2
    say "  ${BOLD}${1}${RESET}$(printf '%*s' "${gap}" '')${2}"
}

# Dispatch to a subcommand within a command group directory.
# $1: display name of the parent command (e.g. "env")
# $2: path to the command group directory
# Remaining args are passed through to the subcommand.
dispatch_subcmd() {
    cmd="${1}"
    cmd_file="${2}"
    shift 2
    subcmd="${1:-}"
    [ $# -ge 1 ] && shift
    case "${subcmd}" in
    "" | -h | --help | help)
        if [ -x "${cmd_file}/help" ]; then
            exec "${cmd_file}/help"
        else
            die "usage: idiot ${cmd} <subcommand>"
        fi
        ;;
    *)
        [ -n "${IDIOT_SHELL}" ] || die "shell not initialized — run: eval \"\$(idiot init bash)\"  or: idiot init fish | source"
        subcmd_file="${cmd_file}/${subcmd}"
        if [ -d "${subcmd_file}" ]; then
            subsubcmd="${1:-}"
            [ $# -ge 1 ] && shift
            case "${subsubcmd}" in
            "" | -h | --help | help)
                if [ -x "${subcmd_file}/help" ]; then
                    exec "${subcmd_file}/help"
                else
                    die "usage: idiot ${cmd} ${subcmd} <subcommand>"
                fi
                ;;
            *)
                subsubcmd_file="${subcmd_file}/${subsubcmd}"
                if [ -x "${subsubcmd_file}" ]; then
                    run_cmd "${subsubcmd_file}" 3 "${cmd}" "${subcmd}" "${subsubcmd}" "$@"
                else
                    die "unknown subcommand: ${cmd} ${subcmd} ${subsubcmd}"
                fi
                ;;
            esac
        elif [ -x "${subcmd_file}" ]; then
            run_cmd "${subcmd_file}" 2 "${cmd}" "${subcmd}" "$@"
        else
            die "unknown subcommand: ${cmd} ${subcmd}"
        fi
        ;;
    esac
}

# True if the command file is a host command: one that needs tools, the
# network or the user's files, and so runs outside the snap's confinement.
is_host_cmd() {
    grep -qx '# idiot: host' "${1}"
}

# Run a resolved command file. Under the snap (confined), a host command
# that isn't asked for help is handed to the user's shell instead.
# Usage: run_cmd <file> <n> <word>... — the first <n> words are the command
# path (e.g. inspect image), the rest its arguments.
run_cmd() {
    _rc_file="${1}"
    _rc_n="${2}"
    shift 2
    if [ -n "${SNAP:-}" ] && is_host_cmd "${_rc_file}"; then
        _rc_first=''
        eval "_rc_first=\${$((_rc_n + 1)):-}"
        case "${_rc_first}" in
        -h | --help | help) ;;
        *)
            emit_handoff "$@"
            exit 0
            ;;
        esac
    fi
    shift "${_rc_n}"
    exec "${_rc_file}" "$@"
}

# Queue a command for the shell wrapper to run in the user's shell, outside
# confinement, from the snap's refresh-proof directory. Sets nothing: the
# host side's environment comes from `idiot init`.
emit_handoff() {
    require_wrapper
    _eh_line="$(shell_quote "$(snap_current)/lib/idiot/idiot")"
    [ -z "${FORCE_COLOR:-}" ] || _eh_line="${_eh_line} --color"
    for _eh_word in "$@"; do
        _eh_line="${_eh_line} $(shell_quote "${_eh_word}")"
    done
    printf '%s\n' "${_eh_line}" >>"${IDIOT_EVAL_FILE}"
}

# The snap's root as seen from here: $SNAP when confined, the exported
# refresh-proof IDIOT_SNAP on the host side, nothing for a source install.
snap_root() {
    printf '%s' "${SNAP:-${IDIOT_SNAP:-}}"
}

# The `core` component's files for the snap revision in use. snapd mounts
# components per snap revision, outside $SNAP.
# TODO(fix-snap-packaging 2.3): confirm this path on a real install.
snap_component_dir() {
    if [ -n "${SNAP:-}" ]; then
        printf '/snap/%s/components/%s/core' "$(snap_instance)" "${SNAP_REVISION:-}"
    else
        _scd_rev="$(readlink "${IDIOT_SNAP}" 2>/dev/null || :)"
        printf '/snap/%s/components/%s/core' "$(snap_instance)" "${_scd_rev##*/}"
    fi
}

# Directories holding the snap's bundled tools, one per line (snap first,
# then the core component). Prints nothing for a source install.
snap_tool_dirs() {
    _std_root="$(snap_root)"
    [ -n "${_std_root}" ] || return 0
    _std_comp="$(snap_component_dir)"
    printf '%s\n' "${_std_root}/usr/bin" "${_std_root}/bin" \
        "${_std_comp}/usr/bin" "${_std_comp}/bin" "${_std_comp}/usr/sbin" "${_std_comp}/sbin"
}

# Path of a tool bundled in the snap or its component, or fail.
snap_tool() {
    for _st_dir in $(snap_tool_dirs); do
        if [ -x "${_st_dir}/${1}" ]; then
            printf '%s' "${_st_dir}/${1}"
            return 0
        fi
    done
    return 1
}

# Run a tool bundled in the snap (or its core component). Anything it needs
# set up is set for its own process only, never in the user's shell; the
# binaries are patched to find core24's libraries, so today that's nothing.
# Without a snap, runs the tool from PATH.
snap_app() {
    _sa_cmd="${1}"
    shift
    if [ -n "$(snap_root)" ]; then
        _sa_bin="$(snap_tool "${_sa_cmd}")" ||
            die "${_sa_cmd}: not found in snap (install the core component: snap install idiot+core)"
        "${_sa_bin}" "$@"
    else
        command -v "${_sa_cmd}" >/dev/null 2>&1 || die "${_sa_cmd} is required but not found"
        "${_sa_cmd}" "$@"
    fi
}

# Die unless the shell wrapper handed us a writable eval file. Commands that
# change the environment call this before doing anything else, so an outdated
# wrapper (one that still evals stdout) changes nothing.
require_wrapper() {
    [ -n "${IDIOT_EVAL_FILE:-}" ] && [ -f "${IDIOT_EVAL_FILE}" ] &&
        [ -w "${IDIOT_EVAL_FILE}" ] ||
        die "shell integration is out of date — run: eval \"\$(idiot init bash)\"  or: idiot init fish | source"
}

# Queue an export of $1=$2 for the shell wrapper to evaluate.
emit_export() {
    require_wrapper
    printf 'export %s=%s\n' "${1}" "$(shell_quote "${2}")" >>"${IDIOT_EVAL_FILE}"
}

# Queue a shell-appropriate unset of $1 for the shell wrapper to evaluate.
# Fish uses "set --erase VAR"; bash/zsh use "unset VAR".
emit_unset() {
    require_wrapper
    if [ "${IDIOT_SHELL:-}" = "fish" ]; then
        printf 'set --erase %s\n' "${1}" >>"${IDIOT_EVAL_FILE}"
    else
        printf 'unset %s\n' "${1}" >>"${IDIOT_EVAL_FILE}"
    fi
}

# Run git with its output dimmed (colours on stderr only), returning git's
# own exit status so callers' `|| die` works.
git() {
    printf '%s' "${DIM}" >&2
    command git "$@"
    _git_rc=$?
    printf '%s' "${RESET}" >&2
    return "${_git_rc}"
}

# Source a share/ module by name.
include() {
    # shellcheck disable=SC1090
    . "${IDIOT_ROOT}/share/${1}"
}
