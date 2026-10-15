"""Produce a single-file installer; source helper stays separately testable."""
from pathlib import Path
import argparse
root=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser()
parser.add_argument('output')
parser.add_argument('--bootstrap', action='store_true')
args=parser.parse_args()
server=(root/'deploy/server/install-server.sh').read_text()
source=(root/'deploy/install/install.sh').read_text() if args.bootstrap else server
if args.bootstrap:
    source=source.replace('# Template. Releases contain a self-contained install.sh emitted by emit-installer.py.', '# Generated self-contained UPR installer. Do not edit.')
    start=source.index('detect_platform() {')
    end=source.index('\n}\n', start)+2
    original=server.index('detect_platform() {')
    original_end=server.index('\n}\n', original)+2
    source=source[:start]+server[original:original_end]+source[end:]
start=source.index('download_package() {')
end=source.index('\n}\n',start)+2
body=(root/'scripts/download-release.py').read_text()
embedded='download_package() {\n    python3 - "$WORK" "$VERSION" "$ARCH" "$MODE" <<\'UPR_PYTHON\'\n'+body+'\nUPR_PYTHON\n}'
output=Path(args.output)
output.parent.mkdir(parents=True,exist_ok=True)
output.write_text(source[:start]+embedded+source[end:],newline='\n')
output.chmod(0o755)
