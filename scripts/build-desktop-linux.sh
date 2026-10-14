#!/usr/bin/env bash
set -Eeuo pipefail
ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "$(uname -m)" != x86_64 ]]; then
  echo 'Desktop v1 packaging supports x86_64 only.' >&2
  exit 1
fi
cargo fetch --locked --manifest-path desktop/Cargo.toml --target x86_64-unknown-linux-gnu
python3 scripts/desktop-licenses.py
if [[ -f desktop/target/legal/MISSING-LICENSE-TEXTS.txt ]]; then
  echo 'Resolve missing license texts before packaging; see desktop/target/legal/MISSING-LICENSE-TEXTS.txt.' >&2
  exit 1
fi
cd desktop
cargo tauri build --bundles deb -- --locked
cd "$ROOT_DIR"
OUTPUT_DIR="$ROOT_DIR/release/desktop/linux-x64-$(date +%Y%m%d-%H%M%S)"
mkdir "$OUTPUT_DIR" -p
shopt -s nullglob
packages=(desktop/target/release/bundle/deb/*_amd64.deb)
if [[ ${#packages[@]} -ne 1 ]]; then
  echo 'Expected exactly one freshly built amd64 DEB; inspect bundle directory.' >&2
  exit 1
fi
cp -- "${packages[0]}" "$OUTPUT_DIR/"
cp -R -- docs LICENSE TRADEMARKS.md "$OUTPUT_DIR/"
(cd "$OUTPUT_DIR" && find . -type f ! -name SHA256SUMS.txt -print0 | sort -z | xargs -0 sha256sum > SHA256SUMS.txt)
printf 'DEB and documentation: %s\n' "$OUTPUT_DIR"
