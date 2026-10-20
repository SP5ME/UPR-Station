# Instalacja i aktualizacja UPR Station na Linux

Pobierz `install.sh` z wydania **SP5ME/UPR-Station**. Skrypt z drugiego produktu
nie jest zamiennikiem. Uruchom: `sudo sh install.sh`.
Domyślnie wybiera najnowsze opublikowane wydanie (także pre-release),
`--version TAG` przypina tag. Przed wykonaniem możesz przeczytać skrypt.

Obsługiwane: Alpine/OpenRC i Debian/Ubuntu/Raspberry Pi OS/systemd,
x86-64, ARM64, ARMv7 (bez ARMv6). `--docker` wymaga już działającego Dockera.
`--listen IP:PORT` ustawia adres; domyślnie panel jest lokalny.
Alpine można aktualizować także przez panel WWW odpowiednim ZIP-em.
Szczegóły i kopie danych: [SERVER-INSTALL.md](SERVER-INSTALL.md).

Po upublicznieniu repozytorium (zastąp TAG właściwym tagiem):

```sh
curl -fsSL https://github.com/SP5ME/UPR-Station/releases/download/TAG/install.sh | sudo sh
```

To polecenie wykonuje pobrany kod. Prywatne repo wymaga zalogowanego GitHub CLI
lub uwierzytelnionego pobrania. Skrypt pobierania paczek odczytuje GITHUB_TOKEN
(lub GH_TOKEN) ze środowiska i sprawdza SHA-256; nie podawaj sekretu w argumentach.
Nowe repo Station nie zostało utworzone ani opublikowane w ramach podziału.
