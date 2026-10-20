#!/usr/bin/env bash
# Offline checks of the updater lifecycle; no real processes or services touched.
set -euo pipefail
cd "$(dirname "$0")/../.."
source <(sed -n '/^station_pids() {/,/^mkdir -p /p' deploy/alpine/ultimatepr-update | sed '$d')

SERVICE=ultimatepr-station
MANAGER=openrc
CURRENT=/opt/ultimatepr-station/current
remaining=42
calls=()
station_pids() { printf '%s\n' "$remaining"; }
rc-service() { calls+=("$2"); }
sleep() { :; }
kill() { calls+=("$1"); remaining=''; }
stop_station_processes
[[ "${calls[*]}" == 'stop zap -TERM' && -z "$remaining" ]]
echo 'PASS: orphan process is terminated after OpenRC stop'

remaining=42
actual=/opt/ultimatepr-station/releases/old/ultimatepr-station
expected=/opt/ultimatepr-station/releases/new/ultimatepr-station
readlink() {
    case "$2" in
        "$CURRENT/ultimatepr-station") printf '%s\n' "$expected" ;;
        /proc/42/exe) printf '%s\n' "$actual" ;;
        *) return 1 ;;
    esac
}
if current_process_running; then
    echo 'FAIL: old executable accepted' >&2
    exit 1
fi
actual=$expected
current_process_running
echo 'PASS: executable must match the current release'

calls=()
wait_for_health() { calls+=(health); }
wait_for_current_process() { calls+=(process); }
start_and_wait
[[ "${calls[*]}" == 'zap start health process' ]]
echo 'PASS: startup checks health and release identity'

calls=()
MANAGER=systemd
systemctl() { calls+=("$1"); [[ "$1" != is-active ]]; }
stop_station_processes
[[ "${calls[*]}" == 'stop is-active' ]]
calls=()
start_and_wait
[[ "${calls[*]}" == 'start health process' ]]
echo 'PASS: systemd lifecycle uses only the managed service'

calls=()
OLD_TARGET=/opt/ultimatepr-station/releases/old
stop_station_processes() { calls+=(stop); }
start_and_wait() { calls+=(start); }
rm() { calls+=(remove-current); }
ln() { [[ "$2" == "$OLD_TARGET" && "$3" == "$CURRENT" ]]; calls+=(restore-current); }
rollback
[[ "${calls[*]}" == 'stop remove-current restore-current start' ]]
echo 'PASS: rollback restores the previous release and checks startup'
