#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
fixture=$(mktemp -d)
trap 'rm -rf -- "$fixture"' EXIT
${UPR_TEST_PYTHON:-python3} scripts/emit-installer.py --bootstrap "$fixture/install.sh"
sh -n "$fixture/install.sh"
export UPR_INSTALLER_LIBRARY=1
source "$fixture/install.sh"
printf 'ID=ubuntu\n' > "$fixture/ubuntu"
[[ "$(detect_platform "$fixture/ubuntu" x86_64 64)" == 'systemd x86_64' ]]
detect_platform() { printf 'systemd x86_64\n'; }
main --help >/dev/null
main --dry-run >/dev/null
if (main --version '../bad' --dry-run >/dev/null 2>&1); then exit 1;fi
if (main --terminal --dry-run >/dev/null 2>&1); then exit 1;fi
printf 'ID=alpine\n' > "$fixture/alpine"
[[ "$(source "$fixture/install.sh";detect_platform "$fixture/alpine" aarch64 64)" == 'openrc aarch64' ]]
main --docker --dry-run >/dev/null
echo 'PASS: standalone Station bootstrap, detection, validation and product isolation.'
