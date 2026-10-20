#!/usr/bin/env bash
# Detection and argument tests only: never install packages or touch a service.
set -euo pipefail
cd "$(dirname "$0")/../.."
export UPR_INSTALLER_LIBRARY=1
source deploy/server/install-server.sh
fixture=$(mktemp -d)
trap 'rm -rf -- "$fixture"' EXIT
printf 'ID=alpine\n' > "$fixture/alpine"
printf 'ID=debian\n' > "$fixture/debian"
printf 'ID=raspbian\nID_LIKE=debian\n' > "$fixture/raspbian"
printf 'ID=linuxmint\nID_LIKE="ubuntu debian"\n' > "$fixture/mint"
printf 'ID=unknown\n' > "$fixture/unknown"
[[ "$(detect_platform "$fixture/alpine" x86_64 64)" == 'openrc x86_64' ]]
[[ "$(detect_platform "$fixture/alpine" aarch64 64)" == 'openrc aarch64' ]]
[[ "$(detect_platform "$fixture/debian" x86_64 64)" == 'systemd x86_64' ]]
[[ "$(detect_platform "$fixture/raspbian" aarch64 32)" == 'systemd armv7' ]]
[[ "$(detect_platform "$fixture/raspbian" aarch64 64)" == 'systemd aarch64' ]]
[[ "$(detect_platform "$fixture/raspbian" armv7l 32)" == 'systemd armv7' ]]
[[ "$(detect_platform "$fixture/raspbian" armhf 32)" == 'systemd armv7' ]]
[[ "$(detect_platform "$fixture/mint" x86_64 64)" == 'systemd x86_64' ]]
if (detect_platform "$fixture/unknown" x86_64 64 >/dev/null 2>&1); then exit 1;fi
if (detect_platform "$fixture/raspbian" armv6l 32 >/dev/null 2>&1); then exit 1;fi
if (main --version '../bad' --dry-run >/dev/null 2>&1); then exit 1;fi
if (main --version vtest --listen '127.0.0.1:8080;bad' --dry-run >/dev/null 2>&1); then exit 1;fi
main --help >/dev/null
echo 'PASS: Alpine/Debian/Raspberry Pi OS detection, 32/64-bit architectures and unsafe input rejection.'

# Source only the privilege wrapper and prerequisites, stopping before any
# application filesystem operation. All system commands below are mocks.
(
    calls=()
    id() { printf '0\n'; }
    apk() { calls+=("apk $*"); }
    python3() { calls+=("python $*"); }
    source <(sed -n '/^as_root() {/,/^as_root install -d /p' deploy/alpine/install-server-linux.sh | sed '$d')
    [[ "${calls[0]}" == 'apk add --no-cache python3 wget ca-certificates' ]]
    [[ "${calls[1]}" == 'python -I -c import sys, ssl, urllib.request, zipfile;'* ]]
    [[ "${#calls[@]}" == 2 ]]
)
for failing in apk python3; do
    if bash -e -c '
        id() { printf "0\n"; }
        apk() { [[ "$1" == add ]]; }
        python3() { :; }
        if [[ "$1" == apk ]]; then apk() { return 17; }; else python3() { return 18; }; fi
        source <(sed -n "/^as_root() {/,/^as_root install -d /p" deploy/alpine/install-server-linux.sh | sed "\$d")
    ' -- "$failing"; then
        echo "FAIL: installer ignored prerequisite failure: $failing" >&2
        exit 1
    fi
done
echo 'PASS: persistent Python/HTTPS/ZIP prerequisites precede installation; package-manager and Python failures abort.'
