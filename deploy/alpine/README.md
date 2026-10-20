# UltimatePR Station pre-alfa 0.1.1 — Alpine Linux server

Ta paczka buduje wariant serwerowy bez Tauri, WebView i bibliotek graficznych.
Po rozpakowaniu na Alpine uruchom `deploy/alpine/build-server-linux.sh`.
Po udanej kompilacji skrypt zapyta `Czy zainstalować aplikację teraz? [y/N]`.

## Instalacja na świeżym Alpine

1. Przenieś ZIP na Alpine i rozpakuj go, np. do `/opt/ultimatepr-build`.
2. Wejdź do katalogu paczki i nadaj skryptom prawa wykonywania:

```sh
chmod +x deploy/alpine/*.sh
```

3. Uruchom kompilację:

```sh
./deploy/alpine/build-server-linux.sh
```

Skrypt sprawdzi źródła, doinstaluje `build-base`, `rust`, `cargo` i
`ca-certificates`, wykona `cargo build --release --locked`, zapisze binarkę
i sumę SHA-256, a następnie zapyta o instalację.

Wpisz `y`, aby zainstalować binarkę do `/opt/ultimatepr-station`, konfigurację
do `/etc/ultimatepr` i dane do `/var/lib/ultimatepr`. Instalator utworzy konto
systemowe `ultimatepr`, skonfiguruje usługę OpenRC, doda ją do autostartu i
uruchomi ją w tle. Instalator doinstaluje `doas` tylko gdy jest potrzebne i
doda regułę pozwalającą kontu `ultimatepr` uruchamiać wyłącznie helper
`/usr/local/libexec/ultimatepr-update` jako root. Wpisz `n`, aby pozostawić tylko artefakt w
`artifacts/alpine-server`.

## Konfiguracja i uruchomienie

Uzupełnij własne dane:

```sh
vi /etc/ultimatepr/station.conf
```

Ustaw przede wszystkim `callsign` oraz właściwy `interface_*`. Przykłady są
neutralne i nie zawierają danych operatora.

Sprawdź usługę:

```sh
rc-service ultimatepr-station status
```

Panel będzie dostępny pod `http://ADRES_ALPINE:8080`. Plik `web.conf` włącza
uwierzytelnianie panelu; konto operatora należy ustawić przy pierwszym wejściu.
Log usługi znajduje się w `/var/log/ultimatepr-station.log`. Ręczne uruchomienie
poza OpenRC pozostaje możliwe przez `/opt/ultimatepr-station/ultimatepr-station-start`.

## Instalacja późniejsza

Jeśli podczas kompilacji wybrano `n`, można później wykonać:

```sh
./deploy/alpine/install-server-linux.sh artifacts/alpine-server/ultimatepr-station
```

Instalator odmawia nadpisania istniejącej instalacji wskazanej przez `current`.
Do działającej instalacji używaj procedury aktualizacji, aby zachować możliwość
rollbacku.

Przed utworzeniem plików aplikacji instalator instaluje przez `apk` trwałe
zależności `python3`, `wget` i `ca-certificates`, następnie sprawdza Python 3.9+
oraz moduły HTTPS/ZIP. Niepowodzenie przerywa instalację przed zmianą plików
Station. Pobranie brakujących pakietów wymaga dostępu do repozytoriów Alpine.
Aktualizacja samym ZIP-em nie uruchamia instalatora zależności: starszą instalację
bez Pythona trzeba jednorazowo uzupełnić przez `apk add --no-cache python3 wget ca-certificates`.

## Weryfikacja

```sh
./deploy/alpine/verify-package.sh
sha256sum -c artifacts/alpine-server/ultimatepr-station.sha256
./deploy/alpine/verify-service.sh
```

## Ograniczenia

- To wariant headless/server, nie desktop Tauri.
- Paczka jest przeznaczona dla Alpine/musl i buduje lokalnie na systemie docelowym.
- Pierwsza kompilacja wymaga Internetu do pobrania pakietów Alpine i crate'ów.
- Paczka nie zawiera katalogu `data/`, sekretów ani prywatnej konfiguracji.
- Usługa OpenRC korzysta z `/opt/ultimatepr-station/current`; aktualizacje
  powinny używać helpera `ultimatepr-update`, a nie nadpisywać aktywnej binarki.
