#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
ARTIFACT_DIR="$ROOT_DIR/artifacts/alpine-server"

die() { echo "BŁĄD: $*" >&2; exit 1; }
need_file() { [ -f "$ROOT_DIR/$1" ] || die "Brak pliku źródłowego: $1"; }
as_root() {
    if [ "$(id -u)" -eq 0 ]; then "$@"
    elif command -v doas >/dev/null 2>&1; then doas "$@"
    elif command -v sudo >/dev/null 2>&1; then sudo "$@"
    else die "Potrzebny root, doas albo sudo."; fi
}

for file in Cargo.toml Cargo.lock src/lib.rs src/main.rs web/index.html; do need_file "$file"; done
[ -f /etc/alpine-release ] || die "Ten skrypt jest przeznaczony dla Alpine Linux."
command -v apk >/dev/null 2>&1 || die "Brak apk."

build_deps_added=0
if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
    echo "Brak Rust/Cargo. Instaluję build-base, rust, cargo, ca-certificates."
    as_root apk add --no-cache ca-certificates
    as_root apk add --no-cache --virtual .ultimatepr-build-deps build-base rust cargo
    build_deps_added=1
fi

mkdir -p "$ARTIFACT_DIR"
echo "Kompiluję UltimatePR Station server dla Alpine/musl..."
(cd "$ROOT_DIR" && cargo build --release --locked --bin ultimatepr-station)
cp "$ROOT_DIR/target/release/ultimatepr-station" "$ARTIFACT_DIR/ultimatepr-station"
chmod 755 "$ARTIFACT_DIR/ultimatepr-station"
sha256sum "$ARTIFACT_DIR/ultimatepr-station" > "$ARTIFACT_DIR/ultimatepr-station.sha256"
echo "Kompilacja zakończona: $ARTIFACT_DIR/ultimatepr-station"

printf 'Czy zainstalować aplikację teraz? [y/N] '
read -r answer
case "$answer" in
    y|Y|yes|YES|tak|TAK)
        "$ROOT_DIR/deploy/alpine/install-server-linux.sh" "$ARTIFACT_DIR/ultimatepr-station"
        rm -rf "$ROOT_DIR/target" "$ROOT_DIR/artifacts"
        if [ "$build_deps_added" -eq 1 ]; then
            as_root apk del .ultimatepr-build-deps || echo "Ostrzeżenie: nie udało się usunąć pakietów kompilacyjnych."
        fi
        echo "Usunięto pliki tymczasowe kompilacji."
        ;;
    *) echo "Instalacja pominięta. Binarka pozostaje w: $ARTIFACT_DIR" ;;
esac
