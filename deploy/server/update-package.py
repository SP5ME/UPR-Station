"""Bounded binary-only ZIP validation. Embedded in the Station executable.

No archive-supplied programs are run. Extraction is into a new private directory.
The published outer SHA-256 establishes download integrity; internal checksums
also protect manual ZIP uploads from corruption, not from an untrusted publisher.
"""
import hashlib
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import struct
import sys
import zipfile

MAX_PACKAGE = 128 * 1024 * 1024
MAX_EXPANDED = 256 * 1024 * 1024


def validate(package, architecture, destination=None, expected_tag=None):
    flags = os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_BINARY', 0)
    with os.fdopen(os.open(package, flags), 'rb') as source:
        info = os.fstat(source.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_PACKAGE:
            raise ValueError('Invalid package file or package exceeds 128 MiB')
        with zipfile.ZipFile(source) as archive:
            members = archive.infolist()
            if len(members) > 4096:
                raise ValueError('Too many ZIP entries')
            names = set()
            total = 0
            for item in members:
                path = PurePosixPath(item.filename)
                mode = item.external_attr >> 16
                if (not item.filename or path.is_absolute() or '..' in path.parts
                        or '\\' in item.filename or ':' in item.filename
                        or str(path) != item.filename or item.filename in names
                        or item.is_dir() or item.flag_bits & 1
                        or stat.S_IFMT(mode) not in (0, stat.S_IFREG)):
                    raise ValueError('Unsafe or duplicate ZIP entry')
                names.add(item.filename)
                total += item.file_size
                if total > MAX_EXPANDED:
                    raise ValueError('Expanded package exceeds 256 MiB')
            required = {'ultimatepr-station', 'PACKAGE-MANIFEST.txt', 'SHA256SUMS.txt',
                        'deploy/alpine/ultimatepr-update', 'deploy/server/ultimatepr-update.path',
                        'deploy/server/ultimatepr-update.service'}
            if not required <= names or any(n.startswith('src/') for n in names):
                raise ValueError('A prebuilt UPR Station binary package is required')
            if archive.getinfo('PACKAGE-MANIFEST.txt').file_size > 16384:
                raise ValueError('Manifest too large')
            manifest = {}
            for line in archive.read('PACKAGE-MANIFEST.txt').decode('utf-8').splitlines():
                key, separator, value = line.partition('=')
                if not separator or key in manifest:
                    raise ValueError('Invalid manifest')
                manifest[key] = value
            if (not manifest.get('name', '').startswith('UPR-Station-')
                    or manifest.get('variant') != 'linux-headless-server-alpine-musl'
                    or manifest.get('source') != 'binary'
                    or manifest.get('architecture') != architecture
                    or not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,100}', manifest.get('version', ''))):
                raise ValueError('Wrong product, architecture or manifest version')
            if expected_tag is not None and manifest.get('build_tag') != expected_tag:
                raise ValueError('Manifest does not match the selected GitHub build')
            if archive.getinfo('SHA256SUMS.txt').file_size > 1024 * 1024:
                raise ValueError('Checksum list too large')
            checksums = {}
            for line in archive.read('SHA256SUMS.txt').decode('utf-8').splitlines():
                digest, separator, name = line.partition('  ')
                if not separator or not re.fullmatch(r'[0-9a-f]{64}', digest) or name in checksums:
                    raise ValueError('Invalid checksum list')
                checksums[name] = digest
            if set(checksums) != names - {'SHA256SUMS.txt'}:
                raise ValueError('Incomplete checksum list')
            for name, expected in checksums.items():
                digest = hashlib.sha256()
                with archive.open(name) as stream:
                    while chunk := stream.read(1024 * 1024):
                        digest.update(chunk)
                if digest.hexdigest() != expected:
                    raise ValueError('Package checksum mismatch')
            if checksums['ultimatepr-station'] != manifest.get('binary_sha256'):
                raise ValueError('Binary checksum mismatch')
            if not archive.getinfo('ultimatepr-station').external_attr >> 16 & 0o111:
                raise ValueError('Binary is not executable')
            with archive.open('ultimatepr-station') as executable:
                header = executable.read(64)
            elf_class, machine = {'x86_64': (2, 62), 'aarch64': (2, 183), 'armv7': (1, 40)}[architecture]
            if (len(header) < 64 or header[:4] != b'\x7fELF' or header[4] != elf_class
                    or header[5] != 1 or struct.unpack_from('<H', header, 18)[0] != machine):
                raise ValueError('ELF architecture mismatch')
            if destination is not None:
                root = Path(destination)
                root.mkdir(mode=0o700)  # must not exist, including symlinks
                for item in members:
                    target = root / item.filename
                    target.parent.mkdir(parents=True, exist_ok=True)
                    with archive.open(item) as src, target.open('xb') as dst:
                        shutil.copyfileobj(src, dst, 1024 * 1024)
                    target.chmod(0o755 if item.external_attr >> 16 & 0o111 else 0o644)
            return manifest


if __name__ == '__main__':
    try:
        package, architecture, *target = sys.argv[1:]
        expected_tag = None
        if target and target[0].startswith('--tag='):
            expected_tag = target.pop(0)[6:]
        manifest = validate(package, architecture, target[0] if target else None, expected_tag)
        print(manifest['version'])
    except (OSError, ValueError, KeyError, zipfile.BadZipFile, RuntimeError) as error:
        print('ZIP rejected: ' + str(error), file=sys.stderr)
        sys.exit(1)
