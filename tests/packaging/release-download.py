"""Offline downloader/checksum/redirect tests; no external requests."""
import hashlib
import io
import json
import os
from pathlib import Path
import runpy
import tempfile
from unittest.mock import patch
import urllib.request

script=Path(__file__).resolve().parents[2]/'scripts/download-release.py'
name='UPR-Station-vtest-linux-x86_64.tar.gz'
payload=b'isolated-package-test'
digest=hashlib.sha256(payload).hexdigest()
release={'tag_name':'vtest','draft':False,'assets':[{'name':name,'url':'https://api.github.com/package'},{'name':'SHA256SUMS.txt','url':'https://api.github.com/sums'}]}
class Opener:
    def __init__(self,bad=False): self.bad=bad
    def open(self,request,timeout):
        if '/releases?per_page=' in request.full_url:
            return io.BytesIO(json.dumps([
                {'tag_name':'draft','draft':True,'published_at':'2099-01-01'},
                {'tag_name':'older','draft':False,'published_at':'2026-01-01'},
                {'tag_name':'vtest','draft':False,'prerelease':True,'published_at':'2026-10-15'},
            ]).encode())
        if '/releases/tags/' in request.full_url: return io.BytesIO(json.dumps(release).encode())
        if request.full_url.endswith('/sums'): return io.BytesIO(f'{"0"*64 if self.bad else digest}  {name}\n'.encode())
        return io.BytesIO(payload)
with tempfile.TemporaryDirectory() as directory:
    with patch('sys.argv',[str(script),directory,'vtest','x86_64','native']),patch('urllib.request.build_opener',return_value=Opener()),patch.dict(os.environ,{'GITHUB_TOKEN':'isolated-test-token'}):
        module=runpy.run_path(str(script))
    assert (Path(directory)/name).read_bytes()==payload
    redirect=module['SafeRedirect']()
    request=urllib.request.Request('https://api.github.com/a',headers={'Authorization':'Bearer isolated-test-token'})
    redirected=redirect.redirect_request(request,None,302,'Found',{},'https://release-assets.githubusercontent.com/a')
    assert not redirected.has_header('Authorization')
for mode,asset in [('docker','UPR-Station-vtest-docker-x86_64.tar.gz')]:
    name=asset
    release['assets'][0]['name']=name
    with tempfile.TemporaryDirectory() as directory:
        with patch('sys.argv',[str(script),directory,'latest','x86_64',mode]),patch('urllib.request.build_opener',return_value=Opener()):
            runpy.run_path(str(script))
        assert (Path(directory)/name).read_bytes()==payload
        assert (Path(directory)/'release-tag.txt').read_text()=='vtest'
name='UPR-Station-vtest-linux-x86_64.tar.gz'
release['assets'][0]['name']=name
with tempfile.TemporaryDirectory() as directory:
    with patch('sys.argv',[str(script),directory,'vtest','x86_64','native']),patch('urllib.request.build_opener',return_value=Opener(bad=True)):
        try: runpy.run_path(str(script))
        except SystemExit as error: assert error.code==1
        else: raise AssertionError('Bad checksum was accepted')
    assert (Path(directory)/name).read_bytes()==payload, 'Checksum failure must be tested after downloading the selected asset'
print('PASS: pinned/latest pre-release selection, draft exclusion, UPR Station assets, mandatory SHA-256 and no token forwarding to another host.')
