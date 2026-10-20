# Binarna paczka aktualizacji Station

Do panelu WWW wgrywamy wewnętrzny plik `alpine/UPR-Station-…-alpine-ARCH.zip`,
nie zewnętrzny ZIP testera. Paczka jest budowana przez `scripts/package-server.py`.

W katalogu głównym ZIP-a znajdują się: `ultimatepr-station`, `Cargo.toml`
(zgodność ze starszym helperem), `PACKAGE-MANIFEST.txt` oraz `SHA256SUMS.txt`.
Katalog `deploy/` zawiera helper i obsługę usług, `licenses/` teksty licencji.
Nie ma źródeł, konfiguracji instalacji, danych użytkownika, target/ ani desktopu.

Manifest zawiera nazwę UPR-Station, version, build_tag, architecture,
variant=linux-headless-server-alpine-musl, source=binary oraz binary_sha256.
Nowy helper wymaga kompletnej listy sum i weryfikuje ELF oraz każdy plik,
odrzuca linki, duplikaty, niebezpieczne ścieżki, niewłaściwy produkt i architekturę.
Nie uruchamia Cargo ani skryptów pobranych z GitHub.

Pobieranie, przesłanie ZIP-a i instalacja wymagają konta operatora także przy
publicznym panelu. Samo sprawdzenie wersji/statusu nie wymaga dodatkowego
logowania, gdy panel jest publiczny. Dane pozostają w istniejących ścieżkach; poprzedni katalog wydania
zostaje zachowany do wycofania zmiany. Nie restartujemy całego kontenera.

Starszy helper przeprowadza tylko pierwszą aktualizację i może mieć własne
ograniczenia. W takim przypadku służy lokalny `AKTUALIZUJ-STATION.sh` z paczki
testera. Wymagane Python 3 i wget. Pełny opis testów i ograniczeń:
[UPDATE-TESTING.md](UPDATE-TESTING.md).
