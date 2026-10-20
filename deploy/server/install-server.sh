#!/bin/sh
# UPR Station installer: native OpenRC/systemd, or explicit --docker mode.
set -eu
fail() { printf '%s\n' "ERROR: $*" >&2; exit 1; }

detect_platform() {
    release=${1:-/etc/os-release}
    machine=${2:-$(uname -m)}
    # Raspbian calls both ARMv6 and ARMv7 userlands armhf; check the CPU first.
    case "$machine" in armv6*) fail 'ARMv6 CPUs are not supported.' ;; esac
    bits=${3:-$(getconf LONG_BIT 2>/dev/null || printf '64')}
    os_id=$(awk -F= '$1=="ID" {gsub(/["\047]/,"",$2); print $2; exit}' "$release")
    os_like=$(awk -F= '$1=="ID_LIKE" {gsub(/["\047]/,"",$2); print $2; exit}' "$release")
    if [ -z "${2:-}" ]; then
        if [ "$os_id" = alpine ] && command -v apk >/dev/null 2>&1; then
            machine=$(apk --print-arch)
        elif command -v dpkg >/dev/null 2>&1; then
            machine=$(dpkg --print-architecture)
        fi
    fi
    case "$os_id $os_like" in
        alpine*) init=openrc ;;
        debian*|ubuntu*|raspbian*|*' debian'*|*' ubuntu'*) init=systemd ;;
        *) fail "Unsupported distribution: $os_id. Supported: Alpine, Debian/Ubuntu and Raspberry Pi OS." ;;
    esac
    case "$machine:$bits" in
        x86_64:64|amd64:64) arch=x86_64 ;;
        aarch64:64|arm64:64) arch=aarch64 ;;
        armv7l:*|armv7:*|armhf:*|aarch64:32|arm64:32) arch=armv7 ;;
        *) fail "Unsupported architecture: $machine/$bits (ARMv6 is not supported)." ;;
    esac
    printf '%s %s\n' "$init" "$arch"
}

# This function is replaced with its embedded Python body in the single-file release installer.
download_package() {
    script_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
    python3 "$script_root/scripts/download-release.py" "$WORK" "$VERSION" "$ARCH" "$MODE"
}

cleanup() {
    case "${WORK:-}" in /var/tmp/upr-install.*) rm -rf -- "$WORK" ;; esac
    if [ "${LOCKED:-0}" = 1 ]; then rmdir /run/lock/upr-station-install 2>/dev/null || true; fi
}

install_docker() {
    command -v docker >/dev/null 2>&1 || fail 'Docker must already be installed. This script does not install the Docker engine.'
    docker info >/dev/null 2>&1 || fail 'Docker daemon unavailable.'
    if command -v rc-service >/dev/null 2>&1 && rc-service ultimatepr-station status >/dev/null 2>&1; then fail 'Native station is running; stop it explicitly before choosing Docker.'; fi
    if command -v systemctl >/dev/null 2>&1 && systemctl is-active --quiet ultimatepr-station; then fail 'Native station is running; stop it explicitly before choosing Docker.'; fi
    old=''
    if docker container inspect upr-station >/dev/null 2>&1; then
        managed=$(docker inspect --format '{{index .Config.Labels "org.upr.managed"}}' upr-station)
        [ "$managed" = true ] || fail 'Existing upr-station container is not managed by this installer.'
        if [ "$LISTEN_SET" = 0 ]; then
            LISTEN=$(docker inspect upr-station | python3 -c 'import json,sys; p=json.load(sys.stdin)[0]["HostConfig"]["PortBindings"]["8080/tcp"][0]; ip=p["HostIp"] or "0.0.0.0"; print(("["+ip+"]" if ":" in ip else ip)+":"+p["HostPort"])')
        fi
        old="upr-station-backup-$(date +%Y%m%d%H%M%S)"
        docker container inspect "$old" >/dev/null 2>&1 && fail 'Backup container already exists.'
    fi
    docker load --input "$PACKAGE"
    docker image inspect "upr-station:$VERSION-$ARCH" >/dev/null
    docker volume create upr-station-data >/dev/null
    if [ -n "$old" ]; then docker stop --time 30 upr-station >/dev/null; docker rename upr-station "$old"; fi
    if docker run -d --name upr-station --label org.upr.managed=true --restart unless-stopped \
        --cap-drop ALL --security-opt no-new-privileges \
        -p "$LISTEN:8080" -v upr-station-data:/var/lib/ultimatepr "upr-station:$VERSION-$ARCH" >/dev/null; then
        for _ in $(seq 1 60); do
            if [ "$(docker inspect --format '{{.State.Health.Status}}' upr-station 2>/dev/null || true)" = healthy ]; then
                printf 'UPR Station ready at http://%s (volume upr-station-data preserved).\n' "$LISTEN"
                [ -z "$old" ] || printf 'Previous container retained, stopped: %s\n' "$old"
                return
            fi
            sleep 1
        done
    fi
    # Remove only the newly created managed container, never its volume.
    if [ "$(docker inspect --format '{{index .Config.Labels "org.upr.managed"}}' upr-station 2>/dev/null || true)" = true ]; then docker rm -f upr-station >/dev/null; fi
    if [ -n "$old" ]; then docker rename "$old" upr-station; docker start upr-station >/dev/null; fi
    fail 'New container failed its health check; previous container restored when available.'
}

