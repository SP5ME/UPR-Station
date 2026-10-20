# Publikacja UPR Station

Docelowe repozytorium: `SP5ME/UPR-Station`. Historia sprzed rozdzielenia pozostaje
w Git. Bieżące źródła, testy i workflow dotyczą wyłącznie UPR Station.
Podział nie tworzy repozytorium zdalnego, nie robi push ani release.

## Blokady publicznego wydania

- [ ] Potwierdzona lista źródeł inspiracji i skopiowanych/adaptowanych fragmentów.
- [ ] Wymagane zgody, atrybucje i oryginalne licencje tych fragmentów.
- [ ] Obowiązki MPL i źródła komponentów sprawdzone dla faktycznych paczek.
- [ ] Licencje i pochodzenie ikon/logo potwierdzone przez opiekuna.
- [ ] Test aktualizacji istniejących instalacji oraz świeżej instalacji.
- [ ] CI wykonane i sprawdzone na GitHub (lokalne testy tego nie potwierdzają).

Do czasu zamknięcia listy zalecane jest repozytorium prywatne.
Push sam nie publikuje Releases. Ręczny workflow wymaga unikalnego tagu
i świadomego wyboru publikacji. Opcja `prerelease` wybiera wydanie testowe;
pełne wydanie wymaga tagu `vVERSION` zgodnego z wersją w Cargo.toml bez sufiksu.
Nie skonfigurowano podpisywania. Dopóki audyt jest otwarty, zachowaj repozytorium prywatne.


## Przed wysłaniem zmian

```sh
python scripts/verify-publication.py
cargo test --locked --tests
git status --short
git diff --check
git diff --cached --check
git diff --cached --stat
git ls-files
```

Skaner sprawdza inwentarz publikacji Git, nie ignorowane lokalne katalogi.
Mimo tego przejrzyj indeks i historię przed upublicznieniem: skaner nie jest
pełnym audytem sekretów ani praw. Nie używaj git add -f dla danych/paczek.
Katalogi data, target, release, backups, github-ready i .local-notes nie mogą
znaleźć się w plikach publikowanych. Instalatory pobierają wyłącznie własny produkt.

Push uruchamia kontrole źródeł; nie tworzy wydania. Build release jest ręczny,
wymaga unikalnego tagu i sprawdzenia wszystkich paczek. Domyślnie tworzy draft;
po jawnym wyborze publikuje wybrany typ wydania. Pełne wydanie jest oznaczane
jako Latest; wydanie testowe ma flagę pre-release. Nie skonfigurowano podpisywania.
Włącz wymagane kontrole gałęzi i ochronę sekretów przed publikacją.
Dotychczasowe wydania w repo Terminal nie zostały zmienione ani usunięte.
