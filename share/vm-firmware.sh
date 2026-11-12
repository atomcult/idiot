#!/usr/bin/env sh
# share/vm-firmware.sh - the UEFI firmware `idiot vm` boots
# Source this file; do not execute directly.
#
# One table for both users: cmd/vm picks firmware from it, and the snap's
# vm part keeps exactly these files of the firmware packages (snap/local/
# vm-build), so the two can't drift. vm_find_qemu and vm_find_firmware are
# the lookups `idiot vm` boots with, shared with `idiot doctor`; they need
# common.sh and arch.sh, and never die.

# Candidate paths for arch $1, secure boot $2 (1|0), $3 code|vars, one per
# line, best first. Plain UEFI images must be 64 MiB pflash images:
# qemu-efi-aarch64's 2 MiB QEMU_EFI.fd can't be pflash on the virt machine.
# armhf and riscv64 have no secure-boot variant (Ubuntu ships none): both
# modes boot the same images (vm_firmware_secboot says so).
vm_firmware() {
    case "${1}:${2}:${3}" in
    amd64:1:code) printf '%s\n' /usr/share/OVMF/OVMF_CODE_4M.secboot.fd /usr/share/OVMF/OVMF_CODE.secboot.fd ;;
    amd64:1:vars) printf '%s\n' /usr/share/OVMF/OVMF_VARS_4M.ms.fd /usr/share/OVMF/OVMF_VARS.ms.fd ;;
    amd64:0:code) printf '%s\n' /usr/share/OVMF/OVMF_CODE_4M.fd ;;
    amd64:0:vars) printf '%s\n' /usr/share/OVMF/OVMF_VARS_4M.fd ;;
    arm64:1:code) printf '%s\n' /usr/share/AAVMF/AAVMF_CODE.secboot.fd /usr/share/qemu-efi-aarch64/QEMU_EFI.secboot.fd ;;
    arm64:1:vars) printf '%s\n' /usr/share/AAVMF/AAVMF_VARS.ms.fd /usr/share/qemu-efi-aarch64/QEMU_VARS.ms.fd ;;
    arm64:0:code) printf '%s\n' /usr/share/AAVMF/AAVMF_CODE.no-secboot.fd /usr/share/AAVMF/AAVMF_CODE.fd ;;
    arm64:0:vars) printf '%s\n' /usr/share/AAVMF/AAVMF_VARS.fd ;;
    armhf:*:code) printf '%s\n' /usr/share/AAVMF/AAVMF32_CODE.fd ;;
    armhf:*:vars) printf '%s\n' /usr/share/AAVMF/AAVMF32_VARS.fd ;;
    riscv64:*:code) printf '%s\n' /usr/share/qemu-efi-riscv64/RISCV_VIRT_CODE.fd ;;
    riscv64:*:vars) printf '%s\n' /usr/share/qemu-efi-riscv64/RISCV_VIRT_VARS.fd ;;
    *) return 1 ;;
    esac
}

# Whether arch $1 has secure-boot firmware at all.
vm_firmware_secboot() {
    case "${1}" in
    armhf | riscv64) return 1 ;;
    *) return 0 ;;
    esac
}

# The package that provides arch $1's firmware.
vm_firmware_package() {
    case "${1}" in
    amd64) printf 'ovmf' ;;
    arm64) printf 'qemu-efi-aarch64' ;;
    armhf) printf 'qemu-efi-arm' ;;
    riscv64) printf 'qemu-efi-riscv64' ;;
    *) return 1 ;;
    esac
}

# Every path in the table, one per line (for the snap build).
vm_firmware_all() {
    for _vfa_arch in amd64 arm64 armhf riscv64; do
        for _vfa_sb in 1 0; do
            vm_firmware "${_vfa_arch}" "${_vfa_sb}" code
            vm_firmware "${_vfa_arch}" "${_vfa_sb}" vars
        done
    done | sort -u
}

# The package providing qemu-system-$1 (a qemu arch name).
vm_qemu_package() {
    case "${1}" in
    x86_64 | i386) printf 'qemu-system-x86' ;;
    aarch64 | arm) printf 'qemu-system-arm' ;;
    riscv64) printf 'qemu-system-misc' ;;
    *) printf 'qemu-system' ;;
    esac
}

# Finds qemu for arch $1 (the host's first, see tool_path). Sets qemu_path;
# vm_comp (the installed vm component, or empty); qemu_comp (that component
# when it's the one running, else empty); and qemu_env and
# qemu_data_flags, which point the component's qemu at its own modules and
# ROMs (it was built for /usr, which on the host is another qemu or none).
# Fails, with qemu_path empty, when there's no qemu for $1.
# shellcheck disable=SC2034 # the caller uses what this sets
vm_find_qemu() {
    vm_comp="$(snap_component_dirs | sed -n '/\/vm$/p')"
    qemu_comp=''
    qemu_env=''
    qemu_data_flags=''
    qemu_path="$(tool_path "qemu-system-$(snap_arch_to_qemu "${1}")")" || {
        qemu_path=''
        return 1
    }
    [ -n "${vm_comp}" ] || return 0
    case "${qemu_path}" in
    "${vm_comp}"/*) ;;
    *) return 0 ;;
    esac
    qemu_comp="${vm_comp}"
    # Ubuntu's qemu searches these for ROMs (its --firmwarepath)
    for _vfq_dir in usr/share/qemu usr/share/seabios usr/lib/ipxe/qemu; do
        [ ! -d "${qemu_comp}/${_vfq_dir}" ] ||
            qemu_data_flags="${qemu_data_flags} -L ${qemu_comp}/${_vfq_dir}"
    done
    # the modules dir (usr/lib/<triplet>/qemu), not usr/lib/ipxe/qemu's ROMs
    for _vfq_mod in "${qemu_comp}"/usr/lib/*/qemu/accel-*.so; do
        [ -f "${_vfq_mod}" ] || continue
        qemu_env="env QEMU_MODULE_DIR=${_vfq_mod%/*}"
        break
    done
}

# The first firmware file for arch $1, secure boot $2, $3 code|vars, for
# the qemu vm_find_qemu found: the host's qemu boots the host's firmware,
# or the vm component's when the host has none; the component's qemu boots
# only the component's. Fails, printing nothing, when there's none.
vm_find_firmware() {
    if [ -z "${qemu_comp:-}" ]; then
        _vff_roots="/ ${vm_comp:-}"
    else
        _vff_roots="${qemu_comp}"
    fi
    for _vff_root in ${_vff_roots}; do
        for _vff_path in $(vm_firmware "${1}" "${2}" "${3}"); do
            _vff_file="${_vff_root%/}${_vff_path}"
            if [ -f "${_vff_file}" ]; then
                printf '%s' "${_vff_file}"
                return 0
            fi
        done
    done
    return 1
}
