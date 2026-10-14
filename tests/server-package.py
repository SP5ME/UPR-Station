"""Check package contracts with a synthetic ELF header; not an executable test."""
import hashlib
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import zipfile
root=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as directory:
    temp=Path(directory)
    binary=temp/'binary'
    data=bytearray(100_001);data[:4]=b'\x7fELF';data[18:20]=(62).to_bytes(2,'little');binary.write_bytes(data)
    licenses=temp/'licenses';licenses.mkdir();(licenses/'THIRD-PARTY.md').write_text('Isolated fixture')
    subprocess.run([sys.executable,str(root/'scripts/package-server.py'),'--binary',str(binary),'--arch','x86_64','--tag','vtest','--licenses',str(licenses),'--output',str(temp/'out')],check=True)
    with zipfile.ZipFile(temp/'out/UPR-Station-vtest-alpine-x86_64.zip') as archive:
        names=archive.namelist()
        for required in ['Cargo.toml','ultimatepr-station','PACKAGE-MANIFEST.txt','licenses/MDI-LICENSE.txt','deploy/alpine/ultimatepr-update','deploy/server/ultimatepr-station.service']:
            assert required in names,required
        assert archive.getinfo('ultimatepr-station').external_attr>>16 & 0o111
        manifest=archive.read('PACKAGE-MANIFEST.txt').decode()
        assert 'binary_sha256='+hashlib.sha256(data).hexdigest() in manifest
        assert not any('proxmox' in name.lower() for name in names)
        assert 'src/main.rs' not in names
    with tarfile.open(temp/'out/UPR-Station-vtest-linux-x86_64.tar.gz') as archive:
        assert archive.getmember('ultimatepr-station').mode==0o755
print('PASS: binary package manifest, architecture, executable permissions, licenses and no Proxmox scripts.')
