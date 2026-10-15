# Łatwa instalacja i aktualizacja Linux

Każde nowe wydanie zawiera samodzielny `install.sh`. Nie używaj szablonu
z `deploy/install/`: gotowy plik jest generowany podczas budowania wydania.
Wymagane jest połączenie z GitHub i repozytoriami pakietów systemu.

## Wybór produktu

Po pobraniu skryptu z Releases:

```sh
sh install.sh --station
sh install.sh --terminal
```

Wykonaj tylko polecenie dla wybranego produktu. Ponowne uruchomienie służy
do aktualizacji; domyślnie pobierane jest najnowsze opublikowane wydanie,
**także pre-release**. Drafty są pomijane. `--version TAG` wybiera konkretny tag;
brak właściwej paczki zatrzymuje instalację, bez zastępowania jej innym wariantem.

Station obsługuje Alpine/OpenRC i Debian/Ubuntu/Raspberry Pi OS z systemd,
x86-64, ARM64 i ARMv7. `--docker` wybiera istniejący silnik Docker;
`--listen IP:PORT` dotyczy serwera. Usługa uruchamia się po instalacji.
Własna konfiguracja jest zachowywana przez istniejący instalator Station.

Terminal: Debian/Ubuntu i pochodne, tylko x86-64, format DEB. Uruchamiaj
instalator jako zwykły użytkownik. sudo jest potrzebne do instalacji pakietu,
ale nie do działania aplikacji. Zamknij Terminal, również w zasobniku,
przed aktualizacją. `--reinstall` pozwala aktualizować także paczki testowe
o niezmienionym numerze wersji. Instalator nie usuwa danych operatora.
Po instalacji Terminal jest uruchamiany w bieżącej sesji graficznej;
`--no-launch` pomija ten krok. Z sesji SSH bez środowiska graficznego skrypt
tylko instaluje pakiet. Nie deklarujemy automatycznej instalacji AppImage
ani obsługi wszystkich dystrybucji desktopowych.

Skrypt doinstaluje Python 3, jeśli go brakuje. Przy koncie bez sudo instaluj
Station jako root. Desktopu nie uruchamiaj jako root.
`--dry-run` sprawdza wybór produktu i platformy bez pobierania i zmian systemu;
nie jest testem dostępności wydania ani uprawnień do instalacji.

## Publiczne repozytorium: curl

Zastąp `TAG` tagiem wydania zawierającego `install.sh`:

```sh
curl -fsSL https://github.com/SP5ME/UPR-Terminal/releases/download/TAG/install.sh | sh -s -- --station
```

Dla desktopu zmień `--station` na `--terminal`. Ten bootstrap pochodzi
z wybranego tagu, ale domyślnie instaluje najnowsze wydanie. Aby je przypiąć,
dodaj `--version TAG`. Wymagany jest curl i zaufane certyfikaty CA.
Polecenie wykonuje kod z sieci; przed wykonaniem można zamiast tego pobrać
skrypt, sprawdzić jego SHA-256 z wydania i przeczytać zawartość.
Repozytorium pozostaje prywatne do zakończenia audytu licencji.

## Prywatne repozytorium

Anonimowy curl nie pobierze prywatnych załączników. Najprościej pobrać
`install.sh` przez zalogowaną przeglądarkę i uruchomić jak wyżej, przekazując
`GITHUB_TOKEN` w środowisku. Token wymaga prawa odczytu Contents tego repozytorium.
Nie zapisuj tokenu w skrypcie, historii poleceń ani argumentach procesu.

Jeśli masz już zainstalowany GitHub CLI i wykonane `gh auth login`, jedno
polecenie pobierze bootstrap do nowego katalogu tymczasowego i uruchomi Station:

```sh
sh -c 'd=$(mktemp -d) || exit 1; trap '\''rm -rf -- "$d"'\'' EXIT; gh release download TAG --repo SP5ME/UPR-Terminal --pattern install.sh --dir "$d" && GITHUB_TOKEN="$(gh auth token)" sh "$d/install.sh" --station'
```

Zastąp `TAG`, a dla desktopu także `--station` przez `--terminal`.
Token jest przekazywany w środowisku, nie w argumentach polecenia.
Station używa `sudo --preserve-env=GITHUB_TOKEN`, jeśli nie pracujesz jako root;
polityka sudo może odmówić zachowania tej zmiennej. Skrypt nie obchodzi tej polityki.

## Kontrole i ograniczenia

Przed wykonaniem instalatora Station albo instalacją DEB sprawdzana jest
suma SHA-256 pobranego pliku z tego samego wydania. Token GitHub nie jest
przekazywany przy przekierowaniu na inny host. SHA-256 chroni integralność;
nie zastępuje podpisu wydawcy ani zaufania do konta publikującego wydanie.
Nie ma usuwania konfiguracji, kreatora Proxmox ani automatycznego zabijania procesów.
Ograniczenia rollback i migracji Station opisuje [SERVER-INSTALL.md](SERVER-INSTALL.md).
Po aktualizacji panelu WWW odśwież kartę przez Ctrl+F5.
