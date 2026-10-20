#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
BINARY=${1:-"$ROOT_DIR/artifacts/alpine-server/ultimatepr-station"}
INSTALL_ROOT=/opt/ultimatepr-station
DATA_DIR=/var/lib/ultimatepr
CONFIG_DIR=/etc/ultimatepr
SERVICE=ultimatepr-station

[ -x "$BINARY" ] || { echo "BŁĄD: brak skompilowanej binarki: $BINARY" >&2; exit 1; }
if [ -e "$INSTALL_ROOT/current" ] || [ -L "$INSTALL_ROOT/current" ]; then
    echo "BŁĄD: istnieje już instalacja UltimatePR Station w $INSTALL_ROOT." >&2
    echo "Użyj helpera aktualizacji zamiast ponownie uruchamiać instalator." >&2
    exit 1
fi
as_root() {
    if [ "$(id -u)" -eq 0 ]; then "$@"
    elif command -v doas >/dev/null 2>&1; then doas "$@"
    elif command -v sudo >/dev/null 2>&1; then sudo "$@"
    else echo "BŁĄD: potrzebny root, doas albo sudo." >&2; exit 1; fi
}

as_root install -d -m 0755 "$INSTALL_ROOT" "$INSTALL_ROOT/releases" "$DATA_DIR" "$CONFIG_DIR" /usr/local/libexec
if ! id ultimatepr >/dev/null 2>&1; then
    as_root addgroup -S ultimatepr
    as_root adduser -S -D -H -s /sbin/nologin -G ultimatepr ultimatepr
fi
RELEASE="$INSTALL_ROOT/releases/initial"
as_root install -d -m 0755 "$RELEASE"
as_root install -m 0755 "$BINARY" "$RELEASE/ultimatepr-station"
as_root ln -s "$RELEASE" "$INSTALL_ROOT/current"
as_root install -m 0644 "$ROOT_DIR/deploy/alpine/station.conf.example" "$CONFIG_DIR/station.conf.example"
as_root install -m 0644 "$ROOT_DIR/deploy/alpine/web.conf.example" "$CONFIG_DIR/web.conf.example"
if [ ! -e "$CONFIG_DIR/station.conf" ]; then
    as_root cp "$ROOT_DIR/deploy/alpine/station.conf.example" "$CONFIG_DIR/station.conf"
fi
if [ ! -e "$CONFIG_DIR/web.conf" ]; then
    as_root cp "$ROOT_DIR/deploy/alpine/web.conf.example" "$CONFIG_DIR/web.conf"
fi
as_root chown -R ultimatepr:ultimatepr "$DATA_DIR" "$CONFIG_DIR"
as_root install -d -o ultimatepr -g ultimatepr -m 0750 "$DATA_DIR/update"
as_root chmod 0750 "$CONFIG_DIR"
as_root chmod 0640 "$CONFIG_DIR/station.conf" "$CONFIG_DIR/web.conf"
as_root install -m 0755 "$ROOT_DIR/deploy/alpine/start-server-linux.sh" "$INSTALL_ROOT/ultimatepr-station-start"
as_root install -m 0755 "$ROOT_DIR/deploy/alpine/ultimatepr-station-run" /usr/local/libexec/ultimatepr-station-run
as_root install -m 0755 "$ROOT_DIR/deploy/alpine/ultimatepr-station.initd" "/etc/init.d/$SERVICE"
as_root install -m 0755 "$ROOT_DIR/deploy/alpine/ultimatepr-update" /usr/local/libexec/ultimatepr-update
as_root chown root:root /usr/local/libexec/ultimatepr-update
if ! command -v python3 >/dev/null 2>&1; then
    as_root apk add --no-cache python3
fi
if ! command -v doas >/dev/null 2>&1; then
    as_root apk add --no-cache doas
fi
DOAS_RULE=$(awk '!/^#/ && NF {print; exit}' "$ROOT_DIR/deploy/alpine/ultimatepr-update.doas")
if ! as_root grep -Fq "$DOAS_RULE" /etc/doas.conf 2>/dev/null; then
    as_root sh -c "printf '%s\\n' '$DOAS_RULE' >> /etc/doas.conf"
fi
as_root chmod 0600 /etc/doas.conf
as_root rc-update add "$SERVICE" default
if ! as_root rc-service "$SERVICE" start; then
    echo "BŁĄD: usługa nie uruchomiła się. Sprawdź: rc-service $SERVICE status" >&2
    exit 1
fi

# OpenRC may report that the supervisor was started even when the child
# process exits immediately. Confirm the application itself before reporting
# a successful installation.
if ! command -v wget >/dev/null 2>&1; then
    as_root apk add --no-cache wget
fi
healthy=false
for _ in 1 2 3 4 5 6 7 8 9 10; do
    health="$(wget -qO- http://127.0.0.1:8080/healthz 2>/dev/null || true)"
    if [ "$health" = "ok" ]; then
        healthy=true
        break
    fi
    sleep 1
done
if [ "$healthy" != true ]; then
    echo "BŁĄD: usługa OpenRC wystartowała, ale panel nie odpowiada na 127.0.0.1:8080/healthz." >&2
    echo "Sprawdź: rc-service $SERVICE status" >&2
    echo "Log: /var/log/ultimatepr-station.log" >&2
    exit 1
fi

cat <<EOF
Instalacja zakończona.
Binarka:      $INSTALL_ROOT/current/ultimatepr-station
Konfiguracja: $CONFIG_DIR/station.conf
Panel WWW:    $CONFIG_DIR/web.conf
Dane:         $DATA_DIR
Usługa OpenRC: $SERVICE (uruchomiona i dodana do autostartu)
Kontrola panelu: http://127.0.0.1:8080/healthz = ok

Przed uruchomieniem ustaw callsign i interfejs TNC w:
  $CONFIG_DIR/station.conf
EOF
