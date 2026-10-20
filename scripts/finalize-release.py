import hashlib
from pathlib import Path
import sys
tag,directory=sys.argv[1:]
root=Path(directory)
required=['install-server.sh','install.sh']
required += [f'UPR-Station-{tag}-{kind}-{arch}.{extension}' for arch in ['x86_64','aarch64','armv7'] for kind,extension in [('linux','tar.gz'),('alpine','zip'),('docker','tar.gz')]]
unexpected=[p.name for p in root.iterdir() if p.is_file() and p.name not in required+['RELEASE-NOTES.md','SHA256SUMS.txt']]
if unexpected: raise SystemExit('Unexpected release assets: '+', '.join(unexpected))
for name in required:
    if not (root/name).is_file() or not (root/name).stat().st_size: raise SystemExit('Missing release asset: '+name)
notes=f'''# UPR Station — {tag}

Wydanie testowe pre-alfa. Wersja aplikacji: 0.1.3-prealpha.1.
To wydanie zawiera wyłącznie UPR Station; drugi produkt ma osobne repozytorium.

Linux/musl: x86-64, ARM64 i ARMv7. Alpine: ZIP do panelu aktualizacji.
Docker: obrazy tar.gz dla tych samych architektur. ARMv6 nie jest obsługiwany.
Instalator rozpoznaje Alpine/OpenRC oraz Debian/Ubuntu/Raspberry Pi OS/systemd.
Opcja --docker wymaga istniejącego Docker Engine. Bez skryptów Proxmox.
Zachowano konfigurację i katalogi danych. Po aktualizacji panelu Ctrl+F5.

## Zmiany w tym wydaniu

- Panel aktualizacji: osobne sekcje GitHub i ZIP, wyrównane kontrolki i układ mobilny.
- Sprawdzanie wersji na publicznym panelu nie otwiera dodatkowego logowania.
  Pobieranie, przesłanie ZIP-a i instalacja nadal wymagają operatora.
- Pobieranie gotowych paczek z GitHub, kanały stabilny/testowy, identyfikator buildu
  i weryfikacja SHA-256, produktu, architektury oraz plików ZIP przed instalacją.
- Aktualizacje bez kompilacji źródeł jako root; niezależny helper OpenRC/systemd,
  kontrola procesu/health oraz próba rollbacku bez nadpisywania danych.
- Nowy panel odświeża kartę po aktualizacji; restart może wymagać ponownego logowania.

Wymagane dla updatera natywnego: Python 3 i wget. Starszy Alpine może wymagać
`apk add python3 wget ca-certificates` przed pierwszym testem. Pierwszą aktualizację
przeprowadza jeszcze stary helper, z jego dotychczasowymi ograniczeniami.
Docker aktualizuj z hosta, nie z panelu wewnątrz kontenera. Przed aktualizacją
wykonaj kopię /etc/ultimatepr i /var/lib/ultimatepr; sesje radiowe zostaną rozłączone.
Testy helpera używają atrap usług. Pełny upgrade/rollback na rzeczywistym OpenRC
i systemd nadal wymaga testu wdrożeniowego na kopii instalacji.

Instalacja/aktualizacja: pobierz install.sh, następnie `sudo sh install.sh`.
Opcja `--version {tag}` przypina wydanie; domyślnie najnowszy opublikowany tag,
także pre-release. Skrypt pobiera tylko pliki z SP5ME/UPR-Station i sprawdza SHA-256.

Instalacja jednym poleceniem z publicznego repozytorium:
`curl -fsSL https://github.com/SP5ME/UPR-Station/releases/download/{tag}/install.sh | sudo sh`

Polecenie wykonuje pobrany kod. Repozytorium prywatne wymaga autoryzowanego
pobrania skryptu i GITHUB_TOKEN z odczytem repozytorium; nie wpisuj tokenu do argumentów.
Testy CI nie zastępują testów rzeczywistego radia i aktualizacji użytkownika.
Audyt pochodzenia kodu opisany w docs/PUBLISHING.md pozostaje otwarty.
'''
(root/'RELEASE-NOTES.md').write_text(notes, encoding='utf-8', newline='\n')
with (root/'SHA256SUMS.txt').open('w', encoding='utf-8', newline='\n') as output:
    for path in sorted(root.iterdir()):
        if path.is_file() and path.name!='SHA256SUMS.txt':
            digest=hashlib.sha256()
            with path.open('rb') as stream:
                while chunk:=stream.read(1024*1024): digest.update(chunk)
            output.write(f'{digest.hexdigest()}  {path.name}\n')
print(f'Validated {len(required)} required release assets.')
