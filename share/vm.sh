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
        [ -n "${_vr}" ] || die "no VMs — boot one with: idiot vm boot <image>"
        [ "$(printf '%s\n' "${_vr}" | wc -l)" -eq 1 ] ||
            die "no VM is running and there are several ($(printf '%s' "${_vr}" | paste -sd, - | sed 's/,/, /g')) — name one"
    fi
    [ -n "${_vr}" ] || die "no VM is running — boot one with: idiot vm boot <image>"
    if [ "$(printf '%s\n' "${_vr}" | wc -l)" -eq 1 ]; then
        printf '%s' "${_vr}"
    elif [ -t 0 ] && [ -t 2 ] && command -v fzf >/dev/null 2>&1; then
        printf '%s\n' "${_vr}" | fzf --prompt='vm> ' || exit 1
    else
        die "several VMs are running ($(printf '%s' "${_vr}" | paste -sd, - | sed 's/,/, /g')) — pick one with --name <name>"
    fi
}

# Succeed if host port $1 is free for VM $2: no other running VM records
# it, and (when idiot-helper is there) binding it works, as qemu's hostfwd
# will.
vm_port_free() {
    for _vpf in $(vm_running_names); do
        [ "${_vpf}" != "${2}" ] || continue
        [ "$(vm_conf_get "${_vpf}" ssh_port)" != "${1}" ] || return 1
    done
    _vpf_h="$(helper_path idiot-helper)" || return 0
    "${_vpf_h}" port-free "${1}"
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
        if kill -0 "${_vss}" 2>/dev/null &&
            _vss_h="$(helper_path idiot-helper)"; then
            "${_vss_h}" swtpm-shutdown "${1}/swtpm.sock" 2>/dev/null || :
        fi
    fi
    rm -f "${_vss_pidf}"
}

# ssh options for every connection to a VM. Host keys aren't checked: a
# VM's host key changes on every reset. VM_SSH_EXTRA adds options (no
# spaces inside one), e.g. '-o BatchMode=yes' or '-t'.
VM_SSH_OPTS='-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR'

# Run ssh to the VM on host port $1 as user $2, passing the remaining
# arguments to ssh as they are (as `idiot vm ssh` does).
vm_ssh_raw() {
    _vsr_port="${1}"
    _vsr_user="${2}"
    shift 2
    # option lists are words split on spaces, whatever the caller's IFS
    _vsr_ifs="${IFS}"
    IFS=' '
    # shellcheck disable=SC2086
    set -- -p "${_vsr_port}" -l "${_vsr_user}" ${VM_SSH_OPTS} ${VM_SSH_EXTRA:-} localhost "$@"
    IFS="${_vsr_ifs}"
    ssh "$@"
}

# Run a command in the VM on host port $1 as user $2: each remaining
# argument reaches the guest as one word, quoted for its shell.
vm_ssh() {
    _vs_port="${1}"
    _vs_user="${2}"
    shift 2
    _vs_cmd=''
    for _vs_word in "$@"; do
        _vs_cmd="${_vs_cmd:+${_vs_cmd} }$(shell_quote "${_vs_word}")"
    done
    vm_ssh_raw "${_vs_port}" "${_vs_user}" "${_vs_cmd}"
}

# Run scp with the VM on host port $1 as user $2 (the guest is localhost:).
vm_scp() {
    _vsc_port="${1}"
    _vsc_user="${2}"
    shift 2
    _vsc_ifs="${IFS}"
    IFS=' '
    # shellcheck disable=SC2086 # option lists are words split on spaces
    set -- -P "${_vsc_port}" ${VM_SSH_OPTS} ${VM_SSH_EXTRA:-} -o "User=${_vsc_user}" "$@"
    IFS="${_vsc_ifs}"
    scp "$@"
}

# Snapshots: an internal qcow2 snapshot of disk.qcow2 tagged with the
# snapshot's name, and ${state}/snapshots/<name>/ holding copies of
# fw_vars.fd and tpm2-* and snapshot.conf (created=<unix time>, vars=0|1,
# tpm=0|1; key=value, never sourced). snapshots/.restoring names the
# snapshot a restore was interrupted in.

# Print VM $1's snapshots directory, or that of its snapshot $2.
vm_snapshot_dir() {
    printf '%s/snapshots%s' "$(vm_dir "${1}")" "${2:+/${2}}"
}

# Die unless $1 is a valid snapshot name: as a VM name, and not digits
# only (qemu-img takes an argument that is a snapshot ID as that ID).
vm_validate_snapshot_name() {
    case "${1}" in
    '' | .* | -* | *[!A-Za-z0-9._-]*) ;;
    *[!0-9]*) [ "${#1}" -gt 32 ] || return 0 ;;
    esac
    die "invalid snapshot name '${1}': use 1-32 letters, digits, '.', '-' and '_', not starting with '.' or '-' and not digits only"
}

# Print the value of key $3 in snapshot $2 of VM $1 (empty if unset).
vm_snapshot_conf_get() {
    _vscg="$(vm_snapshot_dir "${1}" "${2}")/snapshot.conf"
    [ -f "${_vscg}" ] || return 0
    awk -v k="${3}=" 'index($0, k) == 1 { print substr($0, length(k) + 1); exit }' "${_vscg}"
}

# Print the names of VM $1's snapshots (its snapshots/ directories), one
# per line, oldest first.
vm_snapshot_names() {
    _vsnd="$(vm_snapshot_dir "${1}")"
    [ -d "${_vsnd}" ] || return 0
    for _vsn in "${_vsnd}"/*; do
        [ -d "${_vsn}" ] || continue
        _vsn="${_vsn##*/}"
        printf '%s %s\n' "$(vm_snapshot_conf_get "${1}" "${_vsn}" created | tr -dc '0-9')" "${_vsn}"
    done | sort -n -k 1,1 | awk '{ print $2 }'
}

# Print the internal snapshot tags of VM $1's disk overlay, one per line
# (none without an overlay). Works while qemu has the overlay open (-U).
vm_snapshot_tags() {
    _vst="$(vm_dir "${1}")/disk.qcow2"
    [ -f "${_vst}" ] || return 0
    tool qemu-img snapshot -l -U "${_vst}" | awk 'NR > 2 { print $2 }'
}

# Print what is missing from snapshot $2 of VM $1, one part per line;
# nothing when it is whole. $3: the overlay's tags (vm_snapshot_tags),
# when the caller has them.
vm_snapshot_problem() {
    _vsp_d="$(vm_snapshot_dir "${1}" "${2}")"
    _vsp_tags="${3-$(vm_snapshot_tags "${1}")}"
    printf '%s\n' "${_vsp_tags}" | grep -qxF -- "${2}" ||
        printf '%s\n' "the disk snapshot in disk.qcow2"
    if [ ! -f "${_vsp_d}/snapshot.conf" ]; then
        printf '%s\n' "snapshot.conf"
        return 0
    fi
    [ "$(vm_snapshot_conf_get "${1}" "${2}" vars)" != 1 ] || [ -f "${_vsp_d}/fw_vars.fd" ] ||
        printf '%s\n' "the copy of fw_vars.fd"
    [ "$(vm_snapshot_conf_get "${1}" "${2}" tpm)" != 1 ] || vm_snapshot_has_tpm "${_vsp_d}" ||
        printf '%s\n' "the copy of the TPM state"
}

# Succeed if directory $1 holds TPM state (tpm2-* files).
vm_snapshot_has_tpm() {
    for _vsht in "${1}"/tpm2-*; do
        [ -f "${_vsht}" ] && return 0
    done
    return 1
}
