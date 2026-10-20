# Narzędzia UPR Station

- `verify-publication.py`: skan plików przeznaczonych do Git, bez ignorowanych danych lokalnych.
- `download-release.py`: pobieranie wyłącznie z SP5ME/UPR-Station, kontrola SHA-256 i przekierowań.
- `emit-installer.py`: generowanie samodzielnego install.sh z lokalnego szablonu.
- `finalize-release.py`: sprawdzenie pełnego zestawu plików tego produktu i sum kontrolnych.
- `desktop-licenses.py`: zebranie not zależności z Cargo (nazwa historyczna).

`package-server.py` tworzy Linux tar.gz i Alpine ZIP z gotowej binarki musl.
`build-linux.sh` używa Dockerfile tego repozytorium do budowania serwera.

[Budowanie](../README.md), [instalacja Linux](../docs/LINUX-INSTALL.md).
