#!/bin/sh
# Offline repair/bootstrap path for an already installed native Station.
# The tester builder embeds the same reviewed ZIP validator as the root helper.
set -eu
[ "$(id -u)" -eq 0 ] || { echo 'Uruchom jako root (sudo/doas).' >&2; exit 1; }
[ ! -f /.dockerenv ] || { echo 'Docker aktualizuj instalatorem na hoście.' >&2; exit 1; }
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
command -v python3 >/dev/null 2>&1 || { echo 'Wymagany Python 3. Alpine: apk add python3; Debian: apt install python3' >&2; exit 1; }
command -v wget >/dev/null 2>&1 || { echo 'Wymagany wget do sprawdzenia usługi.' >&2; exit 1; }
[ -L /opt/ultimatepr-station/current ] || { echo 'To jest aktualizacja istniejącej instalacji, nie instalator pierwszego uruchomienia.' >&2; exit 1; }
set -- "$ROOT"/alpine/*.zip
[ "$#" = 1 ] && [ -f "$1" ] || { echo 'Brak jednoznacznej paczki Alpine w katalogu alpine/.' >&2; exit 1; }
PACKAGE=$1
ARCH=$(uname -m)
case "$ARCH:$(getconf LONG_BIT)" in x86_64:64) ARCH=x86_64 ;; aarch64:64) ARCH=aarch64 ;; armv7*:32|aarch64:32) ARCH=armv7 ;; *) echo 'Nieobsługiwana architektura.' >&2; exit 1 ;; esac
STAGE=$(mktemp -d /var/tmp/upr-tester.XXXXXX)
trap 'rm -rf "$STAGE"' EXIT
python3 -I - "$PACKAGE" "$ARCH" "$STAGE/package" <<'UPR_VALIDATE'
raise SystemExit('Unexpanded helper: build a binary package with package-server.py')
UPR_VALIDATE
install -d -m 0750 -o ultimatepr -g ultimatepr /var/lib/ultimatepr/update
sh "$STAGE/package/deploy/alpine/ultimatepr-update" --install "$PACKAGE" /var/lib/ultimatepr/update
