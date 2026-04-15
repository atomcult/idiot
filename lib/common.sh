#!/usr/bin/env sh
# lib/common.sh - shared helpers for idiot subcommands
# Source this file; do not execute directly.

# State directory for tracking managed vars
IDIOT_STATE_DIR="${IDIOT_STATE_DIR:-$HOME/.local/state/idiot}"
IDIOT_MANAGED_FILE="$IDIOT_STATE_DIR/managed"

# Credentials directory
IDIOT_CREDS_DIR="${IDIOT_CREDS_DIR:-$HOME/.local/share/idiot/creds}"

# Dedicated Snap Store definitions directory
IDIOT_STORES_DIR="${IDIOT_STORES_DIR:-$HOME/.local/share/idiot/stores}"

# Cloned models repository
IDIOT_MODELS_DIR="${IDIOT_MODELS_DIR:-$HOME/.local/share/idiot/models}"

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
    printf 'idiot: error: %s\n' "$*" >&2
    exit 1
}

# Warn without exiting
warn() {
    printf 'idiot: warn: %s\n' "$*" >&2
}

# ── shell quoting ──────────────────────────────────────────────────────────────

# POSIX-safe single-quote escaping
shell_quote() {
    printf "'"
    printf '%s' "$1" | sed "s/'/'\\\\''/g"
    printf "'"
}

# ── state management ───────────────────────────────────────────────────────────

state_init() {
    mkdir -p "$IDIOT_STATE_DIR"
    touch "$IDIOT_MANAGED_FILE"
}

# Record a variable name as managed by idiot
state_track() {
    state_init
    var="$1"
    # Add only if not already present
    grep -qxF "$var" "$IDIOT_MANAGED_FILE" 2>/dev/null || printf '%s\n' "$var" >> "$IDIOT_MANAGED_FILE"
}

# Remove a variable from managed tracking
state_untrack() {
    state_init
    var="$1"
    tmp="$(mktemp)"
    grep -vxF "$var" "$IDIOT_MANAGED_FILE" > "$tmp" 2>/dev/null || true
    mv "$tmp" "$IDIOT_MANAGED_FILE"
}

# List all tracked variable names
state_list() {
    state_init
    cat "$IDIOT_MANAGED_FILE"
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
