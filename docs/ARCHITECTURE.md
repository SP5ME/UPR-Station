# Architektura UPR Station

`src/` zawiera własny silnik AX.25/KISS, transport TCP/Serial, MailBox,
YAPP, MHeard, statystyki i trwałą konfigurację. `web/` zawiera własny interfejs
HTML/CSS/JS. `languages/` i `messages/` są zasobami tego projektu.
Nie ma wspólnego katalogu źródeł, submodułu ani zależności od drugiego produktu.

`src/main.rs` uruchamia serwer z `runtime::RuntimeOptions::from_cli`.
OpenRC/systemd/Docker uruchamiają ten sam program `ultimatepr-station`.
HTTP obsługuje panel, autoryzację i lokalne API poleceń. Pętla radiowa działa
niezależnie od otwarcia przeglądarki. Telegram i MQTT startują w runtime;
konfiguracja decyduje o aktywnym korzystaniu z tych usług.
Projekt nie zawiera Tauri ani natywnego hosta powiadomień/autostartu.

Zapisy konfiguracji korzystają z istniejących parserów, walidacji i atomowego
zapisu z kopią. Ustawienia radiowe wymagające przebudowy nadal chronią aktywne
sesje; preferencje powiadomień nie przebudowują transportu.
Nie zmieniono schematów ani numeru wersji.

[Granice podziału i kompatybilność](PROJECT-SPLIT.md).
