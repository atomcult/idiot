# shellcheck shell=sh
# share/models.sh - model repository helpers
# Source this file; do not execute directly.

IDIOT_MODELS_DIR="${IDIOT_MODELS_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/models}"

# Die unless the models repository has been cloned.
require_models_repo() {
    [ -d "${IDIOT_MODELS_DIR}/.git" ] || die "models repository not found — run 'idiot model update' first"
}
