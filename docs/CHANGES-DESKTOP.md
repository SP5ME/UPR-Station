# Zmiany: warianty desktop

## Zmiany robocze po 0.1.3-prealpha.1 (bez nowej paczki)

- Nazwy produktów: UPR Terminal (desktop) i UPR Station (serwer).
  Tytuły okien, zasobnik, pasek boczny i stopka odpowiadają wariantowi;
  instalator oraz skróty desktopu noszą nazwę UPR Terminal.
  Identyfikator Tauri, plik wykonywalny, ścieżki danych i klucz autostartu
  Windows pozostają bez zmian. Instalator obsługuje poprzedni katalog
  UltimatePR Station; rzeczywista aktualizacja wymaga jeszcze testu na Windows.
- Dźwięki zdarzeń działają niezależnie od centrum powiadomień i dymków.
  Panel WWW odczytuje osobną, ograniczoną do 100 wpisów kolejkę sygnałów.
  Kilka zdarzeń w jednym odświeżeniu daje jeden sygnał o największej wybranej
  liczbie dzwonków; późniejszy komunikat bez dźwięku nie wycisza wcześniejszego.
  Pierwsze otwarcie strony nie odtwarza historii. Desktop nie dubluje dźwięku
  przy włączonych dymkach; sam dźwięk działa także bez nich.
- Powiadomienia w panelu WWW: lokalny suwak głośności 0–100% i test dźwięku.
  Zwiększono maksymalny poziom sygnału (0,95 zamiast 0,48) i wydłużono
  dzwonek do 0,32 s, z utrzymaniem pełnego poziomu przed wyciszeniem.
  Zapis automatyczny w przeglądarce, bez zmiany konfiguracji serwera;
  kontrolka ukryta w aplikacji desktopowej, której dźwięk pozostaje bez zmian.
- Wiersz „Nowa stacja w MHeard” umieszczono między „Rozłączenie” a MailBox.
- Powiadomienia: opcjonalne zdarzenie „Nowa stacja w MHeard”, domyślnie wyłączone.
  Treść: „Odebrano nową stacje: ZNAK”. Wspólne wykrywanie dla wszystkich
  interfejsów i portów; ponowienie po 3 h bez odbioru lub usunięciu wpisu z listy.
  Odtworzenie listy i samo włączenie opcji nie generują powiadomień.
- MHeard: ostatni użyty digi w panelu terminala, osobna kolumna „Trasa do stacji”
  i rozwijany podgląd ostatniej ramki AX.25 (I/S/U, nie tylko UI).
  Rozwinięcia pozostają otwarte podczas odświeżania, ale wyjście z karty MHeard
  zwija wszystkie wpisy; długie trasy są zawijane.
- Połączenie z wpisu MHeard używa jego interfejsu i odwróconej kolejności
  wyłącznie digi oznaczonych jako użyte. Nie odwraca nieużytej części ścieżki.
- Podgląd ostatniej ramki jest tylko w pamięci (do 4096 znaków pola informacji
  na wpis, ze znacznikiem skrócenia). Format `mheard.dat`, retencja 3 h i limit
  256 wpisów pozostają bez zmian; po restarcie podgląd czeka na następny odbiór.
- Usunięto moduł Winlink z backendu, API, kart, ustawień i powiadomień.
  Usunięto także jego kodek LZHUF i narzędzia GRIB. Lokalny MailBox pozostaje.
- Znane starsze klucze konfiguracji są ignorowane i pomijane przy zapisie.
  Istniejące wiadomości i sekrety użytkownika nie są kasowane.
- Poniższe wpisy opisują historię wcześniejszych wersji, nie obecną dostępność funkcji.

## 0.0.6

- Poprawiono wybór trybu dekodowania ramek AX.25 przy równoległych silnikach
  terminala, MailBoxa i WL2K. Aktywna sesja terminala, w tym YAPP, nie jest
  już mylona z sesją WL2K używającą tego samego znaku stacji.
- Wydanie i instalator Windows mają numer `0.0.6`.

## 0.0.5

- Dodano zarządzanie plikami YAPP: lista z rozmiarem i datą, pobieranie,
  usuwanie, wyszukiwanie, sortowanie oraz podsumowanie zajętego miejsca.
- Dodano limit liczby przechowywanych plików YAPP (domyślnie 1000) i opcję
  automatycznego usuwania najstarszych plików po osiągnięciu limitu.
- Przy starcie usuwane są osierocone pliki tymczasowe transferu, a aktywnego
  transferu nie można usunąć z panelu ani przez API.
- Ustawienie katalogu pobierania YAPP jest używane wspólnie przez zapis,
  listę, pobieranie i usuwanie plików.
