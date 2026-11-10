# shellcheck shell=sh
# share/vm.sh - named VM state helpers
# Source this file; do not execute directly.
#
# Every VM has a state directory, ${IDIOT_VM_DIR}/<name> (a symlink when
# the VM was created with --state-dir), holding vm.conf (key=value lines,
# never sourced), its disk overlay, UEFI variables, TPM state, serial logs,
# qemu.pid and qmp.sock.

IDIOT_VM_DIR="${IDIOT_VM_DIR:-${IDIOT_DATA_HOME}/vm}"

# Print the VM name for image path $1: its file name without the directory
# and a .img or .raw extension, other characters than [A-Za-z0-9._-] as '-'.
vm_name_from_image() {
    _vnfi="${1##*/}"
    case "${_vnfi}" in
    *.img) _vnfi="${_vnfi%.img}" ;;
    *.raw) _vnfi="${_vnfi%.raw}" ;;
    esac
    printf '%s' "${_vnfi}" | tr -c 'A-Za-z0-9._-' '-'
}

# Die unless $1 is a valid VM name: 1-32 of [A-Za-z0-9._-], not starting
# with '.' or '-'.
vm_validate_name() {
    case "${1}" in
    '' | .* | -* | *[!A-Za-z0-9._-]*)
        die "invalid VM name '${1}': use 1-32 letters, digits, '.', '-' and '_', not starting with '.' or '-'"
        ;;
    esac
    [ "${#1}" -le 32 ] ||
        die "invalid VM name '${1}': use 1-32 letters, digits, '.', '-' and '_', not starting with '.' or '-'"
}

# Print the state directory of VM $1.
vm_dir() {
    printf '%s/%s' "${IDIOT_VM_DIR}" "${1}"
}

# Succeed if VM $1 exists (has a vm.conf).
vm_exists() {
    [ -f "$(vm_dir "${1}")/vm.conf" ]
}

# Print the value of key $2 in VM $1's vm.conf (empty if unset).
vm_conf_get() {
    _vcg="$(vm_dir "${1}")/vm.conf"
    [ -f "${_vcg}" ] || return 0
    awk -v k="${2}=" 'index($0, k) == 1 { print substr($0, length(k) + 1); exit }' "${_vcg}"
}

# Set key $2 to value $3 (one line) in VM $1's vm.conf.
vm_conf_set() {
    case "${3}" in
    *'
'*) die "vm.conf values are one line: ${2}" ;;
    esac
    _vcs="$(vm_dir "${1}")/vm.conf"
    {
        [ ! -f "${_vcs}" ] ||
            awk -v k="${2}=" 'index($0, k) != 1' "${_vcs}"
        printf '%s=%s\n' "${2}" "${3}"
    } >"${_vcs}.new"
    mv "${_vcs}.new" "${_vcs}"
}

# Print the pid of VM $1's qemu and succeed when it is running: its pid
# file names a live process whose command line is a qemu-system-* (so a
# recycled pid isn't taken for the VM).
vm_pid() {
    _vp_file="$(vm_dir "${1}")/qemu.pid"
    [ -f "${_vp_file}" ] || return 1
    _vp="$(tr -dc '0-9' <"${_vp_file}")"
    [ -n "${_vp}" ] && kill -0 "${_vp}" 2>/dev/null || return 1
    tr '\0' '\n' <"/proc/${_vp}/cmdline" 2>/dev/null | head -n 1 |
        grep -q 'qemu-system-' || return 1
    printf '%s' "${_vp}"
}

# Succeed if VM $1 is running.
vm_running() {
    vm_pid "${1}" >/dev/null
}

# Print the names of all VMs, one per line.
vm_names() {
    [ -d "${IDIOT_VM_DIR}" ] || return 0
    for _vn in "${IDIOT_VM_DIR}"/*; do
        [ -f "${_vn}/vm.conf" ] && printf '%s\n' "${_vn##*/}"
    done
}

# Print the names of the running VMs, one per line.
vm_running_names() {
    vm_names | while IFS= read -r _vrn; do
        vm_running "${_vrn}" && printf '%s\n' "${_vrn}"
    done
}

