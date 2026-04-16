#!/usr/bin/env sh
# lib/common.sh - shared helpers for idiot subcommands
# Source this file; do not execute directly.

# Credentials directory
IDIOT_CREDS_DIR="${IDIOT_CREDS_DIR:-${HOME}/.local/share/idiot/creds}"

# Dedicated Snap Store definitions directory
IDIOT_STORES_DIR="${IDIOT_STORES_DIR:-${HOME}/.local/share/idiot/stores}"

# Cloned models repository
IDIOT_MODELS_DIR="${IDIOT_MODELS_DIR:-${HOME}/.local/share/idiot/models}"

# Cache directory
IDIOT_CACHE_DIR="${IDIOT_CACHE_DIR:-${XDG_CACHE_HOME:-${HOME}/.cache}/idiot}"

# Use colors only when stderr is a TTY and NO_COLOR is unset
if [ -t 2 ] && [ -z "${NO_COLOR:-}" ]; then
    BOLD="$(printf '\033[1m')"
    DIM="$(printf '\033[2m')"
    RED="$(printf '\033[31m')"
    CYAN="$(printf '\033[36m')"
    RESET="$(printf '\033[0m')"
else
    BOLD=''
    DIM=''
    RED=''
    CYAN=''
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

idiot_help() {
    say "${BOLD}idiot${RESET} - shell environment manager"
    echo >&2
    say "${BOLD}${CYAN}Usage:${RESET} idiot <command> [args...]"
    echo >&2
    say "${BOLD}${CYAN}Commands:${RESET}"
    say "  ${BOLD}creds${RESET}     Manage credentials"
    say "  ${BOLD}store${RESET}     Manage Dedicated Snap Stores"
    say "  ${BOLD}arch${RESET}      Manage target architecture"
    say "  ${BOLD}models${RESET}    Manage model assertions"
    say "  ${BOLD}changes${RESET}   Browse recent changes"
    say "  ${BOLD}init${RESET}      Set up shell integration"
    echo >&2
    say "${BOLD}${CYAN}Shell setup${RESET} (run once, add to your shell rc):"
    say '  eval "$(idiot init bash)"    # bash'
    say '  eval "$(idiot init zsh)"     # zsh'
    say '  idiot init fish | source     # fish'
    echo >&2
    say "Run ${DIM}idiot <command> help${RESET} for subcommand details."
}
