"""Offline updater security and release selection tests. No external requests."""
import hashlib
import io
import json
from pathlib import Path
import runpy
import stat
import tempfile
from unittest.mock import patch
import urllib.request
import zipfile

root = Path(__file__).resolve().parents[2]
validator = runpy.run_path(str(root/'deploy/server/update-package.py'))['validate']
catalog = runpy.run_path(str(root/'deploy/server/update-release.py'))
select = catalog['select_release']
releases = [
    {'tag_name': 'vstable', 'published_at': '2026-01-01', 'prerelease': False},
    {'tag_name': 'vtest', 'published_at': '2026-10-01', 'prerelease': True},
    {'tag_name': 'draft', 'published_at': '2099-01-01', 'draft': True},
]
assert select(releases, 'stable')['tag_name'] == 'vstable'
assert select(releases, 'test')['tag_name'] == 'vtest'
assert select(releases[1:2], 'stable') is None
assert not catalog['valid_tag']('../vtest')
request = urllib.request.Request('https://api.github.com/a')
try:
    catalog['SafeRedirect']().redirect_request(request, None, 302, 'Found', {}, 'http://example.com/a')
except ValueError:
    pass
else:
    raise AssertionError('Unsafe redirect accepted')

binary = bytearray(100_001)
binary[:6] = b'\x7fELF\x02\x01'
binary[18:20] = (62).to_bytes(2, 'little')
manifest = ('name=UPR-Station-vtest-x86_64\nversion=0.1.3-prealpha.1\n'
            'variant=linux-headless-server-alpine-musl\nsource=binary\n'
            'architecture=x86_64\nbinary_sha256=' + hashlib.sha256(binary).hexdigest() + '\n')
files = {'ultimatepr-station': bytes(binary), 'PACKAGE-MANIFEST.txt': manifest.encode(), 'Cargo.toml': b'package fixture',
         'deploy/alpine/ultimatepr-update': b'helper fixture', 'deploy/server/ultimatepr-update.path': b'path fixture',
         'deploy/server/ultimatepr-update.service': b'service fixture'}


def make_zip(path, contents, extra=None):
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as archive:
        for name, data in contents.items():
            item = zipfile.ZipInfo(name)
            item.create_system = 3
            item.external_attr = (stat.S_IFREG | (0o755 if name == 'ultimatepr-station' else 0o644)) << 16
            archive.writestr(item, data)
        archive.writestr('SHA256SUMS.txt', ''.join(hashlib.sha256(v).hexdigest() + '  ' + k + '\n' for k, v in contents.items()))
        if extra:
            archive.writestr(*extra)


with tempfile.TemporaryDirectory() as directory:
    temp = Path(directory)
    package = temp/'update.zip'
    make_zip(package, files)
    assert validator(package, 'x86_64')['version'] == '0.1.3-prealpha.1'
    validator(package, 'x86_64', temp/'extracted')
    assert (temp/'extracted/ultimatepr-station').read_bytes() == binary
    for architecture in ['aarch64', 'armv7']:
        try:
            validator(package, architecture)
        except ValueError:
            pass
        else:
            raise AssertionError('Foreign architecture accepted')
    malicious = zipfile.ZipInfo('link')
    malicious.create_system = 3
    malicious.external_attr = (stat.S_IFLNK | 0o777) << 16
    for name in ['../escape', '/absolute', 'deploy/../../escape', 'C:/escape', 'deploy\\escape', 'ultimatepr-station', 'a/./b']:
        make_zip(package, files, (name, b'unsafe'))
        try:
            validator(package, 'x86_64')
        except ValueError:
            pass
        else:
            raise AssertionError('Unsafe/duplicate entry accepted: ' + name)
    make_zip(package, files, (malicious, b'/etc/shadow'))
    try:
        validator(package, 'x86_64')
    except ValueError:
        pass
    else:
        raise AssertionError('Symlink accepted')
    corrupted = dict(files)
    corrupted['PACKAGE-MANIFEST.txt'] = manifest.replace('source=binary', 'source=source').encode()
    make_zip(package, corrupted)
    try:
        validator(package, 'x86_64')
    except ValueError:
        pass
    else:
        raise AssertionError('Source package accepted')
    # Corruption of a member while retaining the previous checksum list.
    make_zip(package, files)
    with zipfile.ZipFile(package) as source:
        entries = {n: source.read(n) for n in source.namelist()}
    entries['Cargo.toml'] = b'corrupted'
    with zipfile.ZipFile(package, 'w') as archive:
        for name, data in entries.items():
            archive.writestr(name, data)
    try:
        validator(package, 'x86_64')
    except ValueError:
        pass
    else:
        raise AssertionError('Corrupted member accepted')

    payload = b'isolated download fixture'
    name = 'UPR-Station-vtest-alpine-x86_64.zip'
    release = {'tag_name': 'vtest', 'draft': False, 'assets': [
        {'name': name, 'id': 1, 'size': len(payload), 'url': 'http://untrusted.invalid'},
        {'name': 'SHA256SUMS.txt', 'id': 2},
    ]}
    for bad in [False, True]:
        def read(url, *args):
            if '/tags/' in url:
                return json.dumps(release).encode()
            assert url.endswith('/assets/2')
            return (('0' * 64 if bad else hashlib.sha256(payload).hexdigest()) + '  ' + name + '\n').encode()

        def fetch(url, *args):
            assert url.endswith('/assets/1')
            return io.BytesIO(payload)

        download = catalog['download']
        with patch.dict(download.__globals__, {'read': read, 'fetch': fetch}):
            try:
                download(temp, 'vtest', 'x86_64')
            except ValueError:
                assert bad
            else:
                assert not bad
                assert (temp/'incoming.zip').read_bytes() == payload
        assert not (temp/'download.part').exists()

print('PASS: stable/test channels, fixed GitHub assets, mandatory SHA-256, ZIP paths/links/duplicates, architecture, binary manifest and corrupted member rejection.')
