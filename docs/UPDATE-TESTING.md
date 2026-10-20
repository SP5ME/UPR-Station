# Aktualizacje Station — zmiany i zakres testów

## Zmiany

- WWW → Aktualizacja: kanał stabilny/testowy, informacje o buildzie,
  sprawdzanie GitHub i pobieranie gotowego binarnego ZIP-a z SP5ME/UPR-Station.
- Sprawdzanie wersji i statusu korzysta ze zwykłych zasad dostępu do panelu:
  przy publicznym podglądzie nie otwiera dodatkowego okna logowania.
  Pobieranie paczki, przesyłanie ZIP-a, instalacja oraz zmiana konta wymagają
  logowania operatora także przy publicznym podglądzie.
- Pobieranie poza pętlą radiową, limit rozmiaru/czasu, weryfikacja publicznej
  sumy SHA-256, rozmiaru i zgodności manifestu z wybranym buildem.
- Manualny ZIP nadal dostępny. Nie są już przyjmowane paczki źródłowe do
  kompilowania jako root. Helper przed zatrzymaniem usługi sprawdza produkt,
  architekturę ELF, manifest, komplet sum, ścieżki, typy i limity plików ZIP.
- Blokada równoległych operacji i wspólna blokada z instalatorem natywnym.
- OpenRC: niezależna grupa procesu helpera, kontrolowane procesy tej instalacji.
  systemd: osobna usługa/path updatera; NoNewPrivileges aplikacji pozostaje włączone.
- Kontrola odpowiedzi /healthz oraz tożsamości uruchomionej binarki. Błąd po
  zatrzymaniu powoduje próbę przywrócenia poprzedniego wydania. Konfiguracja,
  konto, MHeard, MailBox i pliki użytkownika nie są kopiowane ani usuwane.
- Automatyczne odświeżenie karty po sukcesie aktualizacji uruchomionej z nowego
  panelu; Ctrl+F5 / Ctrl+Shift+R pozostają instrukcją awaryjną.
  Restart unieważnia sesję logowania zapisaną w pamięci; panel może poprosić
  o ponowne zalogowanie. Konto i hasło pozostają niezmienione.
- Docker aktualizuje się wyłącznie z hosta; aplikacja nie dostaje socketu Docker.
- Bez zmiany wersji 0.1.3-prealpha.1, schematu danych i logiki AX.25/KISS.

## Kontrola lokalna

Testy źródłowe Rust (protokoły, konfiguracja, runtime i autoryzacja), testy
JS panelu i testy pakowania są uruchamiane przed przekazaniem paczki.
Nowe testy offline obejmują kanały wydań, sumy SHA-256, ograniczenie źródła
assetów, ZIP traversal/symlinki/duplikaty, złą architekturę i uszkodzone pliki.
Testy helpera używają atrap OpenRC/systemd i rollbacku — nie zatrzymują prawdziwej
usługi. Testy pakowania używają syntetycznego ELF i nie zastępują testu wykonania.

Ta lokalna kompilacja Linux jest kompilacją krzyżową Windows → musl.
Nie oznacza sprawdzenia pełnego upgrade/rollback na działającym Linux/OpenRC
lub systemd. Pierwszy test wdrożenia wykonaj na kopii/odizolowanej instalacji,
po wykonaniu kopii danych. Szczególnie sprawdź stare helpery i niestandardowy port.
Nie wykonywano instalacji na kontenerze użytkownika, push ani publikacji wydania.

## Ważne przy przejściu ze starej paczki

Pierwszą aktualizację obsługuje jeszcze STARY helper, z jego dotychczasowymi
ograniczeniami. Wewnętrzny ZIP zachowuje zgodny układ (Cargo.toml, binarka i
manifest w katalogu głównym), ale nie zawiera źródeł do kompilacji.
Nowy helper jest samowystarczalny i zostaje skopiowany po udanym starcie.
Jeśli stary helper nie przechodzi kontroli panelu lub nie ma obsługi systemd,
użyj lokalnego AKTUALIZUJ-STATION.sh z rozpakowanej paczki testera jako root.
Python 3 i wget muszą być dostępne na serwerze.

Sumy kontrolne wykrywają uszkodzenie; nie zastępują zaufania do autora paczki.
Nie wgrywaj ZIP-ów z nieznanych źródeł. Instalacja oprogramowania jest operacją
uprzywilejowaną, dlatego konto operatora powinno mieć własne silne hasło.
