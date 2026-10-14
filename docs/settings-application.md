# Zastosowanie zmian ustawień

Główny zapis sprawdza wpływ zmian na backendzie (`config/check`). Ta sama
klasyfikacja obowiązuje przy `config/save`, także dla żądań spoza formularza.

Bez odtwarzania stacji i bez rozłączania sesji stosowane są: powiadomienia,
Telegram, język, imię operatora i lokator, powitanie/pożegnanie/komendy,
beacon i jego trasy, digipeater, akceptowanie nowych połączeń, opisy i priorytety
interfejsów, automatyczny odbiór YAPP, pytanie o miejsce zapisu, limity plików
i ustawienie sprzątania. Zachowane zostają transporty TNC, sesje, monitor,
liczniki, czas startu stacji i aktywne transfery. Zmiana okresu lub włączenia
beaconu przelicza termin następnej emisji; inne zmiany nie przesuwają terminu.

Dotychczasowa ścieżka odtworzenia stacji pozostaje dla zmiany znaku, parametrów
AX.25 (w tym XID/modulo 128), struktury lub transportu interfejsów, włączenia
MailBox/YAPP, znaku MailBoxa i katalogu YAPP. Przy aktywnych sesjach wymaga
potwierdzonego rozłączenia. Nie jest to restart procesu systemowego.

Zmiany WWW i MQTT nadal podlegają istniejącym wymaganiom restartu procesu.

Moduł Winlink został usunięty z kodu, interfejsu i powiadomień.
Znane wpisy `wl2k_*` oraz dawne ustawienia `notification_winlink_*` są przy
odczycie ignorowane i pomijane przy zapisie. Zapis konfiguracji wykonuje
standardową kopię poprzedniego pliku. Istniejące wiadomości i pliki haseł
nie są otwierane przez stację ani kasowane. Lokalny MailBox działa niezależnie.
