#!/bin/sh
# Template. Releases contain a self-contained install.sh emitted by emit-installer.py.
set -eu
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
detect_platform() {
    fail 'Use install.sh from Releases, not this source template.'
}
download_package() {
    fail 'Use install.sh from Releases, not this source template.'
}
as_root() {
    if [ "$(id -u)" -eq 0 ]; then "$@"; else
        command -v sudo >/dev/null 2>&1 || fail 'sudo unavailable; install Station as root or install sudo first.'
        sudo "$@"
    fi
}
cleanup() {
    # WORK is created only by mktemp under the system temporary directory.
    [ -z "${WORK:-}" ] || rm -rf -- "$WORK"
}
main() {
    PRODUCT='';VERSION=latest;DOCKER=0;LISTEN='';LAUNCH=1;DRY_RUN=0;WORK=''
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --station|--terminal) [ -z "$PRODUCT" ] || fail 'Choose exactly one product.';PRODUCT=${1#--};shift ;;
            --version) [ "$#" -ge 2 ] || fail 'Missing version';VERSION=$2;shift 2 ;;
            --docker) DOCKER=1;shift ;;
            --listen) [ "$#" -ge 2 ] || fail 'Missing listen address';LISTEN=$2;shift 2 ;;
            --no-launch) LAUNCH=0;shift ;;
            --dry-run) DRY_RUN=1;shift ;;
            --help) printf 'Usage: sh install.sh --station|--terminal [--version TAG] [--dry-run]\nStation: [--docker] [--listen IP:PORT]\nTerminal: [--no-launch]\nDefault: newest published release, including pre-releases. Re-run to update.\n';return ;;
            *) fail "Unknown option: $1" ;;
        esac
    done
    [ -n "$PRODUCT" ] || fail 'Choose --station or --terminal; no application is selected automatically.'
    case "$VERSION" in ''|[!A-Za-z0-9]*|*[!A-Za-z0-9._-]*) fail 'Invalid release tag.' ;; esac
    case "$LISTEN" in *[!0-9a-fA-F:.\[\]]*) fail 'Listen must be a numeric IP:PORT.' ;; esac
    platform=$(detect_platform);INIT=${platform% *};ARCH=${platform#* }
    if [ "$PRODUCT" = terminal ]; then
        [ "$DOCKER" = 0 ] && [ -z "$LISTEN" ] || fail '--docker and --listen apply only to Station.'
        [ "$INIT" = systemd ] && [ "$ARCH" = x86_64 ] || fail 'Terminal installer supports Debian/Ubuntu derivatives on x86-64 only.'
        MODE=terminal
    else
        [ "$LAUNCH" = 1 ] || fail '--no-launch applies only to Terminal.'
        MODE=installer
    fi
    printf 'Product: %s; platform: %s; release: %s\n' "$PRODUCT" "$platform" "$VERSION"
    [ "$DRY_RUN" = 0 ] || return 0
    if [ "$PRODUCT" = terminal ]; then
        [ "$(id -u)" -ne 0 ] || fail 'Run the Terminal installer as your normal desktop user, not root.'
        command -v apt-get >/dev/null 2>&1 || fail 'apt-get unavailable.'
    fi
    if ! command -v python3 >/dev/null 2>&1; then
        if [ "$INIT" = openrc ]; then as_root apk add --no-cache python3 ca-certificates; else
            as_root apt-get update
            as_root apt-get install -y python3 ca-certificates
        fi
    fi
    if [ "$PRODUCT" = terminal ]; then
        python3 - <<'UPR_PROCESS' || fail 'Cannot continue: UPR Terminal must be stopped and process checks must succeed.'
import os
from pathlib import Path
for item in Path('/proc').iterdir():
    if not item.name.isdecimal():
        continue
    try:
        target = os.readlink(item/'exe')
        if target.endswith(' (deleted)'):
            target = target[:-len(' (deleted)')]
    except OSError:
        continue
    if Path(target).name == 'ultimatepr-station-desktop':
        raise SystemExit('Close UPR Terminal, including its tray process, before updating.')
UPR_PROCESS
    fi
    # Preserve a token supplied as an environment variable; never place it in arguments or files.
    set +x
    GITHUB_TOKEN=${GITHUB_TOKEN:-${GH_TOKEN:-}}
    export GITHUB_TOKEN
    WORK=$(mktemp -d)
    WORK=$(cd "$WORK" && pwd -P)
    trap cleanup EXIT
    trap 'exit 1' HUP INT TERM
    PACKAGE=$(download_package)
    VERSION=$(cat "$WORK/release-tag.txt")
    printf 'Verified release: %s\n' "$VERSION"
    if [ "$PRODUCT" = station ]; then
        set -- --version "$VERSION"
        [ "$DOCKER" = 0 ] || set -- "$@" --docker
        [ -z "$LISTEN" ] || set -- "$@" --listen "$LISTEN"
        if [ "$(id -u)" -eq 0 ]; then sh "$PACKAGE" "$@"; else
            command -v sudo >/dev/null 2>&1 || fail 'Run Station installation as root or install sudo first.'
            sudo --preserve-env=GITHUB_TOKEN sh "$PACKAGE" "$@"
        fi
    else
        unset GITHUB_TOKEN GH_TOKEN
        # The application version may stay unchanged between test releases.
        # --reinstall makes updates replace the installed DEB even with that same version.
        as_root apt-get install -y --reinstall "$PACKAGE"
        printf 'UPR Terminal installed. User configuration was not removed.\n'
        if [ "$LAUNCH" = 1 ] && { [ -n "${DISPLAY:-}" ] || [ -n "${WAYLAND_DISPLAY:-}" ]; }; then
            command -v ultimatepr-station-desktop >/dev/null 2>&1 || fail 'Installed executable not found.'
            nohup ultimatepr-station-desktop </dev/null >/dev/null 2>&1 &
            printf 'Starting UPR Terminal as the current user.\n'
        else
            printf 'Launch from the application menu or run: ultimatepr-station-desktop\n'
        fi
    fi
}
if [ "${UPR_BOOTSTRAP_LIBRARY:-0}" != 1 ]; then main "$@"; fi
