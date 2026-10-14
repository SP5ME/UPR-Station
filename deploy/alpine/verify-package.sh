#!/bin/sh
set -eu
ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
for file in Cargo.toml Cargo.lock src/lib.rs src/main.rs web/index.html deploy/alpine/build-server-linux.sh deploy/alpine/install-server-linux.sh deploy/alpine/ultimatepr-station-run deploy/alpine/ultimatepr-station.initd deploy/alpine/ultimatepr-update deploy/alpine/ultimatepr-update.doas deploy/alpine/verify-service.sh; do
    [ -e "$ROOT_DIR/$file" ] || { echo "Brak: $file" >&2; exit 1; }
done
echo "Kontrola paczki Alpine: OK"
