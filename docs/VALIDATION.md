# Kontrola podziału — UPR Station, 2026-10-20

Środowisko lokalne: Windows x86-64, Rust i Node. Brak testu instalacji na produkcji.

- Silnik: `cargo test --offline --locked --tests` — 115 testów.
- JavaScript: pięć testów `tests/browser/*.cjs` i sprawdzenie składni.
- Pakowanie i pobieranie: testy `tests/packaging/*.py` (dane syntetyczne, brak sieci).
- Instalator: `tests/installers/bootstrap.sh` — detekcja, walidacja i izolacja produktu.
- Kontrola publikacji: inwentarz Git; test odrzucania prywatnych ścieżek.
- Test HTTP/KISS: AX.25, YAPP, MailBox, beacon, monitor, MHeard i nowe stacje.
- Izolowany panel w headless Chrome: inicjalizacja, kanały powiadomień,
  opcje WWW i zapis konfiguracji; brak autostartu/natywnego hosta.
- Testy instalatora serwera i kontraktów aktualizacji Alpine bez instalowania usługi.

Nie wykonano kompilacji/instalacji Linux, Alpine, Raspberry Pi OS ani Dockera
w tym środowisku Windows. Odpowiednie workflow pozostają do wykonania na GitHub
po osobnej zgodzie na push/wydanie. Nie testowano rzeczywistego radia ani pełnej
aktualizacji profilu operatora. Paczki testów pakowania nie są binarkami do dystrybucji.
Nie opublikowano wydania i nie zmieniono jego numeru.
