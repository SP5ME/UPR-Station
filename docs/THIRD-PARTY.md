# Materiały osób trzecich

Własny kod deklaruje Apache 2.0. Zależności nie zmieniają swojej licencji
na Apache tylko dlatego, że znajdują się w tej aplikacji.

## Ikony

Panel używa ikon Material Design Icons / Pictogrammers. Do dystrybucji
dołączamy `licenses/MDI-LICENSE.txt` i `licenses/Apache-2.0.txt`.
Źródło licencji: https://pictogrammers.com/docs/general/license/

## Rust

Dokładne wersje wynikają z Cargo.lock.
`python scripts/desktop-licenses.py` zbiera teksty licencji dla docelowej
platformy do target/server-licenses/. Paczki serwera dołączają te noty.
Nie buduj wydania, jeśli pozostaje MISSING-LICENSE-TEXTS.txt.

Obecność tekstu licencji nie dowodzi spełnienia wszystkich jej warunków.
W szczególności sprawdź sposób udostępnienia źródeł komponentów MPL 2.0
i wymagane noty dla każdego dystrybuowanego wariantu/platformy.

## Pochodzenie kodu

Pełna lista wykorzystanych źródeł i zapożyczeń wymaga potwierdzenia przez
opiekuna. Nie deklarujemy zakończonego audytu. Przepisanie cudzego kodu
na inny język nie znosi obowiązków jego oryginalnej licencji.
Usunięty moduł Winlink nie należy do bieżącego zestawu źródeł.

Źródła: https://www.apache.org/licenses/LICENSE-2.0 oraz
https://www.mozilla.org/en-US/MPL/2.0/FAQ/
