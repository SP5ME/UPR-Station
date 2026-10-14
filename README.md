# UPR Terminal

Wspólne źródła terminala packet radio w dwóch wariantach:

- **UPR Terminal Desktop** — lokalna aplikacja Windows/Linux z oknem Tauri.
- **UPR Terminal Server** — stacja bez okna, obsługiwana przez przeglądarkę; Alpine/OpenRC.

Oba warianty korzystają ze wspólnego silnika AX.25/KISS i panelu WWW.
Desktop uruchamia własny silnik, nie jest klientem zdalnej instalacji Server.
To projekt packet radio, nie aplikacja APRS. Winlink nie jest obecnie obsługiwany.

## Stan projektu

Wersja testowa: **0.1.3-prealpha.1**; bez deklaracji gotowości produkcyjnej.
UPR Terminal to docelowa nazwa projektu. Kod, tytuły okien, instalatory,
pakiety Cargo i usługa zachowują jeszcze historyczne nazwy UltimatePR Station.
Zmiana repozytorium nie zmienia katalogów danych ani identyfikatorów aktualizacji.
Przed publiczną dystrybucją zamknij [listę publikacji i licencji](docs/PUBLISHING.md).
Ten zestaw nie zawiera gotowych wydań.

## Funkcje

Sesje AX.25, interfejsy TNC/KISS, terminal, monitor ramek, MHeard, digipeater,
beacony, lokalny MailBox, YAPP i konfigurowalne powiadomienia.
Usługi radiowe serwera działają niezależnie od karty przeglądarki.

## Budowanie i testy

Wymagany Rust zgodny z manifestami; deklarowane minimum wymaga osobnej walidacji.
Zależności są zablokowane w Cargo.lock.

```sh
cargo test --locked --tests
cargo build --locked --bin ultimatepr-station
node tests/mheard-routes.cjs
node tests/notification-audio.cjs
node tests/notification-sounds.cjs
python tests/e2e.py target/debug/ultimatepr-station
```

Na Windows ostatni argument ma rozszerzenie `.exe`. Testy używają izolowanych
katalogów pod target/, bez konfiguracji operatora i bez prawdziwego radia.

- [Budowanie instalatorów](scripts/README.md)
- [Instalacja i aktualizacja Alpine](deploy/alpine/README.md)
- [Obsługa i diagnostyka](docs/DEPLOYMENT.md)
- [Architektura](docs/ARCHITECTURE.md)
- [Zmiany](docs/CHANGES-DESKTOP.md)
- [Współpraca](CONTRIBUTING.md) i [bezpieczeństwo](SECURITY.md)

Paczki testowe publikuj osobno w Releases, nie w historii kodu.
Do aktualizacji Server przez stronę służy ZIP Alpine, nie wspólny ZIP testera.
Po aktualizacji odśwież panel przez Ctrl+F5.

## Licencje

Własny kod deklaruje [Apache License 2.0](LICENSE). Komponenty zewnętrzne
zachowują swoje licencje: [THIRD-PARTY](docs/THIRD-PARTY.md), [marka](TRADEMARKS.md).
Nie jest to zapewnienie zakończonego audytu pochodzenia wszystkich fragmentów.
