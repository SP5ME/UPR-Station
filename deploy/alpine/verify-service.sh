#!/bin/sh
set -eu

SERVICE=ultimatepr-station
INSTALL_ROOT=/opt/ultimatepr-station
CONFIG_DIR=/etc/ultimatepr

fail() { echo "BŁĄD: $*" >&2; exit 1; }
need_file() { [ -e "$1" ] || fail "Brak: $1"; }

[ "$(id -u)" -eq 0 ] || fail "Uruchom jako root."
command -v rc-service >/dev/null 2>&1 || fail "Brak OpenRC (rc-service)."
command -v wget >/dev/null 2>&1 || fail "Brak wget do kontroli healthz."

need_file "/etc/init.d/$SERVICE"
[ -L "$INSTALL_ROOT/current" ] || fail "current nie jest dowiązaniem do wydania."
need_file "$INSTALL_ROOT/current/ultimatepr-station"
need_file "$CONFIG_DIR/station.conf"
need_file "$CONFIG_DIR/web.conf"

rc-service "$SERVICE" status >/dev/null 2>&1 || fail "Usługa nie działa: rc-service $SERVICE status"

health=$(wget -qO- http://127.0.0.1:8080/healthz 2>/dev/null || true)
if [ "$health" != "ok" ]; then
    echo "Diagnostyka usługi:" >&2
    rc-service "$SERVICE" status >&2 2>&1 || true
    ps | grep '[u]ltimatepr-station' >&2 || true
    if [ -f /var/log/ultimatepr-station.log ]; then
        echo "--- /var/log/ultimatepr-station.log (ostatnie 40 linii) ---" >&2
        tail -n 40 /var/log/ultimatepr-station.log >&2 || true
    fi
    fail "Panel WWW nie odpowiada na http://127.0.0.1:8080/healthz"
fi

echo "OK: usługa $SERVICE działa."
echo "OK: aktywna binarka: $(readlink -f "$INSTALL_ROOT/current/ultimatepr-station")"
echo "OK: panel odpowiada na porcie 8080."
