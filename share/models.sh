# shellcheck shell=sh
# share/models.sh - model repository helpers
# Source this file; do not execute directly.

# Parent directory for cloned model repos (one subdirectory per named repo)
IDIOT_MODELS_DIR="${IDIOT_MODELS_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/models}"

# User-defined model repo registry (supplements bundled share/models/)
IDIOT_MODEL_REPOS_DIR="${IDIOT_MODEL_REPOS_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/model-repos}"

# Print one section of the model repo list (built-in or user).
# $1: registry directory to scan
model_list_section() {
    _mls_dir="${1}"
    _mls_max=0
    if [ -d "${_mls_dir}" ]; then
        for _mls_f in "${_mls_dir}"/*; do
            [ -f "${_mls_f}" ] || continue
            _mls_n="$(basename "${_mls_f}")"
            [ "${#_mls_n}" -gt "${_mls_max}" ] && _mls_max="${#_mls_n}"
        done
    fi
    _mls_found=0
    if [ -d "${_mls_dir}" ]; then
        for _mls_f in "${_mls_dir}"/*; do
            [ -f "${_mls_f}" ] || continue
            _mls_found=1
            _mls_name="$(basename "${_mls_f}")"
            _mls_url="$(head -1 "${_mls_f}")"
            _mls_clone="${IDIOT_MODELS_DIR}/${_mls_name}"
            if [ -d "${_mls_clone}/.git" ]; then
                _mls_marker="${GREEN}✓${RESET} "
            else
                _mls_marker="  "
            fi
            _mls_gap=$((_mls_max - ${#_mls_name} + 2))
            say "  ${CYAN}${_mls_name}${RESET}$(printf '%*s' "${_mls_gap}" '')${_mls_marker}${DIM}${_mls_url}${RESET}"
        done
    fi
    [ "${_mls_found}" -eq 1 ] || say "  ${DIM}(none)${RESET}"
}

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
