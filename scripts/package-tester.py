"""Local Station-only tester handoff. Never publishes or changes existing data."""
import argparse
import hashlib
from pathlib import Path
import re
import shutil
import subprocess
import sys
import zipfile

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--binary', required=True)
parser.add_argument('--arch', choices=['x86_64', 'aarch64', 'armv7'], required=True)
parser.add_argument('--tag', required=True)
parser.add_argument('--licenses', required=True)
parser.add_argument('--output', default='release/current')
args = parser.parse_args()
version=re.search(r'^version\s*=\s*"([^"]+)"', (root/'Cargo.toml').read_text(encoding='utf-8'), re.M)[1]
# package-server validates tag before it can be used as a path component.
packages = root/'target/tester-assets'/args.tag
if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,100}', args.tag):
    raise SystemExit('Invalid build tag')
subprocess.run([sys.executable, str(root/'scripts/package-server.py'), '--binary', args.binary,
                '--arch', args.arch, '--tag', args.tag, '--licenses', args.licenses,
                '--output', str(packages)], check=True)
output = Path(args.output).resolve()
output.mkdir(parents=True, exist_ok=True)
name = f'UPR-Station-{args.tag}'
folder, outer = output/name, output/(name+'.zip')
if folder.exists() or outer.exists():
    raise SystemExit('Refusing to replace an existing tester package')
folder.mkdir()
(folder/'alpine').mkdir()
(folder/'linux').mkdir()
for item in packages.iterdir():
    shutil.copy2(item, folder/('alpine' if item.suffix == '.zip' else 'linux')/item.name)
marker = "raise SystemExit('Unexpanded helper: build a binary package with package-server.py')"
script = (root/'deploy/server/update-tester.sh').read_text()
assert script.count(marker) == 1
(folder/'AKTUALIZUJ-STATION.sh').write_text(script.replace(marker, (root/'deploy/server/update-package.py').read_text()), encoding='utf-8', newline='\n')
inner = f'UPR-Station-{args.tag}-alpine-{args.arch}.zip'
(folder/'START-TUTAJ.md').write_text(f'''# UPR Station — paczka testera

Wersja aplikacji: {version}. Build: {args.tag}. Architektura: {args.arch}.
To jest wyłącznie serwer Station, bez Terminala, Windows i skryptów Proxmox.

## Aktualizacja w panelu WWW

1. Zrób kopię /etc/ultimatepr i /var/lib/ultimatepr oraz zaplanuj rozłączenie sesji radiowych.
2. Rozpakuj tę paczkę testera na komputerze. Do WWW → Aktualizacja wgraj WYŁĄCZNIE:
   alpine/{inner}
   Nie wgrywaj zewnętrznego ZIP-a testera ani archiwum linux/*.tar.gz.
3. Potwierdź instalację. Podczas restartu połączenie panelu może chwilowo zniknąć.
4. Po pierwszym uaktualnieniu ze starej wersji naciśnij Ctrl+F5 lub Ctrl+Shift+R.
   Następne aktualizacje uruchomione z nowego panelu odświeżą kartę po sukcesie.
5. Sprawdź zapis ustawień, TNC i powiadomienia. Nie restartuj całego kontenera.

Wymagania nowego updatera: Python 3, wget, zarządzana usługa OpenRC albo systemd.
Starszy Alpine może nie mieć python3: jako root wykonaj apk add python3 PRZED testem.
Pobieranie z GitHub wymaga dostępu HTTPS i certyfikatów CA. Wydania testera nie
opublikowano — GitHub może pokazywać starsze wydanie, którego nie należy instalować.
Zmień początkowe hasło admin/packet, jeżeli nie zrobiłeś tego wcześniej.

## Gdy stary helper nie obsługuje aktualizacji / systemd nie ma jeszcze updatera

Przenieś cały rozpakowany katalog testera na serwer i uruchom jako root:

    sh AKTUALIZUJ-STATION.sh

Skrypt używa wyłącznie lokalnego ZIP-a; nie pobiera kodu i nie kompiluje źródeł.
Weryfikuje produkt/architekturę/sumy, przełącza usługę i sprawdza health oraz proces.
Nie jest instalatorem nowej stacji. Nie uruchamiaj go w kontenerze Docker — Docker
aktualizuj instalatorem na hoście, z zachowaniem wolumenu danych.

## Pozostałe pliki

linux/ zawiera tę samą statyczną binarkę musl w TAR.GZ dla wdrożeń natywnych.
Nie używaj archiwum na innej architekturze. SHA256SUMS.txt obejmuje pliki testera;
wewnętrzny ZIP zawiera osobne sumy każdego pliku i manifest.
''', encoding='utf-8', newline='\n')
(folder/'ZMIANY-I-TESTY.md').write_text((root/'docs/UPDATE-TESTING.md').read_text(), encoding='utf-8', newline='\n')
files = sorted(p for p in folder.rglob('*') if p.is_file())
(folder/'SHA256SUMS.txt').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+p.relative_to(folder).as_posix()+'\n' for p in files), newline='\n')
with zipfile.ZipFile(outer, 'x', zipfile.ZIP_DEFLATED) as archive:
    for path in sorted(folder.rglob('*')):
        if path.is_file():
            archive.write(path, name+'/'+path.relative_to(folder).as_posix())
print(outer)
print('SHA-256: '+hashlib.sha256(outer.read_bytes()).hexdigest())
