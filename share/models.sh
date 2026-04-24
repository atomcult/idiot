# shellcheck shell=sh
# share/models.sh - model repository helpers
# Source this file; do not execute directly.

# Parent directory for cloned model repos (one subdirectory per named repo)
IDIOT_MODELS_DIR="${IDIOT_MODELS_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/models}"

# User-defined model repo registry (supplements bundled share/models/)
IDIOT_MODEL_REPOS_DIR="${IDIOT_MODEL_REPOS_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/model-repos}"

# Die unless at least one model repo has been cloned.
require_models_repo() {
    if [ -d "${IDIOT_MODELS_DIR}" ]; then
        for _rp in "${IDIOT_MODELS_DIR}"/*/; do
            [ -d "${_rp}.git" ] && return 0
        done
    fi
    die "no model repos found — run 'idiot model update' first"
}

# Resolve a named model repo to its clone URL.
# Checks user registry first, then bundled share/models/.
resolve_model_repo_url() {
    for _rmru_dir in "${IDIOT_MODEL_REPOS_DIR}" "${IDIOT_DATA_DIR}/models"; do
        _rmru_file="${_rmru_dir}/${1}"
        [ -f "${_rmru_file}" ] && {
            head -1 "${_rmru_file}"
            return 0
        }
    done
    die "unknown model repo '${1}'"
}
