# Testy UPR Station

`cargo test --locked --tests`: protokoły radiowe, trwałość danych, konfiguracja,
powiadomienia i cykl życia procesu. Lokalny HTTP/KISS używa tylko loopback.
Testy tworzą własne katalogi, nie wymagają rzeczywistego TNC.
`node tests/browser/NAZWA.cjs`: pomocniki MHeard, dźwięku, nazw i ustawień produktu.
`python tests/packaging/release-download.py`: pobieranie testowane bez sieci z atrapami.
`bash tests/installers/bootstrap.sh`: wygenerowany instalator bez rzeczywistej instalacji.
`python tests/e2e.py target/debug/ultimatepr-station[.exe]`: HTTP/KISS/AX.25/YAPP/MailBox.
`python tests/packaging/server-package.py`: inwentarz paczek, manifest i uprawnienia plików.
`bash tests/installers/server-installer.sh` i `alpine-update.sh`: kontrakty instalatora/usługi.

Nie są to testy instalacji na każdej dystrybucji ani testy prawdziwego radia.
