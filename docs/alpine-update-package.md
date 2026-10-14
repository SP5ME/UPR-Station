# Format paczki aktualizacji Alpine

Panel aktualizacji Alpine akceptuje ZIP zawierający manifest oraz albo źródła,
albo binarkę `ultimatepr-station` zbudowaną dla Alpine/musl.

Manifest musi nazywać się `PACKAGE-MANIFEST.txt` lub `UPDATE-MANIFEST.txt` i
zawierać:

```text
name=...
version=...
variant=linux-headless-server-alpine-musl
```

Paczka źródłowa musi zawierać `Cargo.toml`, `Cargo.lock` i `src/main.rs`.
Paczka binarna musi zawierać wykonywalny plik `ultimatepr-station`. Dla paczki
binarnej można dodać `binary_sha256=...`; helper porówna tę wartość z SHA-256
binarki przed zatrzymaniem usługi.

Przykładową paczkę źródłową tworzy:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-alpine-transfer.ps1 -OutputRoot .\release\alpine-test
```

ZIP nie może zawierać danych użytkownika, `data/`, `target/` ani konfiguracji
instalacji. Aktualizacja działa przez panel WWW wyłącznie po potwierdzeniu
operatora. Błędny manifest, nieobsługiwany wariant, niebezpieczna ścieżka lub
niezgodna suma SHA-256 powodują odrzucenie paczki.

Jeżeli zainstalowany helper powoduje rollback i w logu pojawia się
`Another Station is using this configuration`, wdrożenie naprawcze należy
wykonać skryptem `alpine/03-ISTNIEJACY-KONTENER.cmd` z aktualnej paczki testera.
Skrypt uruchamia helper rozpakowany z wybranej paczki jako root przez Proxmox,
zamiast używać starego helpera w kontenerze. Nowy helper zostanie zainstalowany
na stałe po udanym uruchomieniu aplikacji. Podczas przełączenia wydania panel
jest chwilowo niedostępny; restart całego kontenera nie jest wymaganym krokiem.
