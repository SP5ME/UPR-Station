# UPR Station

Niezależna aplikacja packet radio (AX.25/KISS), wersja `0.1.3`.
Repozytorium: `SP5ME/UPR-Station`. Serwer bez GUI systemowego, z panelem WWW, Telegramem i MQTT.

To nie jest APRS. Funkcje obejmują terminal radiowy, digipeater, MailBox,
YAPP, beacon, monitor i MHeard. Winlink i demo nie są częścią projektu.
Zanim uruchomisz radio, ustaw znak i właściwy interfejs TNC.

## Budowanie i testy

Wymagany Rust zgodny z manifestem i zależności zapisane w Cargo.lock.
Projekt jest samodzielny; nie pobiera źródeł z sąsiedniego folderu.

```sh
cargo test --locked --tests
cargo build --locked --release --bin ultimatepr-station
python3 tests/e2e.py target/release/ultimatepr-station
```

Dystrybucja Linux/musl/Alpine/Docker: `.github/workflows/release.yml`.
Lokalne budowanie przez Docker: `bash scripts/build-linux.sh`.


Testy protokołów: `cargo test --locked --tests`.
Testy JS: pliki `tests/browser/*.cjs` uruchamiane przez Node.
Kontrola publikacji: `python scripts/verify-publication.py`.
Testy źródłowe pozostają w repo; dane operatora i wyniki są ignorowane.

## Instalacja i wydania

[Instalacja Linux](docs/LINUX-INSTALL.md). Workflow wydania jest uruchamiany
ręcznie i weryfikuje komplet paczek tylko tego produktu przed publikacją.
Podział lokalny nie wykonał push ani release.

## Zgodność i bezpieczeństwo

[Opis podziału](docs/PROJECT-SPLIT.md), [publikacja](docs/PUBLISHING.md),
[materiały osób trzecich](docs/THIRD-PARTY.md), [SECURITY.md](SECURITY.md).
Zachowano stare identyfikatory instalacji i ścieżki danych. Wykonaj kopię
profilu przed ręcznym wdrożeniem; nie dołączaj sekretów do Git ani zgłoszeń.
Licencja własnego kodu: Apache-2.0; licencje zależności pozostają niezależne.
Audyt pochodzenia inspiracji nie jest uznany za zakończony.
