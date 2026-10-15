#!/usr/bin/env bash
# Generated bootstrap tests, with all installer actions replaced by local mocks.
set -euo pipefail
cd "$(dirname "$0")/../.."
fixture=$(mktemp -d)
trap 'rm -rf -- "$fixture"' EXIT
${UPR_TEST_PYTHON:-python3} scripts/emit-installer.py --bootstrap "$fixture/install.sh"
sh -n "$fixture/install.sh"
export UPR_BOOTSTRAP_LIBRARY=1
source "$fixture/install.sh"
printf 'ID=alpine\n' > "$fixture/alpine"
printf 'ID=ubuntu\n' > "$fixture/ubuntu"
[[ "$(detect_platform "$fixture/alpine" aarch64 64)" == 'openrc aarch64' ]]
[[ "$(detect_platform "$fixture/ubuntu" x86_64 64)" == 'systemd x86_64' ]]
detect_platform() { printf 'systemd x86_64\n'; }
main --help >/dev/null
main --station --dry-run >/dev/null
main --terminal --dry-run >/dev/null
for arguments in '' '--station --terminal' '--terminal --docker' '--terminal --listen 0.0.0.0:8080' '--station --no-launch' '--station --version ../bad'; do
    if (main $arguments --dry-run >/dev/null 2>&1); then echo "Accepted invalid options: $arguments"; exit 1; fi
done
if (detect_platform() { printf 'openrc x86_64\n'; }; main --terminal --dry-run >/dev/null 2>&1); then exit 1; fi
if (detect_platform() { printf 'systemd aarch64\n'; }; main --terminal --dry-run >/dev/null 2>&1); then exit 1; fi
(
    id() { printf '0\n'; }
    if (main --terminal --no-launch >/dev/null 2>&1); then exit 1; fi
)
cat > "$fixture/delegate.sh" <<'FIXTURE'
#!/bin/sh
[ "$GITHUB_TOKEN" = isolated-test-token ] || exit 1
printf '%s\n' "$@" > "$UPR_TEST_LOG"
FIXTURE
download_package() {
    printf vresolved > "$WORK/release-tag.txt"
    if [ "$MODE" = installer ]; then cp "$fixture/delegate.sh" "$WORK/install-server.sh";printf '%s\n' "$WORK/install-server.sh"; else
        printf 'fixture DEB' > "$WORK/package.deb";printf '%s\n' "$WORK/package.deb"
    fi
}
export UPR_TEST_LOG="$fixture/log"
(
    id() { printf '0\n'; }
    export GITHUB_TOKEN=isolated-test-token
    main --station --docker --listen 127.0.0.1:9090 >/dev/null
)
[[ "$(cat "$fixture/log")" == $'--version\nvresolved\n--docker\n--listen\n127.0.0.1:9090' ]]
(
    id() { printf '1000\n'; }
    apt-get() { return 0; }
    python3() { return 0; }
    as_root() { printf '%s\n' "$@" > "$UPR_TEST_LOG"; }
    main --terminal --no-launch >/dev/null
)
grep -Fx -- '--reinstall' "$fixture/log" >/dev/null
grep -Fx -- 'apt-get' "$fixture/log" >/dev/null
(
    id() { printf '1000\n'; }
    apt-get() { return 0; }
    python3() { return 1; }
    download_package() { touch "$fixture/unexpected-download"; exit 99; }
    if (main --terminal --no-launch >/dev/null 2>&1); then exit 1; fi
)
[[ ! -f "$fixture/unexpected-download" ]]
echo 'PASS: generated standalone bootstrap, platform/product validation, root rejection, pinned delegation and same-version DEB reinstall; no real installation.'
