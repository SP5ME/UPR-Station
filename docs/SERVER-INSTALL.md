# UPR Station — instalacja serwera

Pakiet dotyczy serwera UPR Station, nie desktopu UPR Terminal.
Nie wymaga kreatora Proxmox i nie zawiera skryptów tworzenia kontenerów Proxmox.

## Obsługiwane warianty

- Alpine/OpenRC: x86-64, ARM64, ARMv7.
- Debian/Ubuntu i pochodne z systemd: x86-64, ARM64, ARMv7.
- Raspberry Pi OS (dawniej Raspbian): ARM64 dla systemu 64-bitowego,
  ARMv7 dla 32-bitowego. Raspberry Pi z ARMv6 nie jest obsługiwane.
- Docker: obrazy dla tych samych trzech architektur, jako sprawdzane tar.gz.

Statyczna binarka musl jest wspólna dla Linux/Alpine; usługi i zależności
dobiera skrypt. Nie deklarujemy obsługi każdej dystrybucji Linux.

## Jedno polecenie po pobraniu skryptu

Z Releases pobierz `install-server.sh`, sprawdź jego SHA-256 i uruchom:

```sh
sudo sh install-server.sh --version TAG_WYDANIA
```

Na Alpine jako root nie dodawaj sudo. Wydanie zawiera samodzielny skrypt,
który rozpoznaje system/architekturę i pobiera gotową binarkę z tego samego tagu.
Nie kompiluje aplikacji na Raspberry Pi. Przy błędzie sumy kontrolnej lub
nieobsługiwanym systemie nie uruchamia instalacji.

Repozytorium prywatne wymaga tokenu GitHub z prawem odczytu Contents.
Nie zapisuj tokenu w skrypcie i nie umieszczaj go w historii poleceń.
Jeśli GITHUB_TOKEN jest już bezpiecznie ustawiony w środowisku użytkownika:

```sh
sudo --preserve-env=GITHUB_TOKEN sh install-server.sh --version TAG_WYDANIA
```

Przy publicznym repozytorium token nie jest wymagany. Nie zmieniaj widoczności
repozytorium bez zakończenia audytu licencji.

Skrypt lokalnie instaluje wymagane narzędzia z repozytoriów dystrybucji,
sprawdza paczkę i uruchomienie usługi. Istniejące konfiguracje i dane zachowuje.
Nierozpoznana stara instalacja systemd wymaga ręcznej migracji — skrypt jej
nie nadpisuje. Uaktualnienia OpenRC używają istniejącego mechanizmu rollback.

Domyślnie panel nasłuchuje lokalnie na 127.0.0.1:8080. Początkowe konto
admin/packet należy zmienić przed udostępnieniem panelu w sieci.
`--listen` służy do świadomego wyboru adresu przy świeżej instalacji;
zapisana konfiguracja ma pierwszeństwo podczas aktualizacji.
Po aktualizacji odśwież przeglądarkę przez Ctrl+F5.

## Docker

Docker musi być już zainstalowany i uruchomiony. Instalator nie wykonuje
zewnętrznego skryptu instalacji silnika Docker.

```sh
sudo sh install-server.sh --version TAG_WYDANIA --docker
```

Kontener: `upr-station`, wolumen: `upr-station-data`. Aktualizacja nie usuwa
wolumenu. Poprzedni kontener pozostaje zatrzymany jako backup. Jeśli natywna
usługa działa, zatrzymaj ją świadomie przed przejściem na Docker. Własny
kontener o takiej samej nazwie nie jest nadpisywany.

Obsługa USB/Serial w Docker wymaga osobnego, świadomego przekazania urządzenia;
instalator domyślnie nie udostępnia kontenerowi urządzeń ani trybu privileged.

## Aktualizacja przez panel Alpine

Wybierz `UPR-Station-TAG-alpine-ARCH.zip`, zgodny z architekturą działającego
systemu. Nie wybieraj paczki Docker, desktopu ani tar.gz. ZIP zawiera gotową
binarkę, manifest i helper; nie wymaga Cargo na kontenerze.

## Weryfikacja

CI sprawdza formaty paczek i uruchomienie binarek w obrazach odpowiednich
architektur. Testy emulowane nie potwierdzają działania radia, urządzeń USB,
pełnej instalacji OpenRC/systemd ani aktualizacji istniejącej konfiguracji.
