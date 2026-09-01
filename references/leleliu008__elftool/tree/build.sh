#!/bin/sh

set -e

unset IFS

COLOR_RED='\033[0;31m'
COLOR_GREEN='\033[0;32m'
COLOR_PURPLE='\033[0;35m'
COLOR_OFF='\033[0m'

echo() {
    printf '%b\n' "$*"
}

abort() {
    EXIT_STATUS_CODE="$1"
    shift
    printf '%b\n' "${COLOR_RED}💔  $*${COLOR_OFF}" >&2
    exit "$EXIT_STATUS_CODE"
}

run() {
    echo "${COLOR_PURPLE}==>${COLOR_OFF} ${COLOR_GREEN}$@${COLOR_OFF}"
    eval "$@"
}

###################################################

__install_gcc_via_syspm_on_debian() {
    run $sudo apt-get -y update
    run $sudo apt-get -y install gcc
}

__install_gcc_via_syspm_on_ubuntu() {
    run $sudo apt-get -y update
    run $sudo apt-get -y install gcc
}

__install_gcc_via_syspm_on_linuxmint() {
    run $sudo apt-get -y update
    run $sudo apt-get -y install gcc
}

__install_gcc_via_syspm_on_openEuler() {
    run $sudo dnf -y update
    run $sudo dnf -y install gcc
}

__install_gcc_via_syspm_on_rocky() {
    run $sudo dnf -y update
    run $sudo dnf -y install gcc
}

__install_gcc_via_syspm_on_almalinux() {
    run $sudo dnf -y update
    run $sudo dnf -y install gcc
}

__install_gcc_via_syspm_on_centos() {
    run $sudo dnf -y update
    run $sudo dnf -y install gcc
}

__install_gcc_via_syspm_on_fedora() {
    run $sudo dnf -y update
    run $sudo dnf -y install gcc
}

__install_gcc_via_syspm_on_rhel() {
    run $sudo dnf -y update
    run $sudo dnf -y install gcc
}

__install_gcc_via_syspm_on_opensuse() {
    run $sudo zypper update  -y
    run $sudo zypper install -y gcc
}

__install_gcc_via_syspm_on_gentoo() {
    :
}

__install_gcc_via_syspm_on_manjaro() {
    run $sudo pacman -Syyuu --noconfirm
    run $sudo pacman -S     --noconfirm gcc
}

__install_gcc_via_syspm_on_arch() {
    run $sudo pacman -Syyuu --noconfirm
    run $sudo pacman -S     --noconfirm gcc
}

__install_gcc_via_syspm_on_void() {
    run $sudo xbps-install -Syu xbps
    run $sudo xbps-install -S
    run $sudo xbps-install -Syu gcc
}

__install_gcc_via_syspm_on_alpine() {
    run $sudo apk update
    run $sudo apk add gcc libc-dev
}

__install_gcc_via_syspm_on_Linux() {
    if [ -f /etc/os-release ] ; then
        .   /etc/os-release

        case "$ID" in
            opensuse-*) ID=opensuse
        esac

        __install_gcc_via_syspm_on_$ID
    fi
}

__install_gcc_via_syspm_on_MidnightBSD() {
    run $sudo mport index
    run $sudo mport install gcc || true
}

__install_gcc_via_syspm_on_DragonFly() {
    run $sudo pkg update
    run $sudo pkg install -y gcc
}

__install_gcc_via_syspm_on_FreeBSD() {
    run $sudo pkg update
    run $sudo pkg install -y gcc
}

__install_gcc_via_syspm_on_OpenBSD() {
    run $sudo pkg_add gcc
}

__install_gcc_via_syspm_on_NetBSD() {
    run $sudo pkgin -y install gcc
}

__install_gcc_via_syspm_on_SunOS() {
    [ -n "$sudo" ] && sudo=pfexec

    run $sudo pkg refresh
    run $sudo pkg install gcc14
}

__install_gcc_via_syspm_on_Darwin() {
    if command -v brew > /dev/null ; then
        run brew update
        run brew install llvm
    fi
}

__install_gcc_via_syspm_on_Haiku() {
    run $sudo pkgman refresh
    run $sudo pkgman install -y gcc
}

###################################################

unset NATIVE_PLATFORM_TYPE
unset NATIVE_PLATFORM_EUID

NATIVE_PLATFORM_TYPE="$(uname -s)"
NATIVE_PLATFORM_EUID="$(id -u)"

unset sudo

[ "$NATIVE_PLATFORM_EUID" -eq 0 ] || sudo=sudo

###################################################

CC="$(command -v gcc || command -v clang || command -v cc)" || __install_gcc_via_syspm_on_$NATIVE_PLATFORM_TYPE

CC="$(command -v gcc || command -v clang || command -v cc)" || abort 1 "No C Compiler found. tried gcc, clang, cc"

###################################################

[ -z "$*" ] && set -- -std=gnu99 -Os -flto -Wl,-s -o elftool

run exec "$CC" src/*.c -Isrc "$@"
