"""Independent HTTP + wire-level KISS/AX.25/YAPP acceptance test. No radio, stdlib only."""
import contextlib
import http.client
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import time
import urllib.parse


def address(call, command, last):
    name, _, ssid = call.partition('-')
    return bytes(c << 1 for c in name.ljust(6).encode()) + bytes([0x60 | (int(ssid or 0) << 1) | (0x80 if command else 0) | last])


class Radio:
    def __init__(self, sock, source='REMOTE', destination='TEST-1'):
        self.sock, self.source, self.destination = sock, source, destination
        self.buffer = bytearray()
        self.escaped = False
        self.ns = self.nr = 0
        self.data = bytearray()

    def send(self, control, data=b'', command=False):
        raw = address(self.destination, command, 0) + address(self.source, not command, 1) + bytes([control])
        if not control & 1:
            raw += b'\xf0' + data
        self.sock.sendall(b'\xc0\x00' + raw.replace(b'\xdb', b'\xdb\xdd').replace(b'\xc0', b'\xdb\xdc') + b'\xc0')

    def frame(self, timeout=5):
        self.sock.settimeout(timeout)
        while True:
            byte = self.sock.recv(1)
            assert byte, 'TNC socket closed unexpectedly'
            byte = byte[0]
            if byte == 0xc0:
                if self.buffer:
                    wire = bytes(self.buffer)
                    self.buffer.clear()
                    self.escaped = False
                    assert wire[0] == 0
                    return wire[1:]
            elif self.escaped:
                self.buffer.append({0xdc: 0xc0, 0xdd: 0xdb}[byte])
                self.escaped = False
            elif byte == 0xdb:
                self.escaped = True
            else:
                self.buffer.append(byte)

    def pump(self):
        raw = self.frame()
        c = raw[14]
        if not c & 1:
            assert (c >> 1) & 7 == self.nr, (c, self.nr)
            self.nr = (self.nr + 1) % 8
            self.data.extend(raw[16:])
            self.send(1 | self.nr << 5)
        return raw

    def info(self, data):
        self.send(self.ns << 1 | self.nr << 5, data, True)
        self.ns = (self.ns + 1) % 8
        while True:
            raw = self.pump()
            c = raw[14]
            if (not c & 1 or c & 3 == 1) and c >> 5 == self.ns:
                return

    def expect(self, data):
        deadline = time.monotonic() + 6
        while data not in self.data:
            assert time.monotonic() < deadline, (data, self.data)
            self.pump()
        at = self.data.index(data) + len(data)
        del self.data[:at]


def free_port():
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0))
        return s.getsockname()[1]


