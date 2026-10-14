# Architektura i budowanie

## Wspólny rdzeń

`src/runtime.rs` udostępnia `RuntimeOptions`, `start()` i `StationRuntime`.
Uchwyt zwraca rzeczywisty adres HTTP, przyjmuje `request_stop()`, umożliwia
`is_finished()` oraz `wait()`. Porzucenie uchwytu zatrzymuje i dołącza wątki.
Blokada danych pozostaje aktywna przez cały czas życia backendu.

`src/main.rs` jest adapterem CLI. Zachowuje argumenty, sygnały i model pracy
serwera. Moduły HTTP, stacji, historii i transferów są prywatnymi modułami
biblioteki `ultimatepr`. Publiczne API radiowe i endpointy HTTP nie zmieniają się.

Wątek stacji wykonuje polecenia i tick radia niezależnie od GUI. HTTP nadal używa
ograniczonej kolejki poleceń i odczytu migawki stanu. Wygaszanie wywołuje istniejące
`Station::shutdown()` i czeka maksymalnie 12 sekund na sesje. systemd i Compose
mają 20 sekund na zatrzymanie. Nagłe zabicie procesu lub utrata zasilania nie
gwarantują zakończenia transmisji.

`StationEvent` to wewnętrzny kontrakt powiadomień i dźwięków. Rodzaje zdarzeń
określa `NotificationKind` w `src/station_events.rs`. Desktop otrzymuje
je przez ograniczony kanał `sync_channel(64)` z `try_send`: przy przeciążeniu
powiadomienie może zostać pominięte, radio nie czeka. Treść poczty nie przechodzi
przez kanał. Obecna poczta ustala punkt początkowy obserwacji przy starcie i
przeładowaniu konfiguracji. Serwer nie podłącza odbiorcy powiadomień.

## Desktop

Osobny manifest i lockfile `desktop/` izolują zależności GUI od serwera.
Tauri 2 obsługuje główny wątek GUI; backend działa wewnątrz tego samego procesu.
Ustawienia natywne są w menu „Stacja”, także gdy zasobnik jest niedostępny.
Brak capabilities Tauri dla stron oznacza brak dostępnych komend natywnych
w panelu. Integracje systemowe są wywoływane z Rust.

Backend desktopu nasłuchuje na loopbackie i przydzielonym porcie. Token 32-bajtowy
pochodzi z systemowego generatora losowego. Skrypt inicjalizacyjny przekazuje go
do sessionStorage wyłącznie przy zgodnym origin. Obowiązuje tylko do zamknięcia
procesu. Zewnętrzne nawigacje i nowe okna HTTP(S) przechodzą do przeglądarki
systemowej. Nie włączać zdalnych capabilities ani debugowania w wydaniu.

Na Linuksie dostępność zasobnika potwierdza D-Bus StatusNotifierWatcher.
Utrata hosta zasobnika przywraca okno. Brak potwierdzenia oznacza dialog przy
zamykaniu. Powiadomienia i kontrola backendu działają poza JavaScriptem panelu.
Autostart rejestruje wpis HKCU Run na Windows lub plik XDG autostart na Linux.
Nigdy nie jest włączany samą instalacją lub uruchomieniem.

## Budowanie

Serwer, jak dotychczas:

```sh
cargo test --offline --all-targets
cargo build --offline --locked --release --bin ultimatepr-station
python tests/e2e.py target/debug/ultimatepr-station
bash scripts/build-linux.sh
```

Na Windows użyj `.exe` w ścieżce testu E2E. Build Linux serwera wymaga Docker;
nie wymaga Tauri, GTK, WebKit ani Node.

Desktop wymaga Rust, narzędzi C/C++ oraz Tauri CLI 2.11.5:

```sh
cargo install tauri-cli --version 2.11.5 --locked
```

Windows: Build Tools z narzędziami MSVC i WebView2. Budowanie instalatora:

```powershell
PowerShell -File scripts/build-desktop.ps1
```

Skrypt przyjmuje `-Offline` i opcjonalne `-CliScript` wskazujące oficjalny
skrypt Node Tauri CLI. Node nie jest wymagany przy użyciu Cargo Tauri CLI.
Instalator NSIS jest obecnie niepodpisany; produkcyjne podpisywanie wymaga
dostarczenia certyfikatu i oddzielnej konfiguracji, bez sekretów w repozytorium.

Linux desktop: buduj na najstarszej wspieranej dystrybucji (Debian 12 x64),
a powstały DEB testuj także na Ubuntu 24.04. Potrzebne pakiety deweloperskie:

```sh
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev patchelf python3
bash scripts/build-desktop-linux.sh
```

Wersje zależności wyznaczają osobne lockfile. Manifest desktopu deklaruje Rust
1.88; faktycznie sprawdzony toolchain znajduje się w VALIDATION.md. Nie podnosić
minimum bez osobnej weryfikacji. Nie deklarować zgodności z 1.88 wyłącznie na
podstawie resolvera nowszego Cargo.

Do pierwszego przygotowania offline potrzebne są pobrane crate'y dla docelowej
platformy, toolchain, Tauri CLI, zależności OS i narzędzia pakowania NSIS/DEB.
`cargo fetch --locked --manifest-path desktop/Cargo.toml --target TARGET`
przygotowuje crate'y; nie instaluje zależności OS ani NSIS. Następne kompilacje
mogą korzystać z `CARGO_NET_OFFLINE=true`. Nie traktować tego jako instalatora
wszystkich zależności offline.

Przed bezpośrednim buildem desktopu uruchom `python scripts/desktop-licenses.py`.
Skrypty pakujące robią to automatycznie. Licencje trafiają do zasobów instalatora;
brak tekstów blokuje skrypt wydaniowy. Uzupełnienia w `licenses/desktop/` mają
źródła wskazujące dokładną rewizję upstream. Po aktualizacji lockfile odtwórz
noty i sprawdź brakujące teksty. Nie publikuj paczki z niezakończonym audytem.

Nowe paczki desktopu i sumy SHA-256 trafiają do świeżych katalogów pod
`release/desktop/`. Dokumentacja znajduje się także w zasobach instalatora;
instrukcja użytkownika jest dostępna z menu aplikacji bez internetu.

Oficjalne źródła: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/),
[Tauri bundling](https://v2.tauri.app/distribute/),
[API okna](https://docs.rs/tauri/2.11.6/tauri/webview/struct.WebviewWindowBuilder.html).