install_native() {
    PKG="$WORK/package"
    mkdir "$PKG"
    # Reject links, devices, absolute paths and traversal before extraction.
    python3 - "$PACKAGE" "$PKG" <<'PY'
import pathlib,shutil,sys,tarfile
root=pathlib.Path(sys.argv[2]).resolve()
with tarfile.open(sys.argv[1], 'r:gz') as archive:
    total=0
    for item in archive.getmembers():
        path=pathlib.PurePosixPath(item.name)
        if path.is_absolute() or '..' in path.parts or not item.isfile():
            raise SystemExit('Unsafe archive member')
        total+=item.size
        if total>256*1024*1024: raise SystemExit('Package is too large')
        target=root/path
        if not target.resolve().is_relative_to(root): raise SystemExit('Unsafe archive path')
        target.parent.mkdir(parents=True,exist_ok=True)
        with archive.extractfile(item) as source,target.open('xb') as output: shutil.copyfileobj(source,output)
        target.chmod(0o755 if item.mode&0o111 else 0o644)
PY
    [ -x "$PKG/ultimatepr-station" ] || fail 'Binary missing from verified package.'
    if [ -f /var/lib/ultimatepr/update/status.txt ] && grep -Eq '^state=(running|validating|building|stopping|starting)$' /var/lib/ultimatepr/update/status.txt; then fail 'Another update is running.'; fi
    if [ "$INIT" = openrc ]; then
        if [ -L /opt/ultimatepr-station/current ]; then
            # Use the existing supervised lifecycle and rollback rather than starting a second process.
            python3 - "$PKG" "$WORK/update.zip" <<'PY'
import pathlib,sys,zipfile
root=pathlib.Path(sys.argv[1])
with zipfile.ZipFile(sys.argv[2],'w',zipfile.ZIP_DEFLATED) as archive:
    for p in root.rglob('*'):
        if p.is_file(): archive.write(p,p.relative_to(root))
PY
            mkdir -p /var/lib/ultimatepr/update
            sh "$PKG/deploy/alpine/ultimatepr-update" --install "$WORK/update.zip" /var/lib/ultimatepr/update
        else
            printf 'bind=%s\nauth_enabled=true\n' "$LISTEN" > "$PKG/deploy/alpine/web.conf.example"
            sh "$PKG/deploy/alpine/install-server-linux.sh" "$PKG/ultimatepr-station"
        fi
        cp -R "$PKG/licenses" /opt/ultimatepr-station/current/
        cp "$PKG/LICENSE" "$PKG/PACKAGE-MANIFEST.txt" /opt/ultimatepr-station/current/
        return
    fi
    command -v systemctl >/dev/null 2>&1 || fail 'systemd unavailable; no service changes made.'
    systemctl show --property=Version --value >/dev/null 2>&1 || fail 'systemd is not running; no service changes made.'
    if [ ! -L /opt/ultimatepr-station/current ] && systemctl cat ultimatepr-station >/dev/null 2>&1; then
        fail 'Unmanaged existing service detected. Migrate its configuration manually before installation.'
    fi
    INSTALL_ROOT=/opt/ultimatepr-station
    RELEASE="$INSTALL_ROOT/releases/$VERSION-$ARCH"
    [ ! -e "$RELEASE" ] || fail 'This release directory already exists; refusing to overwrite it.'
    old_target=$(readlink "$INSTALL_ROOT/current" 2>/dev/null || true)
    if [ -e "$INSTALL_ROOT/current" ] && [ ! -L "$INSTALL_ROOT/current" ]; then fail 'Current installation is not a controlled symlink.'; fi
    case "$old_target" in ''|/opt/ultimatepr-station/releases/*) ;; *) fail 'Existing release symlink is outside the managed releases directory.' ;; esac
    getent group ultimatepr >/dev/null || groupadd --system ultimatepr
    id ultimatepr >/dev/null 2>&1 || useradd --system --gid ultimatepr --home-dir /var/lib/ultimatepr --shell /usr/sbin/nologin ultimatepr
    install -d -m 0755 "$INSTALL_ROOT/releases" "$RELEASE"
    install -d -m 0750 -o ultimatepr -g ultimatepr /etc/ultimatepr /var/lib/ultimatepr
    install -m 0755 "$PKG/ultimatepr-station" "$RELEASE/ultimatepr-station"
    cp -R "$PKG/licenses" "$RELEASE/"
    cp "$PKG/LICENSE" "$PKG/PACKAGE-MANIFEST.txt" "$RELEASE/"
    if [ ! -e /etc/ultimatepr/station.conf ]; then install -m 0640 -o ultimatepr -g ultimatepr "$PKG/deploy/alpine/station.conf.example" /etc/ultimatepr/station.conf; fi
    if [ ! -e /etc/ultimatepr/web.conf ]; then
        printf 'bind=%s\nauth_enabled=true\n' "$LISTEN" > /etc/ultimatepr/web.conf
        chown ultimatepr:ultimatepr /etc/ultimatepr/web.conf
        chmod 0640 /etc/ultimatepr/web.conf
    fi
    # Existing configuration, accounts, messages and files are never overwritten.
    if getent group dialout >/dev/null; then usermod -a -G dialout ultimatepr; fi
    if [ -f /etc/systemd/system/ultimatepr-station.service ]; then cp -p /etc/systemd/system/ultimatepr-station.service "$WORK/previous-service";fi
    install -m 0644 "$PKG/deploy/server/ultimatepr-station.service" /etc/systemd/system/ultimatepr-station.service
    systemctl daemon-reload
    systemctl stop ultimatepr-station || true
    ln -s "$RELEASE" "$INSTALL_ROOT/.current-$$"
    mv -Tf "$INSTALL_ROOT/.current-$$" "$INSTALL_ROOT/current"
    systemctl enable ultimatepr-station
    systemctl start ultimatepr-station || true
    HEALTH_URL=$(python3 - <<'PY'
from pathlib import Path
bind=next((line.split('=',1)[1].strip() for line in Path('/etc/ultimatepr/web.conf').read_text().splitlines() if line.startswith('bind=')),'127.0.0.1:8080')
host,port=bind.rsplit(':',1)
if host=='0.0.0.0': host='127.0.0.1'
if host in ['[::]','::']: host='[::1]'
print('http://'+host+':'+port+'/healthz')
PY
    )
    for _ in $(seq 1 30); do
        if systemctl is-active --quiet ultimatepr-station && curl -fsS --max-time 2 "$HEALTH_URL" | grep -qx ok; then
            printf 'UPR Station installed; configuration preserved in /etc/ultimatepr.\n'
            return
        fi
        sleep 1
    done
    systemctl stop ultimatepr-station || true
    if [ -f "$WORK/previous-service" ]; then
        install -m 0644 "$WORK/previous-service" /etc/systemd/system/ultimatepr-station.service
        systemctl daemon-reload
    elif [ -z "$old_target" ]; then
        systemctl disable ultimatepr-station || true
    fi
    if [ -n "$old_target" ]; then
        ln -s "$old_target" "$INSTALL_ROOT/.restore-$$"
        mv -Tf "$INSTALL_ROOT/.restore-$$" "$INSTALL_ROOT/current"
        systemctl start ultimatepr-station || true
    fi
    fail 'Health check failed. Previous executable restored when available; see journalctl -u ultimatepr-station.'
}

main() {
    VERSION='';MODE=native;LISTEN=127.0.0.1:8080;LISTEN_SET=0;DRY_RUN=0
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --version) [ "$#" -ge 2 ] || fail 'Missing version';VERSION=$2;shift 2 ;;
            --docker) MODE=docker;shift ;;
            --listen) [ "$#" -ge 2 ] || fail 'Missing listen address';LISTEN=$2;LISTEN_SET=1;shift 2 ;;
            --dry-run) DRY_RUN=1;shift ;;
            --help) printf 'Usage: sudo sh install-server.sh --version RELEASE_TAG [--docker] [--listen IP:PORT] [--dry-run]\n';return ;;
            *) fail "Unknown option: $1" ;;
        esac
    done
    case "$VERSION" in ''|*[!A-Za-z0-9._-]*) fail 'Specify a valid --version release tag.' ;; esac
    case "$LISTEN" in ''|*[!0-9a-fA-F:.\[\]]*) fail 'Listen must be a numeric IP:PORT.' ;; esac
    platform=$(detect_platform);INIT=${platform% *};ARCH=${platform#* }
    if [ "$MODE" = native ] && [ "$INIT" = openrc ]; then
        case "$LISTEN" in 127.0.0.1:8080|0.0.0.0:8080) ;; *) fail 'Alpine updater currently supports IPv4 on port 8080 only.' ;; esac
    fi
    printf 'Platform: %s/%s; mode: %s; release: %s\n' "$INIT" "$ARCH" "$MODE" "$VERSION"
    [ "$DRY_RUN" = 0 ] || return 0
    [ "$(id -u)" -eq 0 ] || fail 'Run with sudo/doas or as root.'
    install -d /run/lock
    mkdir /run/lock/upr-station-install 2>/dev/null || fail 'Another installer is running.'
    LOCKED=1;trap cleanup EXIT;trap 'cleanup;exit 1' HUP INT TERM
    if [ "$INIT" = openrc ]; then apk add --no-cache python3 curl ca-certificates tar gzip unzip; else
        command -v python3 >/dev/null && command -v curl >/dev/null && command -v unzip >/dev/null || {
            apt-get update;apt-get install -y python3 curl ca-certificates tar gzip unzip;
        }
    fi
    python3 - "$LISTEN" <<'PY'
import ipaddress,sys
host,port=sys.argv[1].rsplit(':',1)
ipaddress.ip_address(host.strip('[]'))
assert 1<=int(port)<=65535, 'Invalid port'
PY
    WORK=$(mktemp -d /var/tmp/upr-install.XXXXXX)
    PACKAGE=$(download_package)
    VERSION=$(cat "$WORK/release-tag.txt")
    unset GITHUB_TOKEN GH_TOKEN
    if [ "$MODE" = docker ]; then install_docker;else install_native;fi
    printf 'First-run account: change the initial password before exposing the panel. Refresh the browser with Ctrl+F5.\n'
}
if [ "${UPR_INSTALLER_LIBRARY:-0}" != 1 ]; then main "$@";fi
