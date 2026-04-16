#!/usr/bin/env sh
# lib/common.sh - shared helpers for idiot subcommands
# Source this file; do not execute directly.

# Credentials directory
IDIOT_AUTH_DIR="${IDIOT_AUTH_DIR:-${HOME}/.local/share/idiot/auth}"

# Dedicated Snap Store definitions directory
IDIOT_STORES_DIR="${IDIOT_STORES_DIR:-${HOME}/.local/share/idiot/stores}"

# Cloned models repository
IDIOT_MODELS_DIR="${IDIOT_MODELS_DIR:-${HOME}/.local/share/idiot/models}"

# Cache directory
IDIOT_CACHE_DIR="${IDIOT_CACHE_DIR:-${XDG_CACHE_HOME:-${HOME}/.cache}/idiot}"

# Use colors only when stderr is a TTY and NO_COLOR is unset
# shellcheck disable=SC2034
if [ -t 2 ] && [ -z "${NO_COLOR:-}" ]; then
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

# POSIX-safe single-quote escaping
shell_quote() {
    printf "'"
    printf '%s' "${1}" | sed "s/'/'\\\\''/g"
    printf "'"
}

# Return an fzf --preview command string for files inside a directory.
# $1: name of the exported shell variable holding the directory path.
# The returned string uses {} as the fzf-supplied filename.
file_preview_cmd() {
    if command -v batcat > /dev/null 2>&1; then
        printf 'batcat --style=plain --color=always "$%s/"{}\n' "${1}"
    else
        printf 'cat "$%s/"{}\n' "${1}"
    fi
}
