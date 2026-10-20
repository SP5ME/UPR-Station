"""Download selected GitHub asset and verify its published SHA-256; no tokens in logs."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

root, version, arch, mode = sys.argv[1:]
if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,100}', version):
    raise SystemExit('Invalid release tag')
if arch not in ['x86_64', 'aarch64', 'armv7'] or mode not in ['native', 'docker']:
    raise SystemExit('Invalid platform')
class SafeRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, *args, **kwargs):
        redirected = super().redirect_request(request, *args, **kwargs)
        if redirected and urllib.parse.urlparse(redirected.full_url).netloc != urllib.parse.urlparse(request.full_url).netloc:
            redirected.remove_header('Authorization')
        return redirected
opener = urllib.request.build_opener(SafeRedirect())
headers = {'User-Agent': 'UPR-Station-installer', 'Accept': 'application/vnd.github+json'}
token = os.environ.get('GITHUB_TOKEN', '') or os.environ.get('GH_TOKEN', '')
if token:
    headers['Authorization'] = 'Bearer ' + token
def fetch(url, accept=None, limit=2_000_000):
    request = urllib.request.Request(url, headers={**headers, **({'Accept': accept} if accept else {})})
    with opener.open(request, timeout=60) as response:
        data = response.read(limit+1)
    if len(data) > limit:
        raise RuntimeError('Response exceeds allowed size')
    return data
try:
    base = 'https://api.github.com/repos/SP5ME/UPR-Station/releases'
    if version == 'latest':
        releases = json.loads(fetch(base+'?per_page=100'))
        published = [item for item in releases if not item['draft'] and item.get('published_at')]
        if not published:
            raise RuntimeError('No published release')
        version = max(published, key=lambda item: item['published_at'])['tag_name']
        if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,100}', version):
            raise RuntimeError('Invalid published release tag')
    release = json.loads(fetch(base+'/tags/' + version))
    if release['tag_name'] != version or release['draft']:
        raise RuntimeError('Release is not published')
    extension = 'docker' if mode == 'docker' else 'linux'
    name = f'UPR-Station-{version}-{extension}-{arch}.tar.gz'
    assets = {a['name']: a for a in release['assets']}
    sums = fetch(assets['SHA256SUMS.txt']['url'], 'application/octet-stream').decode()
    expected = dict((line.split(None, 1)[1].lstrip('*'), line.split(None, 1)[0]) for line in sums.splitlines() if line.strip())[name]
    if not re.fullmatch(r'[0-9a-fA-F]{64}', expected):
        raise RuntimeError('Invalid checksum entry')
    out = Path(root)/name
    request = urllib.request.Request(assets[name]['url'], headers={**headers, 'Accept': 'application/octet-stream'})
    digest = hashlib.sha256()
    size = 0
    with opener.open(request, timeout=60) as response, out.open('xb') as destination:
        while chunk := response.read(1024*1024):
            size += len(chunk)
            if size > 768*1024*1024:
                raise RuntimeError('Asset exceeds allowed size')
            digest.update(chunk)
            destination.write(chunk)
    if digest.hexdigest().lower() != expected.lower():
        raise RuntimeError('SHA-256 mismatch; installation stopped')
    (Path(root)/'release-tag.txt').write_text(version, encoding='utf-8')
    print(out)
except (urllib.error.HTTPError, urllib.error.URLError, KeyError, RuntimeError) as error:
    if isinstance(error, urllib.error.HTTPError):
        print(f'GitHub HTTP {error.code}. Private releases require GITHUB_TOKEN with read access.', file=sys.stderr)
    else:
        print('Release asset unavailable or checksum verification failed; installation stopped.', file=sys.stderr)
    sys.exit(1)
