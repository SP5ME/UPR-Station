"""Build native Linux/Alpine packages from one verified static musl executable."""
import argparse
import hashlib
from pathlib import Path
import re
import shutil
import stat
import tarfile
import tempfile
import zipfile

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--binary', required=True)
parser.add_argument('--arch', choices=['x86_64', 'aarch64', 'armv7'], required=True)
parser.add_argument('--tag', required=True)
parser.add_argument('--licenses', required=True)
parser.add_argument('--output', default='target/release-assets')
args = parser.parse_args()
if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,100}', args.tag):
    raise SystemExit('Invalid release tag')
binary = Path(args.binary).resolve()
if not binary.is_file() or binary.stat().st_size < 100_000:
    raise SystemExit('Missing/invalid binary')
header = binary.read_bytes()[:20]
machine = int.from_bytes(header[18:20], 'little')
if header[:4] != b'\x7fELF' or machine != {'x86_64': 62, 'aarch64': 183, 'armv7': 40}[args.arch]:
    raise SystemExit('ELF architecture mismatch')
licenses = Path(args.licenses).resolve()
if not (licenses / 'THIRD-PARTY.md').is_file() or (licenses / 'MISSING-LICENSE-TEXTS.txt').exists():
    raise SystemExit('Incomplete dependency licenses')
out = Path(args.output).resolve()
out.mkdir(parents=True, exist_ok=True)
version = re.search(r'^version\s*=\s*"([^"]+)"', (root/'Cargo.toml').read_text(), re.M)[1]
with tempfile.TemporaryDirectory(prefix='upr-server-package-') as temporary:
    stage = Path(temporary)
    shutil.copy2(binary, stage/'ultimatepr-station')
    for name in ['Cargo.toml', 'LICENSE', 'TRADEMARKS.md']:
        shutil.copy2(root/name, stage/name)
    # The bootstrap template is not executable by itself; only publish its generated release asset.
    shutil.copytree(root/'deploy', stage/'deploy', ignore=shutil.ignore_patterns('install'))
    shutil.copytree(licenses, stage/'licenses/crates')
    for name in ['Apache-2.0.txt', 'MDI-LICENSE.txt']:
        shutil.copy2(root/'licenses'/name, stage/'licenses'/name)
    shutil.copy2(root/'docs/SERVER-INSTALL.md', stage/'README.md')
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    (stage/'PACKAGE-MANIFEST.txt').write_text(
        f'name=UPR-Station-{args.tag}-{args.arch}\nversion={version}\n'
        'variant=linux-headless-server-alpine-musl\nsource=binary\n'
        f'created={args.tag}\narchitecture={args.arch}\nbinary_sha256={digest}\n')
    files = sorted(p for p in stage.rglob('*') if p.is_file())
    def executable(path):
        return path.name == 'ultimatepr-station' or path.suffix == '.sh' or path.name in ['ultimatepr-update', 'ultimatepr-station-run']
    (stage/'SHA256SUMS.txt').write_text(''.join(
        f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.relative_to(stage).as_posix()}\n' for p in files))
    files.append(stage/'SHA256SUMS.txt')
    tar_path = out/f'UPR-Station-{args.tag}-linux-{args.arch}.tar.gz'
    zip_path = out/f'UPR-Station-{args.tag}-alpine-{args.arch}.zip'
    if tar_path.exists() or zip_path.exists():
        raise SystemExit('Refusing to overwrite an existing package')
    with tarfile.open(tar_path, 'w:gz') as archive:
        for p in files:
            info = archive.gettarinfo(str(p), p.relative_to(stage).as_posix())
            info.mode = 0o755 if executable(p) else 0o644
            info.uid = info.gid = 0
            info.uname = info.gname = 'root'
            with p.open('rb') as stream:
                archive.addfile(info, stream)
    with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as archive:
        for p in files:
            info = zipfile.ZipInfo(p.relative_to(stage).as_posix())
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | (0o755 if executable(p) else 0o644)) << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, p.read_bytes())
print(tar_path)
print(zip_path)
