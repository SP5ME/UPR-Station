"""Offline product/publication contracts, including rejected foreign release assets."""
import hashlib
import os
from pathlib import Path
import re
import runpy
import subprocess
import sys
import tempfile
from unittest.mock import patch

root=Path(__file__).resolve().parents[2]
version=re.search(r'^version\s*=\s*"([^"]+)"', (root/'Cargo.toml').read_text(), re.M)[1]
assert f'name = "ultimatepr-station"\nversion = "{version}"' in (root/'Cargo.lock').read_text()
DESKTOP=False
repo='UPR-Terminal' if DESKTOP else 'UPR-Station'
assert ('SP5ME/'+repo+'/releases') in (root/'scripts/download-release.py').read_text()
assert (root/'desktop/Cargo.toml').exists()==DESKTOP
assert (root/'src/main.rs').exists()!=DESKTOP
assert (root/'src/telegram.rs').exists()!=DESKTOP
assert (root/'src/mqtt.rs').exists()!=DESKTOP
assert (root/'Dockerfile').exists()!=DESKTOP
workflow=(root/'.github/workflows/release.yml').read_text()
assert ('  desktop:\n' in workflow)==DESKTOP
assert ('  server:\n' in workflow)!=DESKTOP
assert '      prerelease:\n' in workflow and 'flags+=(--prerelease)' in workflow
assert '--draft=false --latest' in workflow
assert 'proxmox' not in (root/'scripts/download-release.py').read_text().lower()
context=(root/'.dockerignore').read_text()
for name in ['update-package.py','update-release.py']:
    assert '!deploy/server/'+name in context, 'Embedded updater source must be available during Docker builds'

with tempfile.TemporaryDirectory() as directory:
    out=Path(directory)
    required=(['UPR-Terminal-vtest-windows-x86_64-setup.exe','UPR-Terminal-vtest-linux-x86_64.deb','UPR-Terminal-vtest-linux-x86_64.AppImage','install.sh'] if DESKTOP else
        ['install-server.sh','install.sh']+[f'UPR-Station-vtest-{kind}-{arch}.{ext}' for arch in ['x86_64','aarch64','armv7'] for kind,ext in [('linux','tar.gz'),('alpine','zip'),('docker','tar.gz')]])
    for name in required:(out/name).write_bytes(b'isolated release contract fixture')
    script=root/'scripts/finalize-release.py'
    subprocess.run([sys.executable,str(script),'vtest',str(out)],check=True,env={**os.environ,'UPR_RELEASE_PRERELEASE':'false'})
    notes=(out/'RELEASE-NOTES.md').read_text(encoding='utf-8')
    assert f'Wydanie pełne. Wersja aplikacji: {version}.' in notes
    sums=(out/'SHA256SUMS.txt').read_text().splitlines()
    for line in sums:
        digest,name=line.split(None,1)
        assert hashlib.sha256((out/name).read_bytes()).hexdigest()==digest
    subprocess.run([sys.executable,str(script),'vtest',str(out)],check=True,env={**os.environ,'UPR_RELEASE_PRERELEASE':'true'})
    assert 'Wydanie testowe (pre-release).' in (out/'RELEASE-NOTES.md').read_text(encoding='utf-8')
    foreign=out/('UPR-Station-vtest-alpine-x86_64.zip' if DESKTOP else 'UPR-Terminal-vtest-windows-x86_64-setup.exe')
    foreign.write_bytes(b'foreign product fixture')
    result=subprocess.run([sys.executable,str(script),'vtest',str(out)],capture_output=True)
    assert result.returncode!=0 and b'Unexpected release assets' in result.stderr

# Data may live next to Station sources, but never in publication inventory.
scanner=runpy.run_path(str(root/'scripts/verify-publication.py'))
with tempfile.TemporaryDirectory() as directory:
    fixture=Path(directory)
    (fixture/'README.md').write_text('Isolated public source')
    (fixture/'data').mkdir();(fixture/'data/operator.token').write_text('private fixture')
    with patch('subprocess.check_output',return_value=b'README.md\0'):
        count,issues=scanner['inspect'](fixture)
        assert count==1 and not issues
    with patch('subprocess.check_output',return_value=b'README.md\0data/operator.token\0'):
        count,issues=scanner['inspect'](fixture)
        assert any('private/generated publication path' in issue for issue in issues)
print('PASS: own product sources/workflow/downloads, complete release, rejected foreign assets and private publication paths.')
