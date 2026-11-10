#!/usr/bin/env sh
# share/vm-firmware.sh - the UEFI firmware `idiot vm` boots
# Source this file; do not execute directly.
#
# One table for both users: cmd/vm picks firmware from it, and the snap's
# vm part keeps exactly these files of the firmware packages (snap/local/
# vm-build), so the two can't drift.

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
