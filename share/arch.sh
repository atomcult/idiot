# shellcheck shell=sh
# share/arch.sh - architecture mapping helpers
# Source this file; do not execute directly.

# Architectures the store accepts in UBUNTU_STORE_ARCH, space-separated.
STORE_ARCHES='amd64 arm64 armhf i386 ppc64el riscv64 s390x'

# Succeed if $1 is one of STORE_ARCHES.
is_store_arch() {
    case " ${STORE_ARCHES} " in
    *" ${1} "*) [ -n "${1}" ] ;;
    *) return 1 ;;
    esac
}

# Map uname -m output to the snap architecture name for the current host.
uname_to_snap_arch() {
    case "$(uname -m)" in
    x86_64) printf 'amd64' ;;
    aarch64) printf 'arm64' ;;
    armv7l | armv6l) printf 'armhf' ;;
    i686 | i386) printf 'i386' ;;
    ppc64le) printf 'ppc64el' ;;
    riscv64) printf 'riscv64' ;;
    s390x) printf 's390x' ;;
    *) uname -m ;;
    esac
}

# Map a snap architecture name to the qemu-system-* binary suffix.
snap_arch_to_qemu() {
    case "${1}" in
    amd64) printf 'x86_64' ;;
    arm64) printf 'aarch64' ;;
    armhf) printf 'arm' ;;
    riscv64) printf 'riscv64' ;;
    s390x) printf 's390x' ;;
    ppc64el) printf 'ppc64' ;;
    i386) printf 'i386' ;;
    *) die "unsupported architecture: ${1}" ;;
    esac
}
