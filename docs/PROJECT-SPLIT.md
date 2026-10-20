# Rozdzielenie projektów — 2026-10-20

Projekt: **UPR Station**, docelowe repozytorium **SP5ME/UPR-Station**.
Baza historii: `eb42f2e0056c56f2ec0560330186e18641a83609`.
Wersja pozostaje `0.1.3-prealpha.1`. Brak migracji schematu danych.
Nie wykonano push ani wydania. Repozytorium Station nie jest tworzone automatycznie.

Oba produkty mają pełne własne źródła protokołów AX.25/KISS, poczty,
YAPP, MHeard i testów. Nie korzystają z katalogu sąsiedniego projektu.
Poprawki wspólnej logiki trzeba odtąd świadomie przenosić między repozytoriami.
Poprzednia wspólna historia pozostaje w Git; stare pliki mogą być widoczne w dawnych commitach.

Station nie zawiera Tauri, zasobnika, autostartu użytkownika ani instalatorów
desktopowych. Usługa i binarka nadal nazywają się `ultimatepr-station`.
Zachowano `/opt/ultimatepr-station`, `/var/lib/ultimatepr` i formaty konfiguracji.
Klucze native w starych regułach powiadomień są zgodnościowo odczytywane,
ale nie wywołują integracji systemowego desktopu.
Lokalne dane, kopie, paczki i prywatne notatki mogą pozostawać obok źródeł;
nie należą do inwentarza publikacji Git ani pakietów wydania.

Weryfikacja lokalna nie oznacza testu każdej platformy docelowej ani radia.
Linux/AppImage/DEB, Alpine i Docker wymagają weryfikacji w właściwym środowisku;
workflow wydania buduje tylko produkt własnego repozytorium.