def main():
    binary = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/debug/ultimatepr-station.exe').resolve()
    token = 'isolated-e2e-operator-token-1234'
    port = free_port()
    proc = None
    def api(path, fields=None, status=200, raw=None, extra=None, auth=True, method='POST'):
        conn = http.client.HTTPConnection('127.0.0.1', port, timeout=10)
        headers = {'Content-Type': 'application/x-www-form-urlencoded', 'X-Station-Request': '1'}
        if auth:
            headers['Authorization'] = 'Bearer ' + token
        headers.update(extra or {})
        body = raw if raw is not None else urllib.parse.urlencode(fields or {}).encode()
        conn.request(method, '/api/' + path, None if method == 'GET' else body, headers)
        r = conn.getresponse()
        result = r.read()
        conn.close()
        assert r.status == status, (path, r.status, result)
        return json.loads(result) if 'application/json' in r.getheader('Content-Type', '') else result
    def snapshot():
        return api('state', method='GET')
    def web(path):
        conn = http.client.HTTPConnection('127.0.0.1', port, timeout=10)
        conn.request('GET', path)
        response = conn.getresponse()
        body = response.read()
        conn.close()
        assert response.status == 200, (path, response.status, body)
        return body
    def wait(predicate, timeout=8):
        deadline = time.monotonic() + timeout
        s = None
        while time.monotonic() < deadline:
            try:
                s = snapshot()
            except OSError:
                time.sleep(.1)
                continue
            if predicate(s):
                return s
            time.sleep(.1)
        raise AssertionError('State condition timed out: ' + str(s))
    with contextlib.nullcontext(Path('target') / f'e2e-{os.getpid()}-{time.time_ns()}') as folder, socket.socket() as listener, socket.socket() as second_listener:
        folder = Path(folder).resolve()
        folder.mkdir(parents=True)
        listener.bind(('127.0.0.1', 0)); listener.listen(); listener.settimeout(10)
        second_listener.bind(('127.0.0.1',0));second_listener.listen();second_listener.settimeout(10)
        cfg = ('callsign=TEST-1\nmyname=E2E\nax25_autonegotiation=false\nwelcome_enabled=false\ngoodbye_enabled=false\n'
               'beacon_enabled=false\ndigipeat_enabled=false\nyapp_enabled=true\nyapp_auto_receive=false\n'
               'mailbox_enabled=true\nmailbox_callsign=TEST-8\npaclen=128\n'
               'notification_outgoing_connection_in_app=false\nnotification_outgoing_connection_native=false\nnotification_outgoing_connection_sound=2\n'
               f'interface_VHF=127.0.0.1:{listener.getsockname()[1]},0,0,true,Primary\n'
               f'interface_UHF=127.0.0.1:{second_listener.getsockname()[1]},0,1,true,Secondary\n')
        config = folder/'station.conf'; config.write_text(cfg, encoding='utf-8', newline='\n')
        # Authentication is optional in current builds; this test explicitly
        # enables it because the HTTP checks below require protected endpoints.
        (folder/'web.conf').write_text('auth_enabled=true\n', encoding='utf-8')
        log = open(folder/'server.log', 'w', encoding='utf-8')
        try:
            proc = subprocess.Popen([str(binary), '--bind', f'127.0.0.1:{port}', '--config', str(config), '--web-config', str(folder/'web.conf')], env={**os.environ,'STATION_TOKEN':token}, stdout=log, stderr=log, creationflags=0x08000000 if os.name=='nt' else 0)
            sock, _ = listener.accept()
            second_sock,_=second_listener.accept()
            with sock,second_sock:
                assert b'UltimatePR' in web('/')
                for asset in ['/', '/app.js', '/app.css', '/lang/en.json', '/lang/pl.json']:
                    content = web(asset).lower()
                    assert b'wl2k' not in content and b'winlink' not in content, asset
                assert not any(key.startswith(('wl2k', 'p2p')) for key in snapshot())
                assert 'winlink' not in json.dumps(api('config/details')).lower()
                for action in ['wl2k/sync', 'wl2k/create', 'wl2k/password', 'wl2k/p2p/start']:
                    api(action, status=400)
                english = json.loads(web('/lang/en.json'))
                polish = json.loads(web('/lang/pl.json'))
                assert english['settings.title'] == 'Station settings'
                assert polish['settings.title'] == 'Ustawienia stacji'
                radio = Radio(sock)
                wait(lambda s:s['interfaces'][0]['online'])
                api('state', auth=False, status=401, method='GET')
                api('send', {'session':0,'text':'no'}, status=403, extra={'Origin':'https://untrusted.invalid'})
                api('send', {'session':99,'text':'bad'}, status=400)
                api('yapp/send?session=0&name=..%2Fescape', raw=b'no', status=400)
                assert api('config') == cfg.encode()
                api('config/save', {'text':cfg+'unknown_key=1\n'}, status=400)
                assert config.read_text(encoding='utf-8') == cfg
                web = api('web/details')
                assert 'bind' in web and 'tokenFile' in web
                api('web/save', {'bind': f'127.0.0.1:{port}', 'token_file': 'operator.token'})
                assert (folder/'web.conf').is_file()
                api('web/save', {'bind': f'127.0.0.1:{port}', 'token_file': ''})
                assert (folder/'web.bak').is_file()
                # Explicit routing through a second independent TNC.
                wait(lambda s:all(i['online'] for i in s['interfaces']))
                second=Radio(second_sock,source='SECOND')
                api('connect',{'session':3,'interface':1,'call':'SECOND'})
                assert second.frame()[14]==0x3f;second.send(0x73)
                connected = wait(lambda s:s['sessions'][3]['state']=='Connected')
                assert not any(n['kind']=='outgoing_connection' for n in connected['notifications'])
                assert connected['notificationSounds'][-1]['count']==2
                api('send',{'session':3,'text':'via UHF'});second.expect(b'via UHF\r')
                api('disconnect',{'session':3});assert second.frame()[14]==0x53;second.send(0x73)
                wait(lambda s:s['sessions'][3]['state']=='Idle')
                api('templates/save', {'index':0,'text':'Hello {peercall} from {mycall}'})
                assert api('templates')[0].startswith('Hello')
                selected = api('connect', {'session':0,'interface':0,'call':'REMOTE','via':''})
                assert selected['session'] == 0
                assert radio.frame()[14] == 0x3f
                radio.send(0x73)
                wait(lambda s:s['sessions'][0]['state']=='Connected')
                api('config/save', {'text':cfg.replace('callsign=TEST-1', 'callsign=OTHER-1')}, status=400)
                assert api('connect', {'session':4,'interface':0,'call':'remote-0'})['session'] == 0
                message = 'UTF-8: zażółć <script>alert(1)</script>'
                api('send', {'session':0,'text':message})
                radio.expect(message.encode()+b'\r')
                radio.info(b'\xc5'); radio.info(b'\xbc\r'); radio.info(b'\nOK\r')
                wait(lambda s:any(h['text']=='ż' for h in s['sessions'][0]['history']))
                # Remote commands travel over AX.25 and return a possibly segmented response.
                time.sleep(2.1)
                radio.info(b'/info\r'); radio.expect(b'\r')
                # Incoming YAPP: explicit approval, escaped KISS bytes, complete publication.
                radio.data.clear(); radio.info(b'\x05\x01')
                wait(lambda s:s['sessions'][0]['transfer'] and s['sessions'][0]['transfer']['confirm'])
                api('yapp/accept', {'session':0,'accept':'true'}); radio.expect(b'\x06\x01')
                payload = b'\x00\xc0\xdbhello\xff\r\n/help\r\n'
                header = b'received.bin\0'+str(len(payload)).encode()+b'\0'
                radio.info(b'\x01'+bytes([len(header)])+header); radio.expect(b'\x06\x02')
                radio.info(b'\x02'+bytes([len(payload)])+payload)
                radio.info(b'\x03\x01'); radio.expect(b'\x06\x03')
                assert not (folder/'files'/'received.bin').exists()
                radio.info(b'\x04\x01'); radio.expect(b'\x06\x04')
                wait(lambda s:s['sessions'][0]['transfer'] is None)
                assert api('file?name=received.bin', method='GET') == payload
                assert api('files')['files'][0]['name'] == 'received.bin'
                # Outgoing YAPP handshake and byte-for-byte payload verification.
                radio.data.clear()
                api('yapp/send?session=0&name=sent.bin', raw=payload)
                radio.expect(b'\x05\x01');radio.info(b'\x06\x01')
                header=b'sent.bin\0'+str(len(payload)).encode()+b'\0'
                radio.expect(b'\x01'+bytes([len(header)])+header);radio.info(b'\x06\x02')
                radio.expect(b'\x02'+bytes([len(payload)])+payload);radio.expect(b'\x03\x01')
                radio.info(b'\x06\x03');radio.expect(b'\x04\x01');radio.info(b'\x06\x04')
                wait(lambda s:s['sessions'][0]['transfer'] is None)
                # Rejected unsafe filename cannot escape server storage.
                radio.info(b'\x05\x01');api('yapp/accept', {'session':0,'accept':'true'});radio.expect(b'\x06\x01')
                header=b'../unsafe\x000\x00';radio.info(b'\x01'+bytes([len(header)])+header);radio.expect(b'Transfer error')
                wait(lambda s:s['sessions'][0]['transfer'] is None)
                assert not list((folder/'files').glob('*.part'))
                api('mail/create', {'to':'REMOTE','subject':'Web to radio','body':'Persistent body','kind':'P'})
                stored=wait(lambda s:len(s['mail'])==1)['mail'][0]
                assert api('mail/read', {'id':stored['id']})['body']=='Persistent body'
                api('beacon')
                # Both station and MailBox beacons are emitted.
                for _ in range(2): assert radio.frame()[14]==3
                for _ in range(2): assert second.frame()[14]==3
                api('disconnect', {'session':0});assert radio.frame()[14]==0x53;radio.send(0x73)
                wait(lambda s:s['sessions'][0]['state']=='Idle')
                # Incoming mailbox session and radio read of web-created message.
                mailradio=Radio(sock,destination='TEST-8')
                mailradio.send(0x3f,command=True);assert mailradio.frame()[14]==0x73
                mailradio.expect(b'> ')
                mailradio.info(b'L\r');mailradio.expect(b'Web to radio')
                mailradio.info(f'R {stored["id"]}\r'.encode());mailradio.expect(b'Persistent body')
                mailradio.info(b'Q\r')
                while mailradio.frame()[14]!=0x53: pass
                mailradio.send(0x73)
                wait(lambda s:s['mailboxSessions']==0)
                assert b'REMOTE' in api('monitor/export')
                assert snapshot()['heard']
                # MHeard keeps the latest I frame, complete incoming path and port.
                raw = address('OTHER', True, 0) + address('PATH-1', False, 0)
                for digi in ['DIGA', 'DIGB', 'DIGC', 'DIGD']:
                    raw += address(digi, True, 0)
                raw += address('UNUSED', False, 1) + b'\x46\xf0Hello <packet>\r'
                sock.sendall(b'\xc0\x00' + raw.replace(b'\xdb', b'\xdb\xdd').replace(b'\xc0', b'\xdb\xdc') + b'\xc0')
                heard = wait(lambda s:any(e['call']=='PATH-1' for e in s['heard']))['heard']
                entry = next(e for e in heard if e['call']=='PATH-1')
                assert entry['via']=='DIGA* DIGB* DIGC* DIGD* UNUSED'
                assert entry['port']==0 and entry['utc']
                assert 'I N(S)=3 N(R)=2' in entry['last_frame']
                assert 'Hello <packet><CR>' in entry['last_frame']
                # New-source notifications are opt-in, live and deduplicated across TNCs.
                def mheard_notifications():
                    return [n for n in snapshot()['notifications'] if n['kind']=='mheard_new_station']
                assert not mheard_notifications()
                cfg += 'notification_mheard_new_station_in_app=true\nnotification_mheard_new_station_sound=2\n'
                api('config/save', {'text':cfg})
                assert not mheard_notifications(), 'Enabling must not notify for existing entries'
                # PATH-1 was already received while the option was disabled.
                Radio(second_sock, source='PATH-1', destination='OTHER').send(1)
                wait(lambda s:any(e['call']=='PATH-1' and e['interface']==1 for e in s['heard']))
                assert not mheard_notifications()
                newradio = Radio(sock, source='NEW-3', destination='OTHER')
                newradio.send(1)  # S frame, not APRS/UI-specific.
                wait(lambda s:any(n['kind']=='mheard_new_station' for n in s['notifications']))
                notice = mheard_notifications()
                assert len(notice)==1 and notice[0]['body']=='Odebrano nową stacje: NEW-3'
                assert notice[0]['sound']==2
                newradio.send(1)
                Radio(second_sock, source='NEW-3', destination='OTHER').send(0x3f)
                wait(lambda s:any(e['call']=='NEW-3' and e['interface']==1 for e in s['heard']))
                assert len(mheard_notifications())==1
                Radio(sock, source='NEW-4', destination='OTHER').send(1)
                wait(lambda s:len([n for n in s['notifications'] if n['kind']=='mheard_new_station'])==2)
                for source in ['TEST-1', 'TEST-8']:
                    Radio(sock, source=source, destination='OTHER').send(1)
                # A later valid source synchronizes processing of the preceding frames.
                Radio(sock, source='NEW-4', destination='OTHER').send(1)
                wait(lambda s:any(e['call']=='NEW-4' and e['frames']==2 for e in s['heard']))
                assert len(mheard_notifications())==2
                api('config/save', {'text':cfg.replace('myname=E2E','myname=Saved')})
                wait(lambda s:s['operator']=='Saved')
                assert 'myname=Saved' in config.read_text(encoding='utf-8')
                assert (folder/'station.bak').is_file()
                assert snapshot()['mail'][0]['read']
                api('mail/delete', {'id':stored['id']})
                wait(lambda s:not s['mail'])
                print('PASS: HTTP auth/origin/validation; KISS/AX.25 connect, UTF-8 TX/RX; YAPP RX/TX; traversal rejection; radio MailBox; beacon; configuration persistence; monitor; MHeard and opt-in new-station notifications without cross-TNC duplicates.')
        finally:
            if proc:
                proc.terminate()
                try:proc.wait(timeout=15)
                except subprocess.TimeoutExpired:proc.kill();proc.wait()
            log.close()
            if sys.exc_info()[0]:
                print((folder/'server.log').read_text(encoding='utf-8'))


if __name__=='__main__':main()
