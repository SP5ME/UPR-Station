# Warianty UltimatePR Station

Station korzysta z jednego rdzenia Rust i jednego panelu WWW. Program serwerowy
`ultimatepr-station` działa bez GUI. Osobny `ultimatepr-station-desktop` uruchamia
lokalną stację i wyświetla ten sam panel we własnym oknie.

Docelowa macierz v1: Windows 11 x64, Ubuntu 24.04 LTS x64, Debian 12 x64.
Serwer zachowuje dotychczasowy build Linux amd64/arm64. Jest to zakres testów,
a nie deklaracja, że wszystkie platformy zostały już zweryfikowane.
Aktualny stan w [VALIDATION.md](VALIDATION.md).

## Desktop

Windows: instalator NSIS instaluje aplikację dla bieżącego użytkownika, tworzy
wpis w menu Start i pozwala utworzyć skrót. Wymagane jest środowisko WebView2.
Instalator może wymagać internetu do przygotowania zależności. Nie dostarczamy
kompletnego instalatora offline ani automatycznych aktualizacji.

Linux: zainstaluj pakiet przez `sudo apt install ./ultimatepr-station-desktop*.deb`.
APT rozwiązuje zależności systemowe, w tym WebKitGTK. Uruchom aplikację z menu
środowiska graficznego. Skrót na samym pulpicie zależy od środowiska. Dostęp do
portu szeregowego wymaga uprawnień użytkownika (zwykle grupa `dialout` i ponowne
zalogowanie). Nie uruchamiaj desktopu jako root.

Na Linux desktop dostępny jest także opcjonalny tryb serwerowy uruchamiany z
terminala. Nie otwiera on okna Tauri, ale uruchamia ten sam backend i panel WWW:

```sh
ultimatepr-station-desktop --mode server
```

Równoważny alias to `--headless`. Tryb ten jest dodatkiem dla użytkownika, który
świadomie chce uruchomić wariant serwerowy na komputerze z graficznym Linuxem;
zwykłe uruchomienie aplikacji nadal otwiera desktop. W tym trybie obowiązują
ustawienia serwera z `web.conf`, a domyślny adres panelu to `127.0.0.1:8080`.
Jeżeli panel ma być dostępny z innego komputera, ustaw adres nasłuchu zgodnie z
konfiguracją sieci i zabezpiecz dostęp operatora.

Po pierwszym uruchomieniu ustaw znak stacji i interfejs TNC w panelu. Bez znaku
aplikacja nie rozpoczyna pracy radiowej. Desktop obsługuje lokalną stację;
zdalny serwer otwieraj w zwykłej przeglądarce.

Minimalizacja pozostawia okno na pasku zadań. Krzyżyk chowa je do zasobnika,
o ile zasobnik jest dostępny. **Stacja nadal wtedy pracuje.** Menu aplikacji
pozwala otworzyć okno i zakończyć pracę. Na Linuksie bez potwierdzonego zasobnika
zamknięcie wymaga wyboru: pozostawienie okna lub zakończenie.

Powiadomienia obejmują połączenia przychodzące, nową pocztę, zakończenie odbioru
YAPP i utratę połączenia z TNC. Kategorie można wyłączyć. Powiadomienia nie
zawierają treści poczty. Nieobecna usługa powiadomień i tryb „Nie przeszkadzać”
mogą uniemożliwić ich pokazanie; nie zatrzymuje to stacji.

Autostart jest domyślnie wyłączony. Włączenie oznacza start po zalogowaniu,
a nie usługę działającą przed logowaniem. Przy autostarcie aplikacja chowa okno
tylko wtedy, gdy potwierdzono dostępność zasobnika.

## Dane i konfiguracja

Domyślny katalog desktopu:

- Windows: `%LOCALAPPDATA%/UltimatePR Station`;
- Linux: `$XDG_DATA_HOME/ultimatepr-station`, a przy braku tej zmiennej
  `~/.local/share/ultimatepr-station`.

Zawiera `station.conf`, `web.conf`, pocztę, pliki, sekrety oraz `desktop.conf`
z preferencjami desktopu i `desktop.log` z błędami integracji systemowej.
Nie publikuj całego katalogu danych ani nie włączaj go do paczek wydaniowych.
Formaty danych radiowych pozostają zgodne z serwerem.

Desktop przyjmuje `--config PATH` i `--web-config PATH`; do migracji używaj ścieżek
bezwzględnych. Parametr `--demo` używa odrębnego katalogu demonstracyjnego.
Nie łącz demo z katalogiem danych rzeczywistej stacji.

Desktop wiąże HTTP wyłącznie z loopbackiem na przydzielonym porcie i stosuje
token na czas uruchomienia. Ustawienia nasłuchu w `web.conf` dotyczą wariantu
serwerowego; nie otwierają desktopu na sieć. Token desktopu nie jest zapisywany
w konfiguracji, URL ani logu. Panel korzysta z niego automatycznie.

## Migracja, kopia i powrót

