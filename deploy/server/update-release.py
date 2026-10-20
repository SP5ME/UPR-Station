"""Public GitHub release lookup/download, run unprivileged outside the radio loop.

Only assets of SP5ME/UPR-Station are accepted. No downloaded script is executed.
Python is already a prerequisite of the native Station installer.
"""
import hashlib
from datetime import datetime
import json
import os
from pathlib import Path
import re
import sys
import time
import urllib.parse
import urllib.request

BASE = 'https://api.github.com/repos/SP5ME/UPR-Station/releases'
LIMIT = 128 * 1024 * 1024


class SafeRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, *args, **kwargs):
        result = super().redirect_request(request, *args, **kwargs)
        if result:
            url = urllib.parse.urlparse(result.full_url)
            if url.scheme != 'https' or url.hostname not in {
                    'api.github.com', 'github.com', 'release-assets.githubusercontent.com',
                    'objects.githubusercontent.com'}:
                raise ValueError('Untrusted asset redirect')
        return result


def fetch(url, accept='application/vnd.github+json'):
    request = urllib.request.Request(url, headers={
        'User-Agent': 'UPR-Station-updater', 'Accept': accept,
        'X-GitHub-Api-Version': '2022-11-28'})
    return urllib.request.build_opener(SafeRedirect()).open(request, timeout=15)


def read(url, accept='application/vnd.github+json', limit=8 * 1024 * 1024):
    with fetch(url, accept) as response:
        data = response.read(limit + 1)
    if len(data) > limit:
        raise ValueError('GitHub response exceeds allowed size')
    return data


def valid_tag(tag):
    return isinstance(tag, str) and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,100}', tag)


def select_release(releases, channel):
    if channel not in ('stable', 'test'):
        raise ValueError('Invalid update channel')
    available = [r for r in releases if not r.get('draft') and r.get('published_at')
                 and (channel == 'test' or not r.get('prerelease')) and valid_tag(r.get('tag_name'))]
    return max(available, key=lambda r: r['published_at']) if available else None


def asset(release, name):
    matches = [a for a in release['assets'] if a.get('name') == name]
    if len(matches) != 1 or type(matches[0].get('id')) is not int:
        raise ValueError('Missing or ambiguous release asset: ' + name)
    return matches[0]


def asset_url(item):
    # Ignore release-supplied URLs, including a substituted repository/host.
    return BASE + '/assets/' + str(item['id'])


def download(root, tag, architecture):
    if not valid_tag(tag):
        raise ValueError('Invalid release tag')
    release = json.loads(read(BASE + '/tags/' + tag))
    if release.get('draft') or release.get('tag_name') != tag:
        raise ValueError('Release is not published')
    name = f'UPR-Station-{tag}-alpine-{architecture}.zip'
    package = asset(release, name)
    sums = read(asset_url(asset(release, 'SHA256SUMS.txt')), 'application/octet-stream', 1024 * 1024).decode('utf-8')
    entries = [line.split() for line in sums.splitlines() if line.strip()]
    matches = [line[0] for line in entries if len(line) == 2 and line[1].lstrip('*') == name]
    if len(matches) != 1 or not re.fullmatch(r'[0-9a-f]{64}', matches[0]):
        raise ValueError('Missing or invalid published SHA-256')
    root = Path(root)
    temporary = root / 'download.part'
    size, digest, started = 0, hashlib.sha256(), time.monotonic()
    try:
        with fetch(asset_url(package), 'application/octet-stream') as source, temporary.open('xb') as target:
            while chunk := source.read(1024 * 1024):
                size += len(chunk)
                if size > LIMIT or time.monotonic() - started > 180:
                    raise ValueError('Download size/time limit exceeded')
                target.write(chunk)
                digest.update(chunk)
        if digest.hexdigest() != matches[0] or size != package.get('size'):
            raise ValueError('Published ZIP size or SHA-256 mismatch')
        os.replace(temporary, root / 'incoming.zip')
    finally:
        temporary.unlink(missing_ok=True)


def main():
    action, architecture, *args = sys.argv[1:]
    if architecture not in ('x86_64', 'aarch64', 'armv7'):
        raise ValueError('Unsupported architecture')
    if action == 'check':
        release = select_release(json.loads(read(BASE + '?per_page=100')), args[0])
        if release is None:
            print(json.dumps({'available': False, 'channel': args[0]}))
        else:
            tag = release['tag_name']
            item = asset(release, f'UPR-Station-{tag}-alpine-{architecture}.zip')
            print(json.dumps({'available': True, 'tag': tag, 'channel': args[0],
                              'prerelease': bool(release.get('prerelease')), 'size': item['size'],
                              'publishedAt': int(datetime.fromisoformat(release['published_at'].replace('Z', '+00:00')).timestamp()),
                              'notes': str(release.get('body') or '')[:16000]}))
    elif action == 'download':
        download(args[0], args[1], architecture)
    else:
        raise ValueError('Invalid update operation')


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        # Never emit request URLs, credentials or response bodies.
        print('GitHub update failed: ' + type(error).__name__ + '. Check connectivity, release assets and API limits.', file=sys.stderr)
        sys.exit(1)
