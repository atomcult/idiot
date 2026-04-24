# shellcheck shell=sh
# share/examples.sh - example repository helpers
# Source this file; do not execute directly.

# User-defined example repo aliases (supplements bundled share/examples/)
IDIOT_EXAMPLES_DIR="${IDIOT_EXAMPLES_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/examples}"

# Resolve an example argument to a clone URL.
# Accepts: full URL, <user>/<repo> (GitHub shorthand), or a saved alias name.
resolve_example_url() {
    case "${1}" in
    *://*)
        printf '%s\n' "${1}"
        ;;
    */*)
        printf 'https://github.com/%s\n' "${1}"
        ;;
    *)
        for _reu_dir in "${IDIOT_EXAMPLES_DIR}" "${IDIOT_DATA_DIR}/examples"; do
            _reu_file="${_reu_dir}/${1}"
            [ -f "${_reu_file}" ] && {
                head -1 "${_reu_file}"
                return 0
            }
        done
        die "unknown example '${1}' — use <user>/<repo>, a full URI, or run 'idiot example list'"
        ;;
    esac
}
