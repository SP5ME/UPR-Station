# Skrypty budowania UltimatePR Station

Polecenia uruchamiaj z głównego katalogu projektu. Na Windows używaj
PowerShella, a na Linuxie powłoki Bash. Skrypty nie kopiują katalogu `data/`
do paczek instalacyjnych.

## Jedno polecenie na Windows

Z katalogu głównego projektu uruchom:

```powershell
.\build-installer.ps1
```

Skrypt zbuduje instalator NSIS i zapisze go wraz z sumami SHA-256 w
świeżym katalogu pod `release/desktop/`. Dla budowania bez
połączenia z siecią użyj `.\build-installer.ps1 -Offline`.

## Windows — instalator desktop

Wymagane:

- Rust/Cargo zgodny z `rust-version` projektu;
- Visual Studio Build Tools z narzędziami MSVC;
- WebView2 Runtime;
- Tauri CLI 2.11.5 albo oficjalny skrypt Node Tauri CLI.

Najpierw, jednorazowo, można zainstalować Tauri CLI:

```powershell
cargo install tauri-cli --version 2.11.5 --locked
```

Następnie:

```powershell
PowerShell -ExecutionPolicy Bypass -File scripts/build-desktop.ps1
```

Skrypt pobiera lub sprawdza zależności, zbiera licencje, buduje aplikację
desktop i tworzy instalator NSIS. Wynik trafia do nowego katalogu:

```text
release/desktop/windows-x64-YYYYMMDD-HHmmss/
```

W katalogu znajdują się instalator, dokumentacja i `SHA256SUMS.txt`.

Jeśli Tauri CLI jest uruchamiane przez oficjalny skrypt Node, podaj jego ścieżkę:

```powershell
PowerShell -ExecutionPolicy Bypass -File scripts/build-desktop.ps1 `
  -CliScript .\target\desktop-tools\cli\package\tauri.js
```

Po wcześniejszym pobraniu wszystkich crate'ów można użyć trybu offline:

```powershell
PowerShell -ExecutionPolicy Bypass -File scripts/build-desktop.ps1 -Offline
```

Tryb offline nie instaluje WebView2, MSVC ani NSIS. Muszą być już dostępne w
systemie. Przed aktualizacją zamknij działającą aplikację desktop; serwera
Linux ani działającej instancji serwerowej Windows nie trzeba zatrzymywać podczas
samego budowania.

## Linux — pakiet desktop DEB

Pierwszą kompilację wykonuj na Debianie 12 x64 albo równoważnym środowisku.
Wymagane pakiety systemowe:

```sh
sudo apt install build-essential pkg-config \
  libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev \
  patchelf python3
```

Zainstaluj Tauri CLI 2.11.5:

```sh
cargo install tauri-cli --version 2.11.5 --locked
```

Zbuduj pakiet:

```sh
bash scripts/build-desktop-linux.sh
```

Wynik trafia do `release/desktop/linux-x64-YYYYMMDD-HHmmss/`.
Skrypt obsługuje pierwszą wersję wyłącznie dla x86_64 i tworzy pakiet `.deb`.

## Linux — serwer

Wariant serwerowy nie używa Tauri ani bibliotek graficznych.

Build przez Docker:

```sh
bash scripts/build-linux.sh
```

Lokalne testy i build bez Dockera:

```sh
cargo test --offline --all-targets
cargo build --offline --locked --release --bin ultimatepr-station
python3 tests/e2e.py target/debug/ultimatepr-station
```

## Kontrola licencji

Przed pakowaniem można ręcznie uruchomić:

```sh
python3 scripts/desktop-licenses.py
```

Skrypt tworzy noty zależności w `desktop/target/legal/`. Jeśli pozostanie plik
`MISSING-LICENSE-TEXTS.txt`, publiczne wydanie należy zatrzymać i uzupełnić
brakujące teksty licencji.

## Zmiana wersji i aktualizacja

Przed zmianą wersji zaktualizuj jednocześnie `Cargo.toml`, `desktop/Cargo.toml`
i `desktop/tauri.conf.json`. Zbuduj nową paczkę w świeżym katalogu i zachowaj
sumy SHA-256. Instalator Windows może zostać zainstalowany w tym samym miejscu;
nie usuwa danych użytkownika z `%LOCALAPPDATA%\UltimatePR Station`.

Przed aktualizacją wykonaj kopię katalogu danych i zamknij aplikację desktop.
Nie umieszczaj `data/`, sekretów ani katalogu `target/` w paczce instalacyjnej.
