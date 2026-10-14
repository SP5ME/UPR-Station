# UltimatePR Station — plan wdrażania pre-alfa

Dokument opisuje kolejność prac dla wersji testowej. Nie zmienia numeru wersji,
formatu konfiguracji ani procedury wydawania. Każdy etap powinien być osobno
sprawdzony przed rozpoczęciem następnego.

## Etap 0 — stabilna baza

Status: baza robocza `0.1.0-prealpha.1`.

- wspólny backend Rust i panel WWW;
- desktop Tauri oraz wariant server bez GUI;
- istniejące pliki konfiguracji i dane operatora pozostają kompatybilne;
- brak automatycznego publikowania, podpisywania i aktualizacji desktopu.

## Etap 1 — kanały powiadomień

Status: wdrożone i sprawdzone składniowo.

- Centrum powiadomień w panelu;
- osobne kanały: centrum, dymek, Telegram i dźwięk;
- kliknięcie nazwy kanału wysyła test;
- dźwięk desktopowy korzysta z systemu, a panel WWW z Web Audio po interakcji
  użytkownika;
- brak odbiornika desktopowego w wariancie server jest zgłaszany jako kontrolowany
  błąd, a nie traktowany jako działające powiadomienie systemowe.

## Etap 2 — rozdzielenie środowisk

Status: wdrożone; kontrakt runtime ma testy dla obu profili.

- backend zwraca profil `desktop` albo `server`;
- ustawienia WWW mogą być ukryte w desktopie i pozostają dostępne w serverze;
- Linux desktop może zostać uruchomiony opcjonalnie jako server przez
  `--mode server` albo `--headless`;
- domyślne uruchomienie desktopu nie zmienia się.

## Etap 3 — Alpine/OpenRC

Status: backend aktualizacji i skrypty przygotowane, wymagają testu na rzeczywistym Alpine.

- instalator tworzy konto `ultimatepr`;
- aktywna wersja jest wskazywana przez `/opt/ultimatepr-station/current`;
- usługa OpenRC jest instalowana, dodawana do autostartu i uruchamiana;
- instalator potwierdza `/healthz`, zanim pokaże komunikat o pomyślnej instalacji;
- `verify-service.sh` sprawdza usługę, binarkę i `/healthz`;
- aktualizacja używa osobnego helpera i nie powinna nadpisywać aktywnej binarki.
- API `update/status` i `update/upload` zapisuje paczkę oraz status w katalogu
  danych konfiguracji; uruchomienie instalacji pozostaje po stronie helpera Alpine.
- Zakładka WWW zawiera zwinięty panel aktualizacji z wyborem ZIP-a, potwierdzeniem
  instalacji i statusem etapów.

Warunek przejścia: uruchomić na Alpine kompilację, instalację, restart systemu,
`rc-service ultimatepr-station status` oraz `verify-service.sh`.

## Etap 4 — stabilizacja bieżącej pre-alfa

Status: następny etap po potwierdzeniu działania na Alpine.

- potwierdzić, że aktywna usługa OpenRC rzeczywiście utrzymuje proces i panel;
- sprawdzić po restarcie systemu panel, konfigurację, MHeard i logi;
- sprawdzić aktualizację ZIP: walidację, zatrzymanie usługi, uruchomienie nowej
  wersji oraz zachowanie konfiguracji i danych;
- usunąć wyłącznie potwierdzone problemy z aktualnym przepływem aktualizacji;
- nie zmieniać formatu konfiguracji ani numeru wersji `0.1.0-prealpha.1`.

Ten etap kończy się dopiero po ręcznym potwierdzeniu działania na rzeczywistym
Alpine. Nie należy na tym etapie budować jeszcze wydania dla testerów ani
przenosić zmian do instalatora Windows.

## Etap 5 — przygotowanie wydań platformowych

Status: planowany po stabilizacji etapu 4.

- przygotować osobną paczkę transferową Alpine z instrukcją budowy i instalacji;
- osobno zbudować i ręcznie sprawdzić instalator Windows desktop;
- sprawdzić panel z drugiego komputera oraz dźwięk WWW po zgodzie przeglądarki;
- opisać różnice między desktopem Windows, desktopem Linux i serverem Linux;
- zachować jedną wspólną konfigurację i backend, bez kopiowania aplikacji.

Wydania desktopowe będą aktualizowane przez ponowną instalację odpowiedniej
paczki. Wariant server będzie aktualizowany przez zweryfikowaną paczkę ZIP i
mechanizm usługi systemowej.

## Etap 6 — opcjonalny tryb „Linux desktop jako server”

Status: dodatek, nie warunek wydania pre-alfa.

- umożliwić użytkownikowi uruchomienie backendu w trybie server na Linux desktop;
- pozostawić domyślny tryb Linux desktop bez zmian, z aplikacją Tauri i lokalnym
  oknem;
- użyć jawnego parametru uruchomieniowego lub skryptu, bez zmiany konfiguracji;
- panel będzie wtedy dostępny z przeglądarki pod adresem serwera, domyślnie na
  porcie `8080`;
- ustawienia desktopowe niedostępne bez środowiska graficznego pozostaną
  nieaktywne lub będą oznaczone jako niedostępne.

Ten etap można realizować dopiero po ustabilizowaniu wariantu server Linux.
Nie powinien blokować testów Windows, Alpine ani podstawowego desktopu Linux.

## Etap 7 — kolejne systemy i wydanie dla testerów

Status: przyszły, poza bieżącą pre-alfa.

- dodać kolejne dystrybucje Linux według tej samej macierzy desktop/server;
- rozważyć Raspberry Pi w wariancie desktop i server;
- przygotować końcowe pakiety dla testerów dopiero po przejściu checklisty
  odbioru dla każdej deklarowanej platformy;
- dopiero wtedy ustalić sposób dystrybucji, podpisywania i aktualizacji wydań.

Nie łączyć etapów 4–7 w jedną zmianę. Błąd na etapie Alpine wraca do skryptów
Alpine, błąd desktopu do jego paczki, a błąd wspólnego backendu do osobnego
testu regresji. Działające warianty pozostają bez zmian.