# Print the VM a command acts on: $1 when given (dying unless it exists),
# else the only running VM; with several, a picker when a terminal is
# attached and fzf is found, else die listing them. With $2 = any and no
# VM running, the only VM. Use as `name="$(vm_resolve ...)" || exit 1`.
vm_resolve() {
    if [ -n "${1:-}" ]; then
        vm_validate_name "${1}"
        vm_exists "${1}" || die "no VM named '${1}' — see: idiot vm list"
        printf '%s' "${1}"
        return 0
    fi
    _vr="$(vm_running_names)"
    if [ -z "${_vr}" ] && [ "${2:-}" = any ]; then
        _vr="$(vm_names)"
        [ -n "${_vr}" ] || die "no VMs — boot one with: idiot vm <image>"
        [ "$(printf '%s\n' "${_vr}" | wc -l)" -eq 1 ] ||
            die "no VM is running and there are several ($(printf '%s' "${_vr}" | paste -sd, - | sed 's/,/, /g')) — name one"
    fi
    [ -n "${_vr}" ] || die "no VM is running — boot one with: idiot vm <image>"
    if [ "$(printf '%s\n' "${_vr}" | wc -l)" -eq 1 ]; then
        printf '%s' "${_vr}"
    elif [ -t 0 ] && [ -t 2 ] && command -v fzf >/dev/null 2>&1; then
        printf '%s\n' "${_vr}" | fzf --prompt='vm> ' || exit 1
    else
        die "several VMs are running ($(printf '%s' "${_vr}" | paste -sd, - | sed 's/,/, /g')) — pick one with --name <name>"
    fi
}

# Succeed if host port $1 is free for VM $2: no other running VM records
# it, and (when python3 is there) binding it works, as qemu's hostfwd will.
vm_port_free() {
    for _vpf in $(vm_running_names); do
        [ "${_vpf}" != "${2}" ] || continue
        [ "$(vm_conf_get "${_vpf}" ssh_port)" != "${1}" ] || return 1
    done
    command -v python3 >/dev/null 2>&1 || return 0
    python3 -c '
import socket, sys
s = socket.socket()
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
try:
    s.bind(("", int(sys.argv[1])))
except OSError:
    sys.exit(1)
' "${1}"
}

# Print the ssh port VM $1 boots with: $2 when given (dying when it isn't
# free), else its recorded port if free, else 8022, else the first free
# port in 8023-8122.
vm_pick_ssh_port() {
    if [ -n "${2:-}" ]; then
        vm_port_free "${2}" "${1}" || die "port ${2} is in use — pick another with --ssh-port"
        printf '%s' "${2}"
        return 0
    fi
    _vpsp="$(vm_conf_get "${1}" ssh_port)"
    if [ -n "${_vpsp}" ] && vm_port_free "${_vpsp}" "${1}"; then
        printf '%s' "${_vpsp}"
        return 0
    fi
    _vpsp=8022
    while [ "${_vpsp}" -le 8122 ]; do
        if vm_port_free "${_vpsp}" "${1}"; then
            printf '%s' "${_vpsp}"
            return 0
        fi
        _vpsp=$((_vpsp + 1))
    done
    die "no free ssh port in 8022-8122 — pick one with --ssh-port"
}

# Stop the swtpm serving state directory $1, if it is still running, and
# forget its pid file. Ubuntu's AppArmor profile for swtpm refuses signals
# from the user's processes, so when kill doesn't work this sends the TPM
# control channel's shutdown command (CMD_SHUTDOWN) over swtpm.sock.
vm_swtpm_stop() {
    _vss_pidf="${1}/swtpm.pid"
    [ -f "${_vss_pidf}" ] || return 0
    _vss="$(tr -dc '0-9' <"${_vss_pidf}")"
    if [ -n "${_vss}" ] && kill -0 "${_vss}" 2>/dev/null; then
        kill "${_vss}" 2>/dev/null || :
        sleep 0.2
        if kill -0 "${_vss}" 2>/dev/null && command -v python3 >/dev/null 2>&1; then
            python3 -c '
import socket, struct, sys
s = socket.socket(socket.AF_UNIX)
s.settimeout(5)
s.connect(sys.argv[1])
s.sendall(struct.pack(">I", 3))  # CMD_SHUTDOWN
s.recv(4)
' "${1}/swtpm.sock" 2>/dev/null || :
        fi
    fi
    rm -f "${_vss_pidf}"
}
