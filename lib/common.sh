#!/usr/bin/env sh
# lib/common.sh - shared helpers for idiot subcommands
# Source this file; do not execute directly.

# Credentials directory
IDIOT_AUTH_DIR="${IDIOT_AUTH_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/auth}"

# Dedicated Snap Store definitions directory
IDIOT_STORES_DIR="${IDIOT_STORES_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/stores}"

# Cloned models repository
IDIOT_MODELS_DIR="${IDIOT_MODELS_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/models}"

# Cache directory
IDIOT_CACHE_DIR="${IDIOT_CACHE_DIR:-${XDG_CACHE_HOME:-${HOME}/.cache}/idiot}"

# Bundled data directory (examples, kernels, etc.)
IDIOT_DATA_DIR="${IDIOT_DATA_DIR:-${IDIOT_ROOT}/lib}"

# Launchpad username file
IDIOT_LP_USER_FILE="${IDIOT_LP_USER_FILE:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/lp-user}"

# User-defined example repo aliases (supplements bundled lib/examples/)
IDIOT_EXAMPLES_DIR="${IDIOT_EXAMPLES_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/examples}"

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
require() {
    command -v "${1}" >/dev/null 2>&1 ||
        die "${1} is required but not found${2:+ — ${2}}"
}

# Die unless a credential named $1 exists.
require_cred() {
    [ -f "${IDIOT_AUTH_DIR}/${1}" ] || die "credentials '${1}' not found"
}

# Die if a credential named $1 already exists.
require_no_cred() {
    [ -f "${IDIOT_AUTH_DIR}/${1}" ] && die "credentials '${1}' already exist — remove it first"
}

# Die unless a store named $1 exists.
require_store() {
    [ -f "${IDIOT_STORES_DIR}/${1}" ] || die "store '${1}' not found"
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
                    exec "${subsubcmd_file}" "$@"
                else
                    die "unknown subcommand: ${cmd} ${subcmd} ${subsubcmd}"
                fi
                ;;
            esac
        elif [ -x "${subcmd_file}" ]; then
            exec "${subcmd_file}" "$@"
        else
            die "unknown subcommand: ${cmd} ${subcmd}"
        fi
        ;;
    esac
}

# Run a binary that may be bundled inside the snap.
# When $SNAP is set, resolves to $SNAP/usr/bin/<cmd> or $SNAP/bin/<cmd>.
# Falls back to the system PATH when not running inside a snap.
snap_app() {
    _sa_cmd="${1}"
    shift
    if [ -n "${SNAP:-}" ]; then
        if [ -x "${SNAP}/usr/bin/${_sa_cmd}" ]; then
            "${SNAP}/usr/bin/${_sa_cmd}" "$@"
        elif [ -x "${SNAP}/bin/${_sa_cmd}" ]; then
            "${SNAP}/bin/${_sa_cmd}" "$@"
        else
            die "${_sa_cmd}: not found in snap (install the core component: snap install idiot+core)"
        fi
    else
        command -v "${_sa_cmd}" >/dev/null 2>&1 || die "${_sa_cmd} is required but not found"
        "${_sa_cmd}" "$@"
    fi
}

# Return an fzf --preview command string for files inside a directory.
# $1: name of the exported shell variable holding the directory path.
# The returned string uses {} as the fzf-supplied filename.
file_preview_cmd() {
    if command -v batcat >/dev/null 2>&1; then
        printf 'batcat --style=plain --color=always "$%s/"{}\n' "${1}"
    else
        printf 'cat "$%s/"{}\n' "${1}"
    fi
}

# Emit a shell-appropriate unset statement for a variable.
# Fish uses "set -e VAR"; bash/zsh use "unset VAR".
emit_unset() {
    if [ "${IDIOT_SHELL:-}" = "fish" ]; then
        printf 'set --erase %s\n' "${1}"
    else
        printf 'unset %s\n' "${1}"
    fi
}

git() {
    printf '%s' "${DIM}"
    command git "$@"
    printf '%s' "${RESET}"
}
