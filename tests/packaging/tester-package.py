"""Verify an actual local tester handoff, without executing a Linux binary."""
import argparse
import hashlib
from pathlib import Path
import runpy
import struct
import zipfile

root = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument('folder')
parser.add_argument('--binary', required=True)
parser.add_argument('--tag', required=True)
args = parser.parse_args()
folder = Path(args.folder)
binary = Path(args.binary).read_bytes()
assert binary[:6] == b'\x7fELF\x02\x01'
assert struct.unpack_from('<H', binary, 18)[0] == 62
offset = struct.unpack_from('<Q', binary, 32)[0]
size, count = struct.unpack_from('<HH', binary, 54)
for index in range(count):
    entry = offset + index * size
    kind = struct.unpack_from('<I', binary, entry)[0]
    assert kind != 3, 'PT_INTERP: binary is not static'
    if kind == 2:
        dynamic_offset = struct.unpack_from('<Q', binary, entry + 8)[0]
        dynamic_size = struct.unpack_from('<Q', binary, entry + 32)[0]
        assert all(struct.unpack_from('<q', binary, p)[0] != 1 for p in range(dynamic_offset, dynamic_offset + dynamic_size, 16)), 'DT_NEEDED dependency'
assert args.tag.encode() in binary
assert (root/'web/app.js').read_bytes() in binary, 'Binary does not embed the current panel'
checksums = folder/'SHA256SUMS.txt'
names = set()
for line in checksums.read_text().splitlines():
    digest, name = line.split('  ', 1)
    assert hashlib.sha256((folder/name).read_bytes()).hexdigest() == digest, name
    names.add(name)
assert names == {p.relative_to(folder).as_posix() for p in folder.rglob('*') if p.is_file()} - {'SHA256SUMS.txt'}
archive = folder.with_suffix('.zip')
with zipfile.ZipFile(archive) as outer:
    assert set(outer.namelist()) == {folder.name+'/'+n for n in names | {'SHA256SUMS.txt'}}
    assert outer.testzip() is None
    for name in outer.namelist():
        relative = name[len(folder.name)+1:]
        assert outer.read(name) == (folder/relative).read_bytes()
inner = list((folder/'alpine').glob('*.zip'))
assert len(inner) == 1
validate = runpy.run_path(str(root/'deploy/server/update-package.py'))['validate']
validate(inner[0], 'x86_64', expected_tag=args.tag)
with zipfile.ZipFile(inner[0]) as package:
    assert package.read('ultimatepr-station') == binary
    names = package.namelist()
    assert not any(n.startswith(('src/', 'tests/', 'data/', 'target/', 'desktop/')) or 'proxmox' in n.lower() for n in names)
    assert not any(n.endswith(('.token', '.password')) for n in names)
    assert not any('update-package.py' in n or 'update-release.py' in n for n in names)
    helper = package.read('deploy/alpine/ultimatepr-update').decode()
    assert 'Unexpanded helper:' not in helper
    embedded = helper.split("<<'UPR_VALIDATE'\n", 1)[1].split('\nUPR_VALIDATE', 1)[0]
    assert embedded.strip() == (root/'deploy/server/update-package.py').read_text().strip()
print('PASS: actual static x86_64 ELF, exact current panel, build identity, tester/inner ZIP checksums, binary identity, complete embedded validator and no user/test/desktop data.')
print('Outer ZIP SHA-256: ' + hashlib.sha256(archive.read_bytes()).hexdigest())
