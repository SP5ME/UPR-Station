import hashlib
from pathlib import Path
import sys
tag,directory=sys.argv[1:]
root=Path(directory)
required=[f'UPR-Terminal-{tag}-windows-x86_64-setup.exe',f'UPR-Terminal-{tag}-linux-x86_64.deb','install-server.sh']
required += [f'UPR-Station-{tag}-{kind}-{arch}.{extension}' for arch in ['x86_64','aarch64','armv7'] for kind,extension in [('linux','tar.gz'),('alpine','zip'),('docker','tar.gz')]]
for name in required:
    if not (root/name).is_file() or not (root/name).stat().st_size: raise SystemExit('Missing release asset: '+name)
notes=f'''# UPR Terminal / UPR Station — {tag}

Wydanie testowe pre-alfa. Wersja aplikacji pozostaje 0.1.3-prealpha.1.

- UPR Terminal: Windows x86-64 (NSIS) oraz Linux x86-64 (DEB, Debian/Ubuntu).
- UPR Station: statyczne binarki Linux/musl dla x86-64, ARM64 i ARMv7.
- Alpine: ZIP odpowiedniej architektury do panelu aktualizacji.
- Raspberry Pi OS: ARM64 dla systemu 64-bitowego, ARMv7 dla 32-bitowego; ARMv6 nie jest obsługiwany.
- Docker: osobny obraz zapisany w tar.gz dla każdej architektury; bez skryptów Proxmox.

## Instalacja serwera jednym poleceniem po pobraniu install-server.sh

`sudo sh install-server.sh --version {tag}`

Tryb Docker: `sudo sh install-server.sh --version {tag} --docker`.
Skrypt dobiera system i architekturę, sprawdza SHA-256, zachowuje konfigurację
i kontroluje uruchomienie. Domyślnie panel jest dostępny tylko lokalnie.
Prywatne repozytorium wymaga GITHUB_TOKEN z prawem odczytu zawartości;
przekaż go przez środowisko, nie zapisuj w plikach. Po aktualizacji Ctrl+F5.

Linux desktop wymaga WebKitGTK 4.1; nie jest to pakiet dla wszystkich dystrybucji.
Instalator Windows nie jest podpisany certyfikatem wydawcy.
CI kompiluje i sprawdza uruchomienie binarek; nie zastępuje testu radia,
pełnej instalacji na Raspberry Pi ani aktualizacji istniejącego komputera.
Audyt pochodzenia inspiracji pozostaje otwarty; nie upubliczniaj repozytorium
bez zamknięcia listy w docs/PUBLISHING.md.
'''
(root/'RELEASE-NOTES.md').write_text(notes)
with (root/'SHA256SUMS.txt').open('w') as output:
    for path in sorted(root.iterdir()):
        if path.is_file() and path.name!='SHA256SUMS.txt':
            digest=hashlib.sha256()
            with path.open('rb') as stream:
                while chunk:=stream.read(1024*1024): digest.update(chunk)
            output.write(f'{digest.hexdigest()}  {path.name}\n')
print(f'Validated {len(required)} required release assets.')
