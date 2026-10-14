# Walidacja wydania desktop — 2026-10-01

To wydanie testowe. Nie jest jeszcze potwierdzone jako wydanie produkcyjne
na całej macierzy platform.

## Potwierdzone podczas implementacji

- Kopia stanu sprzed zmian: `backups/before-desktop-20261001-130731`, SHA-256.
- Windows: Rust/Cargo 1.98.1; WebView2 154.0.4258.37.
- Pierwszy build Tauri 2.11.6 i załadowanie istniejącego panelu w WebView2.
- Dotychczasowe testy Rust: 100 zaliczonych, przed dodaniem nowych testów.
- Lokalne E2E serwera: autoryzacja, Origin, walidacja, KISS/AX.25, UTF-8,
  YAPP RX/TX, MailBox, beacon, trwałość konfiguracji, monitor i MHeard.
- Nowy test runtime: losowy port, token API, odrzucenie obcego Origin,
  blokada danych, zwolnienie zasobów i ponowne uruchomienie.
- Runtime zwraca i testuje osobno profile `server` oraz `desktop`; oba profile
  przechodzą kontrolę autoryzacji i zwolnienia zasobów.
- Test runtime obejmuje zapis statusu oraz upload paczki aktualizacji; instalacja
  jest celowo wykonywana wyłącznie przez helper OpenRC na Alpine.
- Pełna regresja Rust po zmianach: 76 testów biblioteki, 10 automatyzacji,
  24 testy link/AX.25 oraz 2 testy runtime — wszystkie zaliczone.
- Testy preferencji desktopu i cytowania ścieżek autostartu Windows.

Clippy z `-D warnings` na Rust 1.98.1 zgłasza ostrzeżenia także dla oryginalnej
kopii projektu: m.in. lzhuf i wl2k_protocol. Nie oznaczamy tego sprawdzenia jako
zaliczonego i nie refaktoryzujemy przy tej okazji protokołów radiowych.

## Lista odbioru przed wydaniem produkcyjnym

- [ ] Instalacja, aktualizacja, odinstalowanie na czystym Windows 11.
- [ ] Budowa DEB na Debian 12 x64 i test na Debian 12 oraz Ubuntu 24.04.
- [ ] GNOME i KDE, sesje X11/Wayland, zasobnik obecny i nieobecny.
- [ ] Powiadomienia systemowe wszystkich kategorii z wersji zainstalowanej.
- [ ] Autostart po rzeczywistym wylogowaniu i ponownym zalogowaniu.
- [ ] Zamknięcie systemu podczas aktywnej sesji; kontrola trwałości danych.
- [ ] Instrukcja migracji i odtworzenia kopii wykonana na paczkach końcowych.
- [ ] Weryfikacja z fizycznym TNC na każdym wspieranym OS.
- [ ] Serwer Linux amd64/arm64 i kontener, bez bibliotek GUI.
- [ ] Alpine: instalacja usługi OpenRC, restart systemu, `verify-service.sh`,
  dostęp panelu z drugiego komputera i rollback aktualizacji.
- [ ] Sprawdzenie deklarowanego minimum Rust 1.88.
- [ ] Podpisanie instalatora i końcowy przegląd licencji przed publikacją.

Na stanowisku implementacyjnym brak WSL/Docker oraz środowiska Linux.
Testy demo nie zastępują testu radiowego. Brak powiadomień zależny od konfiguracji
OS nie może być interpretowany jako brak pracy stacji.
