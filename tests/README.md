# Testy projektu

Testy są źródłami potrzebnymi do rozwoju projektu, nie plikami do instalacji.
Wyniki, logi i paczki pozostają w ignorowanych katalogach `target/` i `release/`.
Wszystkie poniższe polecenia uruchamiaj z głównego katalogu repozytorium.

## Układ

- `*.rs` — testy integracyjne automatycznie wykrywane przez Cargo; pozostają w tym katalogu.
- `e2e.py` — testy HTTP/KISS z lokalną binarką, bez prawdziwego radia.
- `browser/` — testy pomocników interfejsu i powiadomień, uruchamiane przez Node.js.
- `packaging/` — testy formatów paczek, sum kontrolnych i konfiguracji instalatora.
- `installers/` — testy instalatorów i cyklu aktualizacji.
- `manual/` — ręczne i historyczne sprawdzanie desktopu oraz dawnej paczki portable; nie należą do CI.

## Automatyczne kontrole

```sh
cargo test --locked --tests
cargo build --locked --bin ultimatepr-station
python tests/e2e.py target/debug/ultimatepr-station
node tests/browser/mheard-routes.cjs
node tests/browser/notification-audio.cjs
node tests/browser/notification-sounds.cjs
node tests/browser/product-identity.cjs
node tests/browser/settings-variants.cjs
python tests/packaging/desktop-release.py
python tests/packaging/server-package.py
python tests/packaging/release-download.py
python tests/packaging/installer-hooks.py
bash tests/installers/server-installer.sh
bash tests/installers/bootstrap.sh
bash tests/installers/alpine-update.sh
```

Na Windows binarka przekazywana do `e2e.py` ma rozszerzenie `.exe`.
Testy skryptów Alpine używają atrap usług i procesów, nie prawdziwej instalacji.

`installers/windows-installer.ps1` wykonuje rzeczywistą instalację i zapisuje
testowe wpisy rejestru. Jest ograniczony do tymczasowego runnera Windows na
GitHub Actions i uruchamiany przez workflow wydania, nigdy na komputerze operatora.

## Kontrole ręczne i historyczne

`manual/windows-desktop-smoke.ps1` i `manual/desktop-smoke.mjs` tworzą parę
ręcznych testów GUI/CDP. Używają jawnych ścieżek do nowego profilu pod `target/`,
bez znaku stacji i bez interfejsów TNC. Nie włączają autostartu ani nie nadają ramek.
Wymagają Windows z WebView2 i zasobnikiem systemowym; nie należą do CI.
`manual/release_check.py` dotyczy dawnego układu paczki Windows portable,
nie obecnych instalatorów NSIS, DEB ani AppImage.

Zachowano je jako źródła testów, ale nie są potwierdzeniem poprawności bieżącego wydania.
