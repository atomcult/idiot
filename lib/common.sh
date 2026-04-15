#!/usr/bin/env sh
# lib/common.sh - shared helpers for idiot subcommands
# Source this file; do not execute directly.

# Credentials directory
IDIOT_CREDS_DIR="${IDIOT_CREDS_DIR:-$HOME/.local/share/idiot/creds}"

# Dedicated Snap Store definitions directory
IDIOT_STORES_DIR="${IDIOT_STORES_DIR:-$HOME/.local/share/idiot/stores}"

# Cloned models repository
IDIOT_MODELS_DIR="${IDIOT_MODELS_DIR:-$HOME/.local/share/idiot/models}"

# ── color ─────────────────────────────────────────────────────────────────────
# Use colors only when stderr is a TTY and NO_COLOR is unset
if [ -t 2 ] && [ -z "${NO_COLOR:-}" ]; then
    BOLD="$(printf '\033[1m')"
    DIM="$(printf '\033[2m')"
    RED="$(printf '\033[31m')"
    RESET="$(printf '\033[0m')"
else
    BOLD=''
    DIM=''
    RED=''
    RESET=''
fi

# ── output helpers ─────────────────────────────────────────────────────────────
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

# ── shell quoting ──────────────────────────────────────────────────────────────

# POSIX-safe single-quote escaping
shell_quote() {
    printf "'"
    printf '%s' "$1" | sed "s/'/'\\\\''/g"
    printf "'"
}

# ── help ───────────────────────────────────────────────────────────────────────

idiot_help() {
    cat >&2 <<'EOF'
idiot - shell environment manager

Usage: idiot <command> [args...]

Commands:
  creds     Manage credentials
  store     Manage Dedicated Snap Stores
  arch      Manage target architecture
  models    Manage model assertions
  changes   Browse recent changes
  init      Set up shell integration

Shell setup (run once, add to your shell rc):
  eval "$(idiot init bash)"    # bash
  eval "$(idiot init zsh)"     # zsh
  idiot init fish | source     # fish

Run 'idiot <command> help' for subcommand details.
EOF
}
