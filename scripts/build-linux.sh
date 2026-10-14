#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${ROOT_DIR}/release/linux"
PLATFORM="${PLATFORM:-linux/$(uname -m)}"
IMAGE="ultimatepr-station-build:local"
CONTAINER="ultimatepr-station-build-export-$$"

case "${1:-}" in
  --help|-h)
    cat <<'EOF'
Builds the static Alpine/musl Linux binary through the repository Dockerfile.

Usage:
  ./scripts/build-linux.sh
  PLATFORM=linux/amd64 ./scripts/build-linux.sh
  PLATFORM=linux/arm64 ./scripts/build-linux.sh

The selected platform must be supported by the local Docker engine. Cross-
architecture builds may require Docker Desktop emulation or buildx/QEMU.
EOF
    exit 0
    ;;
esac

command -v docker >/dev/null 2>&1 || {
  echo "Docker is required. Install Docker Engine or Docker Desktop first." >&2
  exit 1
}

mkdir -p "${OUTPUT_DIR}"
echo "Building ${PLATFORM} from ${ROOT_DIR}..."
docker build --platform "${PLATFORM}" --target build --tag "${IMAGE}" "${ROOT_DIR}"

cleanup() {
  docker rm "${CONTAINER}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker create --name "${CONTAINER}" "${IMAGE}" >/dev/null
docker cp "${CONTAINER}:/src/target/release/ultimatepr-station" "${OUTPUT_DIR}/ultimatepr-station"
chmod +x "${OUTPUT_DIR}/ultimatepr-station"

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "${OUTPUT_DIR}/ultimatepr-station" > "${OUTPUT_DIR}/ultimatepr-station.sha256"
else
  shasum -a 256 "${OUTPUT_DIR}/ultimatepr-station" > "${OUTPUT_DIR}/ultimatepr-station.sha256"
fi

cat > "${OUTPUT_DIR}/README.md" <<EOF
# UltimatePR Station Linux build

Architecture: ${PLATFORM}

Files:

- ultimatepr-station — statically linked Linux binary built with Alpine/musl;
- ultimatepr-station.sha256 — SHA-256 checksum.

Run locally:

    chmod +x ultimatepr-station
    ./ultimatepr-station --config ./station.conf

The binary does not contain a station callsign, service passwords, operator token,
or personal data. Create the configuration and any service secret files on the
target machine. Never copy another operator's data/ directory into this folder.
EOF

echo "Created: ${OUTPUT_DIR}/ultimatepr-station"
echo "Checksum: ${OUTPUT_DIR}/ultimatepr-station.sha256"
echo "Readme: ${OUTPUT_DIR}/README.md"
