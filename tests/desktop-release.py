"""Check both Linux formats and the complete release contract with isolated fixtures."""
import hashlib
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

root=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as directory:
    stage=Path(directory)
    (stage/'scripts').mkdir()
    for name in ['collect-desktop-release.py','finalize-release.py']:
        shutil.copy2(root/'scripts'/name,stage/'scripts'/name)
    for name in ['deb/test_amd64.deb','appimage/test.AppImage']:
        path=stage/'desktop/target/release/bundle'/name
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_bytes(b'isolated packaging fixture')
    out=stage/'out'
    subprocess.run([sys.executable,str(stage/'scripts/collect-desktop-release.py'),'Linux','vtest',str(out)],check=True)
    assert (out/'UPR-Terminal-vtest-linux-x86_64.deb').is_file()
    assert (out/'UPR-Terminal-vtest-linux-x86_64.AppImage').is_file()
    names=['UPR-Terminal-vtest-windows-x86_64-setup.exe','install-server.sh']
    names += [f'UPR-Station-vtest-{kind}-{arch}.{extension}' for arch in ['x86_64','aarch64','armv7'] for kind,extension in [('linux','tar.gz'),('alpine','zip'),('docker','tar.gz')]]
    for name in names: (out/name).write_bytes(b'isolated release fixture')
    subprocess.run([sys.executable,str(stage/'scripts/finalize-release.py'),'vtest',str(out)],check=True)
    sums={line.split(None,1)[1]:line.split(None,1)[0] for line in (out/'SHA256SUMS.txt').read_text().splitlines()}
    assert len(sums)==14
    for name,digest in sums.items(): assert hashlib.sha256((out/name).read_bytes()).hexdigest()==digest
    (out/'UPR-Terminal-vtest-linux-x86_64.AppImage').unlink()
    result=subprocess.run([sys.executable,str(stage/'scripts/finalize-release.py'),'vtest',str(out)],capture_output=True)
    assert result.returncode!=0 and b'AppImage' in result.stderr
print('PASS: Linux DEB + AppImage collection, checksums and missing-format rejection.')