1. Zatrzymaj poprzednią stację. Dla serwera użyj Ctrl+C lub zatrzymania usługi;
   dla desktopu wybierz „Zakończ”. Wygaszanie sesji może trwać do 12 sekund.
2. Skopiuj cały katalog danych, w tym pocztę, pliki i sekrety, do bezpiecznej
   lokalizacji. Zachowaj ograniczone uprawnienia dostępu. Kopia na drugim
   nośniku chroni dodatkowo przed awarią dysku.
3. Przenieś pełny katalog do docelowej lokalizacji desktopu albo uruchom desktop
   z jawnymi ścieżkami. Nie nakładaj danych na już używaną stację. Ścieżki
   bezwzględne zapisane w konfiguracji sprawdź osobno, szczególnie przy zmianie OS.
4. Sprawdź znak, TNC, pocztę i konfigurację. Nigdy nie uruchamiaj jednocześnie
   dwóch stacji na tych samych danych lub tym samym fizycznym TNC.

Aktualizuj po zakończeniu aplikacji i wykonaniu kopii danych. Instalator nie
powinien usuwać danych operatora. Przed odinstalowaniem wyłącz autostart
w **Ustawienia → Aplikacja**. Ta zakładka dotyczy wyłącznie UPR Terminal;
autostart zapisuje się automatycznie, bez restartu i bez przycisku zapisu stacji.
Zakładka **Powiadomienia** jest wspólna: Terminal udostępnia dymki systemowe,
a Station — Telegram. Konfiguracja i token Telegrama pozostają zachowane
w danych desktopu, ale klient i endpointy Telegrama są w nim nieaktywne.
Przy powrocie zachowaj osobno bieżące dane, zainstaluj poprzednią wersję i
odtwórz jej kopię do pustego katalogu. Nie mieszaj plików różnych kopii.

## Linux server

Dotychczasowe argumenty CLI i API pozostają zachowane. Domyślny nasłuch to
`127.0.0.1:8080`. Przy pierwszym uruchomieniu serwer tworzy konto `admin` z
hasłem `packet`. Zmiana danych konta nie jest wymuszana przy pierwszym logowaniu. Możesz nadal ustawić
`STATION_TOKEN` lub `token_file` jako zgodnościowy dostęp Bearer API, bez wpisywania
go do radiowego `station.conf`. Zwykłe HTTP nie szyfruje transmisji: nie wystawiaj
go bezpośrednio w niezaufanej sieci; korzystaj z zabezpieczonego dostępu sieciowego.

Przykład terminalowy:

```sh
./ultimatepr-station --config /var/lib/ultimatepr/station.conf --web-config /var/lib/ultimatepr/web.conf
```

Jednostka `deploy/ultimatepr-station.service` zakłada konto `ultimatepr`, program
w `/usr/local/bin/ultimatepr-station` i zapisywalny dla tego konta katalog
`/var/lib/ultimatepr`. Utwórz konto i katalog przed instalacją jednostki do
`/etc/systemd/system/`. Następnie wykonaj `systemctl daemon-reload` i
`systemctl enable --now ultimatepr-station`. Stan: `systemctl status
ultimatepr-station`; log: `journalctl -u ultimatepr-station`; zatrzymanie:
`systemctl stop ultimatepr-station`. Dostęp do TNC przydziel temu samemu kontu.

Docker zachowuje `Dockerfile` i `docker-compose.yml`. Ustaw `STATION_TOKEN`,
uruchom `docker compose up -d --build`, a dane zachowaj na trwałym wolumenie.
Port szeregowy wymaga przekazania urządzenia. Zatrzymuj kontener z czasem
co najmniej 20 sekund (`docker compose stop -t 20`). Przed kopią wolumenu
zatrzymaj kontener. Hasła można przekazywać przez pliki sekretów.

## Diagnostyka

- Brak okna: sprawdź WebView2/GTK/WebKitGTK i `desktop.log`.
- Brak zasobnika: GNOME może wymagać rozszerzenia; bez niego pozostaw okno
  dostępne. Zbudowanie ikony nie jest samo w sobie dowodem, że pulpit ją pokazuje.
- Brak powiadomień: sprawdź ustawienia kategorii, powiadomień systemowych i
  tryb „Nie przeszkadzać”; na Windows testuj również wersję zainstalowaną.
- Blokada danych: zamknij drugą instancję. Nie obchodź blokady usuwaniem pliku
  blokady podczas pracy stacji.
- Brak TNC: sprawdź endpoint, urządzenie, uprawnienia i inny proces zajmujący port.
- Zajęty port serwera: wybierz poprawny wolny adres przez `--bind` lub `web.conf`.

Do zgłoszenia dołącz wersję, OS, środowisko pulpitu, kroki odtworzenia i wybrane
komunikaty błędu. Usuń tokeny, hasła, treść poczty i dane operatora. Wymuszone
wyłączenie systemu nie daje gwarancji zakończenia transmisji.
