# shellcheck shell=sh
# share/fzf.sh - fzf helpers for interactive selection
# Source this file; do not execute directly.

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

# Interactively pick a filename from a directory using fzf.
# $1: directory path
# $2: fzf prompt string (e.g. "cred> ")
# $3: error message when the directory is empty
# $4: (optional) custom --preview command; defaults to file_preview_cmd
#
# Callers supplying a custom preview that references an env variable must
# export that variable before calling this function.
#
# Prints the selected filename (no directory prefix) to stdout.
# Returns fzf's exit code — caller should handle cancellation with || exit 0.
fzf_pick_from_dir() {
    _fpfd_dir="${1}"
    _fpfd_prompt="${2}"
    _fpfd_empty="${3}"
    require fzf "install fzf for interactive selection"
    mkdir -p "${_fpfd_dir}"
    _fpfd_files=$(find "${_fpfd_dir}" -maxdepth 1 -type f ! -name '.*' |
        sort | sed "s|${_fpfd_dir}/||")
    [ -n "${_fpfd_files}" ] || die "${_fpfd_empty}"
    if [ -n "${4:-}" ]; then
        _fpfd_preview="${4}"
    else
        export _fpfd_dir
        _fpfd_preview="$(file_preview_cmd _fpfd_dir)"
    fi
    printf '%s\n' "${_fpfd_files}" | fzf --prompt="${_fpfd_prompt}" --preview="${_fpfd_preview}"
}
