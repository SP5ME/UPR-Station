# Przygotowanie publikacji UPR Terminal

Ten katalog to kontrolowana kopia publikacyjna, nie zsynchronizowane repozytorium.
Późniejsze zmiany w oryginale nie trafiają tu automatycznie. Przed wysłaniem
porównaj zmiany, zamiast publikować starszą kopię. Nie inicjalizuj Gita w katalogu
roboczym zawierającym dane, stare paczki i prywatne notatki.

## Stan

- Źródła i testy zachowane; katalogi wynikowe i dane operatora pominięte.
- CI sprawdza Windows/Linux. Osobny, ręcznie uruchamiany „Build release”
  buduje paczki Desktop/Server i publikuje draft albo pre-release po sukcesie wszystkich zadań.
- Początkowa kopia została przygotowana bez historii Git. Stan po pierwszym
  wysłaniu sprawdzaj przez git status i git remote -v.
- Produkty: UPR Terminal (desktop) i UPR Station (serwer). Nazwy widoczne
  rozdzielono; wewnętrzne identyfikatory i ścieżki danych zachowano.

## Blokady publicznego wydania

- [ ] Potwierdzona lista źródeł inspiracji i skopiowanych/adaptowanych fragmentów.
- [ ] Wymagane zgody, atrybucje i oryginalne licencje tych fragmentów.
- [ ] Obowiązki MPL i źródła komponentów sprawdzone dla faktycznych paczek.
- [ ] Licencje i pochodzenie ikon/logo potwierdzone przez opiekuna.
- [ ] Test aktualizacji istniejących instalacji oraz świeżej instalacji.
- [ ] Przejście starego instalatora UltimatePR Station do UPR Terminal sprawdzone na Windows.
- [ ] CI wykonane i sprawdzone na GitHub (lokalne testy tego nie potwierdzają).

Do czasu zamknięcia listy zalecane jest repozytorium prywatne.
Push sam nie publikuje Releases. Ręczny workflow wymaga unikalnego tagu
i świadomego wyboru publikacji; zawsze oznacza wydanie jako pre-release.
Nie skonfigurowano podpisywania. Dopóki audyt jest otwarty, zachowaj repozytorium prywatne.

## Przed pierwszym wysłaniem

Uruchom w głównym katalogu TEJ kopii:

```sh
python scripts/verify-publication.py
cargo test --locked --tests
```

Skaner jest kontrolą pomocniczą, nie gwarancją braku sekretów. Obejrzyj cały
zestaw plików; ustawienia kont GitHub i audyt praw wymagają osobnej kontroli.
Nie dołączaj wygenerowanych katalogów target/ i release/.

Utwórz na GitHub repozytorium UPR-Terminal, początkowo prywatne i puste
(bez automatycznego README/licencji). Włącz 2FA, secret scanning/push protection
w zakresie dostępnym dla konta, prywatne zgłaszanie podatności i po zaliczeniu CI
ochronę main z wymaganymi kontrolami. Nie dodawaj sekretów do workflow testowego.

Poniższe polecenia są instrukcją; nie zostały wykonane podczas przygotowania:

```sh
git init -b main
git add -- .
git diff --cached --stat
git diff --cached --check
git ls-files
```

Przed commitem sprawdź zawartość indeksu, nie tylko .gitignore.
Poniższe polecenie nie powinno zwrócić żadnego pliku:

```sh
git ls-files -- data backups release target .local-notes github-ready
```

Dopiero po ręcznej akceptacji utwórz commit i dodaj adres własnego repozytorium.
Nie używaj git add -f. Nie wykonuj force push. Nie kopiuj prawdziwego adresu
repozytorium ani poświadczeń do skryptów. Push wykonasz osobno, na własne polecenie.

## Zakres CI

CI testuje źródła, JavaScript, lokalne HTTP/KISS i desktop na Windows.
Nie potwierdza działania radia, aktualizacji OpenRC na Proxmox ani instalatora
na czystym komputerze. Historyczne testy ręczne/desktop-smoke i release_check.py
nie należą do automatycznego zestawu CI; mają własne wymagania i starsze formaty paczek.