- Wydanie i instalator Windows mają numer `0.0.5`.

## 0.0.4

- Rozdzielono ustawienia „WWW i dostęp” od ustawień „Aplikacja”.
- Dodano czytelne opisy obecności operatora, profilu nieobecności i limitu
  heartbeat oraz poprawiono opisy logowania i tokenu.
- Dodano brakujące tłumaczenia nazw zakładek PL/EN i uporządkowano układ pól.
- Przy zamykaniu backend wysyła bezpośredni AX.25 `DISC` do aktywnych sesji,
  zamiast opóźniać rozłączenie przez szablon pożegnania.

## 0.0.3

- Scalono desktop i serwer w jeden wspólny model dostępu, konfiguracji WWW i
  obecności operatora. Desktop nie wstrzykuje już osobnego tokenu sesji.
- Nowe instalacje mają wyłączone logowanie; można je włączyć w ustawieniach.
  Istniejący serwer z plikiem `operator.account` zachowuje dotychczasową ochronę,
  a istniejący desktop pozostaje lokalny i otwarty.
- Dodano tryb obecności `Auto`, `Zawsze dostępny` i `Nie przeszkadzać`, limit
  heartbeat panelu oraz cztery profile zachowania połączeń przychodzących.
  Profil zamkniętej stacji odrzuca nowe połączenia DM, pozostawiając MailBox,
  digipeater, beacon i WL2K.
- Dodano przełącznik „Nie przeszkadzać” do menu i zasobnika desktopu.
- Wydanie i instalator Windows mają numer `0.0.3`.

## 0.0.2

- Przywrócono pełną konfigurację WL2K w osobnym oknie otwieranym z karty WL2K;
  główne ustawienia zawierają tylko przełącznik włączenia usługi.
- Ujednolicono numer wersji aplikacji, backendu, desktopu, panelu i instalatora
  do `0.0.2`.
- Ujednolicono oprawę kart panelu WWW, widoków dynamicznych, tabel, formularzy,
  stanów pustych i responsywności bez zmiany identyfikatorów ani API.

## Ustawienia stacji

- Uporządkowano ustawienia w poziomych, zawijanych zakładkach zajmujących całą
  szerokość panelu.
- Rozdzielono istniejące opcje na: stację, interfejsy TNC, AX.25 i ruch,
  komunikaty i komendy, beacon, MailBox, YAPP, WL2K/Winlink oraz WWW i dostęp.
- Dodano opcjonalny główny lokator QTH stacji z walidacją, zapisem w konfiguracji
  i zmienną szablonów `{mylocator}`.
- Konfigurację WL2K przeniesiono do głównych ustawień; przy pustym lokatorze
  WL2K używany jest lokator stacji.
- Zachowano wspólny zapis, import/eksport, podgląd konfiguracji, szablony,
  obsługę desktopu bez logowania oraz konto operatora w trybie serwerowym.

- Dodano osobny program desktop dla Windows i Linux, korzystający z tego samego
  backendu i panelu co serwer.
- Dodano zasobnik, przywracanie jednej instancji, opcjonalny autostart i
  powiadomienia generowane przez backend niezależnie od panelu.
- Dane desktopu są przechowywane poza katalogiem instalacji, w kanonicznym profilu
  `%APPDATA%\\UltimatePR Station`. Przy pierwszym uruchomieniu po aktualizacji
  profil z wcześniejszej lokalizacji `%LOCALAPPDATA%\\UltimatePR Station` jest
  migrowany bez usuwania źródła. Wybrany jest nowszy poprawny `station.conf`,
  a istniejący plik docelowy otrzymuje kopię `.bak`; brakujące pliki użytkownika
  są kopiowane, natomiast cache i log nie są przenoszone.
- Jawne opcje `--config` i `--web-config` pozostają nadrzędne i nie uruchamiają
  automatycznej migracji. Niepoprawna konfiguracja zatrzymuje start z komunikatem
  wskazującym problem, zamiast uruchamiać aplikację z wartościami domyślnymi.
- Wariant serwerowy zachowuje CLI, API i formaty danych. Brak migracji schematu.
- systemd i Compose mają czas zatrzymania dopasowany do wygaszania sesji.
- Dodano skrypty pakowania, instrukcję użytkownika, architekturę i macierz testów.

Zakres pierwszego wydania: lokalny desktop x64 i dotychczasowy Linux server.
Wydanie pozostaje testowe do zamknięcia listy odbioru w VALIDATION.md.

Możliwe skutki uboczne: wymagane biblioteki GUI, różnice zasobnika i powiadomień
między pulpitami, krótka przerwa przy aktualizacji oraz inna domyślna lokalizacja
danych desktopu. Wersja serwerowa nie otrzymuje zależności GUI.
