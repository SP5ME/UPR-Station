'use strict';
// The static settings rules now live in app.css; remove the legacy inline block.
document.head.querySelector('style')?.remove();
const $=id=>document.getElementById(id);
const state={snapshot:null,webDetails:null,selected:0,view:'terminal',drafts:Array(8).fill(''),seen:Array(8).fill(0),frozen:false,templates:[],historyKey:'',monitorKey:'',token:sessionStorage.getItem('station-token')||'',language:'',catalog:{},statsEnabled:false,configText:'',configSaveInProgress:false,notificationDismissed:0,notificationExpanded:false,telegramTokenPresent:false};
const names={terminal:['Terminal','Communications center','Radio sessions and network activity in one place.'],stats:['Statistics','Station statistics','Current station, interface and traffic status.'],monitor:['Monitor','Frame monitor','Live AX.25 and KISS traffic from all interfaces.'],heard:['MHeard','Radio activity','Stations heard by your KISS interfaces.'],mail:['MailBox','Station messages','Local packet radio mailbox.'],files:['YAPP files','YAPP files','Files received by your station.'],interfaces:['TNC interfaces','TNC interfaces','Status and connection details for all TNC interfaces.'],settings:['Settings','Your station, your rules','Connections, services and message settings.'],diagnostics:['Diagnostics','Station diagnostics','Runtime status, paths and service checks.']};
const helpDescriptions={
  terminal:{pl:{title:'Terminal',summary:'Sesje AX.25 i komunikacja ze stacjami.',items:['Wybierz sesję, aby zobaczyć jej historię i stan połączenia.','Przycisk Połącz otwiera parametry stacji i trasy; używana jest pierwsza wolna sesja.','Pole wiadomości na dole wysyła tekst do aktywnej sesji.']},en:{title:'Terminal',summary:'AX.25 sessions and station communication.',items:['Select a session to see its history and connection state.','Connect opens station and route parameters and uses the first free session.','The message field at the bottom sends text to the active session.']}},
  'terminal-monitor':{pl:{title:'Monitor ramek',summary:'Podgląd bieżącego ruchu AX.25 i KISS.',items:['Otwórz monitor, aby zobaczyć pełny strumień ramek.','Wstrzymaj widok zatrzymuje odświeżanie przewijanej listy.','Eksport zapisuje widoczne dane do wybranego pliku.']},en:{title:'Frame monitor',summary:'Live view of AX.25 and KISS traffic.',items:['Open monitor to see the complete frame stream.','Pause view stops updates while you inspect the list.','Export saves the visible data to a file you choose.']}},
  'terminal-heard':{pl:{title:'Ostatnio słyszane',summary:'Ostatnie stacje wykryte przez interfejsy TNC.',items:['Nazwa interfejsu pokazuje, gdzie odebrano ramkę.','Dwuklik lub Połącz używa zapamiętanej trasy, jeśli interfejs jest dostępny.','Niedostępny TNC jest oznaczony na czerwono i nie można go wybrać.']},en:{title:'Recently heard',summary:'Stations recently heard by TNC interfaces.',items:['The interface name shows where the frame was received.','Double-click or Connect uses the remembered route when the interface is available.','An unavailable TNC is marked red and cannot be selected.']}},
  heard:{pl:{title:'MHeard',summary:'Lista stacji słyszanych przez wszystkie interfejsy.',items:['Filtruj listę i sprawdzaj wiek oraz liczbę odebranych ramek.','Połącz wykorzystuje zapamiętany interfejs i trasę digi.','Gdy TNC jest niedostępny, połączenie jest wyłączone.']},en:{title:'MHeard',summary:'Stations heard on all interfaces.',items:['Filter the list and check age and received frame count.','Connect uses the remembered interface and digipeater route.','When the TNC is unavailable, connection is disabled.']}},
  mail:{pl:{title:'MailBox',summary:'Lokalna skrzynka wiadomości stacji.',items:['Zakładki pokazują odebrane, wysłane, wszystkie i nieprzeczytane wiadomości.','Wyszukiwarka filtruje nadawcę, adres, temat i treść.','Nowa wiadomość otwiera formularz wysyłania przez lokalną sieć.']},en:{title:'MailBox',summary:'The station local mailbox.',items:['Tabs show received, sent, all and unread messages.','Search filters sender, address, subject and body.','New message opens the local network message form.']}},
  files:{pl:{title:'Pliki YAPP',summary:'Pliki odebrane i przechowywane przez stację.',items:['Wyszukuj po nazwie i wybierz widok listy lub kafelków.','Podgląd otwiera obsługiwany tekst, CSV, JSON lub log bez zapisywania kopii.','Pobierz zapisuje plik w wybranej lokalizacji.']},en:{title:'YAPP files',summary:'Files received and stored by the station.',items:['Search by name and choose list or tile view.','Preview opens supported text, CSV, JSON or log files without saving a copy.','Download saves the file to a location you choose.']}},
  interfaces:{pl:{title:'Interfejsy TNC',summary:'Stan i konfiguracja połączeń z modemami TNC.',items:['Karty pokazują transport TCP lub Serial, adres i stan połączenia.','Priorytet jest używany przy wyborze trasy.','Błąd interfejsu pojawia się przy jego karcie i w powiadomieniach.']},en:{title:'TNC interfaces',summary:'Status and configuration of TNC modem connections.',items:['Cards show TCP or Serial transport, address and connection state.','Priority is used when choosing a route.','An interface error appears on its card and in notifications.']}},
  stats:{pl:{title:'Statystyki',summary:'Podsumowanie pracy stacji i ruchu.',items:['Metryki pokazują połączenia, ramki i czas pracy.','Zakres oraz odświeżanie danych zależą od ustawień statystyk.','Wyłączenie zbierania statystyk zatrzymuje lokalną historię.']},en:{title:'Statistics',summary:'Summary of station activity and traffic.',items:['Metrics show connections, frames and uptime.','The range and refresh depend on the statistics settings.','Disabling statistics stops the local history.']}},
  monitor:{pl:{title:'Monitor ramek',summary:'Pełny, przewijany dziennik ramek i zdarzeń.',items:['Lista pokazuje czas, interfejs, kierunek i treść ramki.','Możesz otworzyć osobny monitor, wstrzymać widok lub wyeksportować dane.','Błędy połączeń są oznaczone kolorem czerwonym.']},en:{title:'Frame monitor',summary:'A full scrolling log of frames and events.',items:['The list shows time, interface, direction and frame content.','Open a separate monitor, pause the view or export the data.','Connection errors are marked in red.']}},
  diagnostics:{pl:{title:'Diagnostyka',summary:'Stan usług, ścieżek danych i podstawowych kontroli.',items:['Użyj tej karty, aby sprawdzić działanie usług stacji.','Szczegóły techniczne pomagają znaleźć problem z konfiguracją lub połączeniem.','Zmiany wykonuj w odpowiedniej zakładce ustawień.']},en:{title:'Diagnostics',summary:'Status of services, data paths and basic checks.',items:['Use this card to check station services.','Technical details help locate configuration or connection problems.','Make changes in the relevant settings tab.']}},
  station:{pl:{title:'Stacja',summary:'Tożsamość stacji i język interfejsu.',items:['Znak stacji jest używany w komunikacji AX.25 i wiadomościach.','Nazwa operatora oraz lokator są danymi opisowymi.','Zmiana ustawień stacji może wymagać rozłączenia sesji.']},en:{title:'Station',summary:'Station identity and interface language.',items:['The callsign is used in AX.25 communication and messages.','Operator name and locator are descriptive data.','Changing station settings may require disconnected sessions.']}},
  settingsInterfaces:{pl:{title:'Interfejsy TNC',summary:'Dodawanie i ustawianie portów radiowych.',items:['Wybierz TCP lub Serial i podaj adres albo urządzenie.','Priorytet określa kolejność wyboru interfejsu dla tras.','Wyłączony interfejs nie jest używany przy nowych połączeniach.']},en:{title:'TNC interfaces',summary:'Adding and configuring radio ports.',items:['Choose TCP or Serial and enter an address or device.','Priority defines interface selection order for routes.','A disabled interface is not used for new connections.']}},
  beacon:{pl:{title:'Beacon',summary:'Automatyczne ogłoszenia stacji.',items:['Włącz beacon i ustaw interwał oraz adres docelowy.','Ścieżka digi określa sposób przekazywania ogłoszenia.','Beacon działa osobno dla wybranego interfejsu.']},en:{title:'Beacon',summary:'Automatic station announcements.',items:['Enable the beacon and set its interval and destination.','The digi path controls how the announcement is forwarded.','Beacon settings apply separately to the selected interface.']}},
  mailbox:{pl:{title:'MailBox',summary:'Ustawienia lokalnej skrzynki wiadomości.',items:['Włączenie modułu udostępnia kartę MailBox.','Znak skrzynki identyfikuje lokalnego odbiorcę.','Wyłączenie modułu ukrywa jego kartę i powiadomienia.']},en:{title:'MailBox',summary:'Local mailbox settings.',items:['Enabling the module makes the MailBox card available.','The mailbox callsign identifies the local recipient.','Disabling the module hides its card and notifications.']}},
  messages:{pl:{title:'Komunikaty i komendy',summary:'Teksty powitalne oraz komendy zdalne.',items:['Ustaw powitanie i pożegnanie sesji.','Komendy zdalne są wykonywane zgodnie z ich przeznaczeniem w stacji.','Pola z ikoną i zawierają dodatkowe objaśnienie.']},en:{title:'Messages and commands',summary:'Welcome texts and remote commands.',items:['Set the session welcome and goodbye messages.','Remote commands are executed according to their station function.','Fields with an i icon have additional explanations.']}},
  yapp:{pl:{title:'YAPP',summary:'Odbiór i przechowywanie plików.',items:['Wybierz katalog pobierania, w którym powstanie folder stacji.','Limit plików i automatyczne czyszczenie chronią miejsce na dysku.','Wyłączenie YAPP ukrywa kartę plików i jego powiadomienia.']},en:{title:'YAPP',summary:'File transfer and storage.',items:['Choose a download directory; the station folder is created inside it.','The file limit and automatic cleanup protect disk space.','Disabling YAPP hides the files card and its notifications.']}},
  ax25:{pl:{title:'AX.25 i ruch',summary:'Parametry transmisji i połączeń AX.25.',items:['T1, T3, N2 i długość pakietu sterują pracą protokołu.','Akceptowanie połączeń i digipeater można niezależnie włączyć.','Tryb automatycznego doboru trasy korzysta z danych MHeard.']},en:{title:'AX.25 and traffic',summary:'AX.25 transmission and connection parameters.',items:['T1, T3, N2 and packet length control protocol behavior.','Accepting connections and digipeating can be enabled independently.','Automatic route selection uses MHeard data.']}},
  application:{pl:{title:'Aplikacja i WWW',summary:'Usługi panelu, dostępu operatora i powiadomień.',items:['Sekcje można rozwijać, aby zmienić tylko potrzebne ustawienia.','Powiadomienia mają osobne kanały: aplikacja, dymek, Telegram i dźwięk.','Telegram jest opcjonalny i wymaga tokenu bota oraz wybranego czatu.']},en:{title:'Application and web',summary:'Panel services, operator access and notifications.',items:['Expand sections to change only the settings you need.','Notifications have separate channels: app, system alert, Telegram and sound.','Telegram is optional and requires a bot token and selected chat.']}}
};
async function loadLanguage(code){const selected=/^[a-z]{2}(?:-[a-z]{2})?$/.test(code||'')?code:'en';let catalog;try{catalog=await fetch(`/lang/${selected}.json`).then(r=>r.ok?r.json():Promise.reject());}catch{catalog=selected==='en'?{}:await fetch('/lang/en.json').then(r=>r.json());}closeHelpPopover();state.language=selected;state.catalog=catalog;applyCatalog();if(typeof fixSettingsHeadings==='function')fixSettingsHeadings();if(state.snapshot)render(state.snapshot);}
function setText(selector,text){const el=document.querySelector(selector);if(el)el.textContent=text;}
setText('.station-identity > span','PACKET RADIO STATION');
function setLabel(id,text){const el=$(id);if(el?.closest('label')?.firstChild)el.closest('label').firstChild.textContent=`${text}`;}
const staticTranslations={'TWOJA STACJA. ZAWSZE W ZASIĘGU.':'YOUR STATION. ALWAYS WITHIN REACH.','STACJA RADIOWA':'RADIO STATION','ZNAK STACJI':'STATION CALLSIGN','INTERFEJSY TNC':'TNC INTERFACES','POŁĄCZENIA AX.25':'AX.25 CONNECTIONS','RUCH RAMEK':'FRAME TRAFFIC','Czas pracy:':'Uptime:','Połączone z TNC':'Connected to TNC','Brak połączenia z TNC':'No TNC connection','Połączenia AX.25':'AX.25 connections','Oczekiwanie na pierwsze ramki':'Waiting for the first frames','Wiadomość':'Message','Wyczyść widok':'Clear view','Połącz':'Connect','Rozłącz':'Disconnect','Wstrzymaj widok':'Pause view','Wznów widok':'Resume view','Monitor ramek':'Frame monitor','Ostatnio słyszane':'Recently heard','Interfejsy':'Interfaces','Ustawienia':'Settings','Pliki YAPP':'YAPP files','MHeard':'MHeard','Wiadomości stacji':'Station messages','Diagnostyka':'Diagnostics','Diagnostyka stacji':'Station diagnostics','Statystyki stacji':'Station statistics','Dane odświeżają się automatycznie':'Data refresh automatically','Odśwież':'Refresh','Ładowanie…':'Loading…','Zapisz ustawienia':'Save settings','Dodaj interfejs':'Add interface','Import .conf':'Import .conf','Eksport .conf':'Export .conf','Tożsamość':'Identity','Serwer WWW':'Web server','AX.25 i ruch':'AX.25 and traffic','Usługi':'Services','Znaki wywoławczy':'Callsign','Nazwa stacji':'Operator name (optional)','Imię operatora (opcjonalne)':'Operator name (optional)','Lokator QTH (opcjonalny)':'QTH locator (optional)','Język':'Language','Opis':'Description','Transport':'Transport','Adres / urządzenie':'Address / device','Port KISS':'KISS port','Priorytet':'Priority','Włączony':'Enabled','Beacon na interfejsie':'Beacon on interface','Ścieżka digipeater (opcjonalna)':'Digipeater path (optional)','Adres nasłuchu':'Listen address','Plik tokenu':'Token file','Interwał beaconu (min)':'Beacon interval (min)','Adres beaconu':'Beacon destination','Calls. MailBox':'MailBox callsign','Katalog pobierania YAPP':'YAPP download directory','Akceptuj połączenia przychodzące':'Accept incoming connections','Negocjuj parametry AX.25 (Modulo 128)':'Negotiate AX.25 parameters','Włącz digipeater':'Enable digipeater','Powitanie':'Welcome message','Pożegnanie':'Goodbye message','Komendy zdalne':'Remote commands','MailBox':'Mailbox','YAPP':'YAPP','Automatyczny odbiór YAPP':'Automatic YAPP receive','Pytaj o lokalizację':'Ask for location','Usuń':'Remove','Pobierz ↓':'Download ↓','Przyjmij':'Accept','Odrzuć':'Reject','Anuluj':'Cancel','Szablony wiadomości':'Message templates','Dostęp operatora':'Operator access','Token jest przechowywany tylko w tej sesji przeglądarki.':'The token is stored only in this browser session.','Podaj token operatora ustawiony na serwerze.':'Enter the operator token configured on the server.','Otwórz stację':'Open station','Token':'Token','Usuń token z przeglądarki':'Remove browser token','Awaryjny podgląd pliku tekstowego':'Emergency text configuration preview','Zmienne: {mycall}, {myname}, {mylocator}, {peercall}. Limit: 4096 B.':'Variables: {mycall}, {myname}, {mylocator}, {peercall}. Limit: 4096 B.','Wymagany restart serwera.':'Server restart required.','Oczekiwanie na pierwsze ramki':'Waiting for the first frames','Brak ramek. Połącz TNC, aby obserwować ruch.':'No frames. Connect a TNC to observe traffic.','Nie odebrano jeszcze żadnych plików':'No files received yet','Stacja czeka na konfigurację. Ustaw swój znak i interfejs TNC w Ustawieniach.':'Station needs configuration. Set your callsign and TNC interface in Settings.','Dostęp do stacji':'Station access'};
staticTranslations['Nazwa']='Name';staticTranslations['Pytaj o lokalizację przy zapisie YAPP']='Ask for location when saving YAPP';staticTranslations['Otwórz monitor']='Open monitor';staticTranslations['Zamknij']='Close';staticTranslations['Zbieranie statystyk stacji']='Collect station statistics';staticTranslations['Zapisuje lokalnie historię ruchu i aktywności z ostatnich 30 dni. Wyłączenie usuwa zapisaną historię.']='Stores local traffic and activity history for the last 30 days. Disabling removes the saved history.';
staticTranslations['Wyświetl pasek obecności operatora']='Show operator presence bar';staticTranslations['Obecność']='Presence';staticTranslations['Auto']='Auto';staticTranslations['Zawsze dostępny']='Always available';staticTranslations['Nie przeszkadzać']='Do not disturb';
staticTranslations['Zaloguj się, aby otworzyć stację.']='Log in to open the station.';staticTranslations['Konto operatora']='Operator account';staticTranslations['Zmień login i hasło']='Change login and password';staticTranslations['Wyloguj']='Log out';
staticTranslations['Status połączenia z TNC']='TNC connection status';staticTranslations['Interfejsy TNC']='TNC interfaces';staticTranslations['Telegram']='Telegram';staticTranslations['Włącz powiadomienia Telegram']='Enable Telegram notifications';staticTranslations['Włącz powiadomienia Telegram']='Enable Telegram notifications';staticTranslations['Token bota Telegram']='Telegram bot token';staticTranslations['Czat lub kanał']='Chat or channel';staticTranslations['Zapisz token']='Save token';staticTranslations['Wykryj czat']='Discover chat';staticTranslations['Wyślij test']='Send test';staticTranslations['Jak podłączyć Telegram?']='How to connect Telegram?';staticTranslations['Wyłączony']='Disabled';staticTranslations['Brak tokenu']='Token missing';staticTranslations['Oczekiwanie na czat']='Waiting for chat';staticTranslations['Skonfigurowany']='Configured';staticTranslations['Powiadomienia']='Notifications';staticTranslations['W aplikacji']='In app';staticTranslations['Dymek']='System notification';staticTranslations['Dźwięk']='Sound';staticTranslations['Nowe połączenie']='New connection';staticTranslations['Nowe połączenie przychodzące']='Incoming connection';staticTranslations['Nowe połączenie wychodzące']='Outgoing connection';staticTranslations['Nowa wiadomość w terminalu']='New terminal message';staticTranslations['Rozłączenie']='Disconnected';staticTranslations['Nowa wiadomość MailBox']='New MailBox message';staticTranslations['Odebrany plik YAPP']='YAPP file received';staticTranslations['Utrata połączenia z TNC']='TNC connection lost';staticTranslations['Błąd krytyczny stacji']='Critical station error';staticTranslations['Wybierz kanały dla każdego zdarzenia. Dymki i dźwięki działają w aplikacji desktopowej, a statusy są dostępne także w panelu WWW.']='Choose channels for each event. System notifications and sounds work in the desktop app; statuses are also available in the web panel.';staticTranslations['Brak']='None';staticTranslations['1 dzwonek']='1 tone';staticTranslations['2 dzwonki']='2 tones';staticTranslations['3 dzwonki']='3 tones';staticTranslations['Panel WWW i dostęp']='Web panel and access';staticTranslations['Dostęp operatora']='Operator access';staticTranslations['Statystyki']='Statistics';
staticTranslations['Znajdź trasę']='Find route';staticTranslations['Wpisz znak stacji.']='Enter a station callsign.';staticTranslations['Trasa do stacji jest nieznana. Wprowadź ręcznie parametry.']='The route to the station is unknown. Enter the parameters manually.';
function translateDom(){const reverse=Object.fromEntries(Object.entries(staticTranslations).map(([pl,en])=>[en,pl]));const map=state.language==='pl'?reverse:staticTranslations;const walker=document.createTreeWalker(document.body,NodeFilter.SHOW_TEXT);const nodes=[];while(walker.nextNode())nodes.push(walker.currentNode);nodes.forEach(n=>{const value=n.nodeValue.trim();if(map[value])n.nodeValue=n.nodeValue.replace(value,map[value]);});}
function applyCatalog(){const c=state.catalog;if(!c['view.terminal.heading'])return;setText('.nav-label',c['nav.station']);setText('.sidebar-bottom strong',c['nav.packet']);setText('.sidebar-bottom p',c['nav.subtitle']);const nav={terminal:'Terminal',stats:'Statystyki',monitor:'Monitor',heard:'MHeard',mail:'MailBox',files:'YAPP',interfaces:'TNC interfaces',settings:'Ustawienia',diagnostics:'Diagnostyka'};Object.entries(nav).forEach(([view,fallback])=>{const b=document.querySelector(`[data-view="${view}"]`);if(b){const text=[...b.childNodes].find(n=>n.nodeType===3&&n.textContent.trim());if(text)text.textContent=` ${c[`view.${view}.title`]||fallback} `;}});const v=names[state.view];if(v){$('page-name').textContent=c[`view.${state.view}.title`]||v[0];$('heading').replaceChildren(document.createTextNode(c[`view.${state.view}.heading`]||v[1]),node('span','','.'));$('subtitle').textContent=c[`view.${state.view}.subtitle`]||v[2];}setText('.settings-form .panel-head h2',c['settings.title']);setText('#config-save',c['settings.save']);setText('#interface-add',c['settings.add']);setText('#config-import',c['settings.import']);setText('#config-export',c['settings.export']);setLabel('settings-callsign',c['settings.callsign']);setLabel('settings-myname',c['settings.myname']);setLabel('settings-language',c['settings.language']);translateDom();fixSettingsHeadings();}
function node(tag,cls,text){const n=document.createElement(tag);if(cls)n.className=cls;if(text!==undefined)n.textContent=text;return n;}
function svgIcon(id,cls='icon'){const icon=document.createElementNS('http://www.w3.org/2000/svg','svg');icon.classList.add(...cls.split(/\s+/).filter(Boolean));icon.setAttribute('aria-hidden','true');const use=document.createElementNS('http://www.w3.org/2000/svg','use');use.setAttribute('href',`/icons.svg#${id}`);icon.append(use);return icon;}
function normalizeCloseIcons(root=document){root.querySelectorAll?.('use[href$="#close-circle-outline"]').forEach(use=>use.setAttribute('href','/icons.svg#close'));}
normalizeCloseIcons();
new MutationObserver(()=>normalizeCloseIcons()).observe(document.body,{childList:true,subtree:true});
const settingsTabIcons={station:'home',interfaces:'lan-connect',beacon:'antenna',mailbox:'email-outline',messages:'console',yapp:'file-swap-outline',ax25:'access-point',application:'cog-outline',web:'lan-connect'};

function button(text,fn,cls='text-button'){const b=node('button',cls,text);b.type='button';b.addEventListener('click',()=>guard(fn));return b;}
let activeHelpPopover=null;
function helpLanguage(){return state.language==='pl'?'pl':'en';}
const helpOptionDetails={
  terminal:{pl:['Połącz: wybierz znak, interfejs i trasę digi.','Rozłącz: kończy połączenie bieżącej sesji.','Wyczyść terminal: usuwa historię tylko wybranej sesji.'],en:['Connect: choose the callsign, interface and digi route.','Disconnect: ends the current session.','Clear terminal: removes history only for the selected session.']},
  'terminal-monitor':{pl:['Otwórz monitor pokazuje pełny dziennik ramek.','Wstrzymaj widok zatrzymuje napływ nowych wierszy.','Eksport zapisuje dane monitora do wskazanego pliku.'],en:['Open monitor shows the complete frame log.','Pause view stops new rows while you inspect them.','Export saves monitor data to a file you choose.']},
  'terminal-heard':{pl:['Status ONLINE/OFFLINE wskazuje dostępność TNC.','Czas i interfejs pomagają ocenić aktualność trasy.'],en:['The ONLINE/OFFLINE status shows TNC availability.','Time and interface help assess route freshness.']},
  heard:{pl:['Filtry pozwalają ograniczyć listę stacji.','Przycisk Połącz wykorzystuje zapamiętaną trasę.'],en:['Filters narrow the list of heard stations.','Connect uses the remembered route.']},
  mail:{pl:['Zakładka Wszystkie pokazuje pełną skrzynkę.','Pola Od, Do, Temat i status działają razem z wyszukiwarką.'],en:['The All tab shows the complete mailbox.','From, To, Subject and status fields work with the search box.']},
  files:{pl:['Podgląd służy do szybkiego odczytu plików tekstowych.','Pobierz zapisuje plik w folderze UPR YAPP.','Kafelki pokazują typ pliku za pomocą ikony rozszerzenia.'],en:['Preview quickly reads supported text files.','Download saves the file in the UPR YAPP folder.','Tiles show the file type with an extension icon.']},
  interfaces:{pl:['Nazwa i opis identyfikują interfejs.','Adres/urządzenie wskazuje połączenie TCP lub port szeregowy.','Status i błąd pokazują bieżącą dostępność TNC.'],en:['Name and description identify the interface.','Address/device identifies the TCP connection or serial port.','Status and error show current TNC availability.']},
  stats:{pl:['Metryki RX/TX pokazują ruch odebrany i wysłany.','Historia obejmuje dni, stacje i interfejsy.'],en:['RX/TX metrics show received and sent traffic.','History covers days, stations and interfaces.']},
  monitor:{pl:['Dziennik można przewijać niezależnie od reszty strony.','Wiersze błędów są oznaczone czerwonym kolorem.'],en:['The log scrolls independently from the rest of the page.','Error rows are marked in red.']},
  diagnostics:{pl:['Odśwież pobiera aktualny stan usług.','Szczegóły pomagają zweryfikować ścieżki plików i panel WWW.'],en:['Refresh gets the current service state.','Details help verify file paths and the web panel.']},
  station:{pl:['Znak stacji jest używany w komunikacji radiowej.','Nazwa operatora i lokator opisują stację.','Język zmienia teksty panelu i pomocy.'],en:['The callsign is used in radio communication.','Operator name and locator describe the station.','Language changes panel and help text.']},
  settingsInterfaces:{pl:['Transport wybiera TCP/IP albo Serial.','Baud i Port KISS dotyczą urządzeń szeregowych.','Priorytet decyduje o kolejności wyboru trasy.','Beacon i ścieżka beaconu sterują ogłoszeniami na tym interfejsie.'],en:['Transport selects TCP/IP or Serial.','Baud and KISS port apply to serial devices.','Priority controls route selection order.','Beacon and its path control announcements on this interface.']},
  beacon:{pl:['Interwał określa częstotliwość wysyłania.','Adres beaconu wskazuje odbiorcę ogłoszenia.','Ścieżka digi określa pośrednie stacje.'],en:['The interval controls sending frequency.','Beacon address selects the announcement recipient.','The digi path selects intermediate stations.']},
  mailbox:{pl:['Włączenie udostępnia kartę MailBox.','Znak MailBox identyfikuje lokalną skrzynkę.','Wyłączenie ukrywa kartę i powiadomienia MailBox.'],en:['Enabling makes the MailBox card available.','The MailBox callsign identifies the local mailbox.','Disabling hides the card and MailBox notifications.']},
  messages:{pl:['Powitanie i pożegnanie są wysyłane przy sesji.','Komendy zdalne określają dostępne polecenia stacji.'],en:['Welcome and goodbye messages are sent during a session.','Remote commands define available station commands.']},
  yapp:{pl:['Katalog pobierania jest bazą dla folderu UPR YAPP.','Limit plików ogranicza liczbę przechowywanych plików.','Automatyczne czyszczenie usuwa najstarsze pliki po osiągnięciu limitu.'],en:['The download directory is the base for the UPR YAPP folder.','The file limit restricts stored file count.','Automatic cleanup removes oldest files at the limit.']},
  ax25:{pl:['T1 i T3 określają czasy oczekiwania protokołu.','N2 określa liczbę ponowień, a PACLEN rozmiar pakietu.','Akceptowanie połączeń i digipeater są niezależnymi opcjami.'],en:['T1 and T3 define protocol timeouts.','N2 defines retries and PACLEN defines packet size.','Accepting connections and digipeating are independent options.']},
  application:{pl:['Panel WWW określa adres nasłuchu i ochronę dostępu.','Telegram wymaga tokenu bota i wybranego czatu.','Powiadomienia wybiera się osobno dla aplikacji, dymka, Telegrama i dźwięku.'],en:['The web panel defines its listen address and access protection.','Telegram requires a bot token and selected chat.','Notifications are selected separately for the app, system alert, Telegram and sound.']}
};
function helpCopy(key){const entry=helpDescriptions[key]||helpDescriptions.terminal;const copy=entry[helpLanguage()]||entry.en;const extra=helpOptionDetails[key]?.[helpLanguage()]||[];return {...copy,items:[...copy.items,...extra]};}
function helpButtonLabel(key){const copy=helpCopy(key);return state.language==='pl'?`Pokaż pomoc: ${copy.title}`:`Show help: ${copy.title}`;}
function closeHelpPopover(){if(!activeHelpPopover)return;const active=activeHelpPopover;active.popover.remove();active.button.setAttribute('aria-expanded','false');activeHelpPopover=null;if(document.contains(active.button))active.button.focus({preventScroll:true});if(state.snapshot)render(state.snapshot);}
function positionHelpPopover(){const active=activeHelpPopover;if(!active)return;const card=active.card.getBoundingClientRect();const width=Math.min(900,Math.max(280,card.width-32),window.innerWidth-24);active.popover.style.width=`${width}px`;const measured=active.popover.getBoundingClientRect();const left=Math.max(12,Math.min(card.left+(card.width-measured.width)/2,window.innerWidth-measured.width-12));let top=card.top+(card.height-measured.height)/2;top=Math.max(12,Math.min(top,window.innerHeight-measured.height-12));active.popover.style.left=`${left}px`;active.popover.style.top=`${top}px`;}
function openHelpPopover(key,anchor,card){if(activeHelpPopover?.button===anchor){closeHelpPopover();return;}closeHelpPopover();const copy=helpCopy(key);const popover=node('div','help-popover');popover.id=`help-popover-${Date.now()}`;popover.setAttribute('role','dialog');popover.setAttribute('aria-modal','false');popover.setAttribute('aria-label',copy.title);const head=node('div','help-popover-head');const title=node('h3','help-popover-title',copy.title);const close=node('button','help-close');close.type='button';close.setAttribute('aria-label',state.language==='pl'?'Zamknij pomoc':'Close help');close.title=close.getAttribute('aria-label');close.append(svgIcon('close'));close.onclick=()=>closeHelpPopover();head.append(title,close);const summary=node('p','help-popover-summary',copy.summary);const label=node('h4','help-popover-options-title',state.language==='pl'?'Najważniejsze opcje':'Key options');const list=node('ul','help-popover-list');copy.items.forEach(item=>list.append(node('li','',item)));popover.append(head,summary,label,list);document.body.append(popover);anchor.setAttribute('aria-expanded','true');anchor.setAttribute('aria-controls',popover.id);activeHelpPopover={button:anchor,card,popover,key};requestAnimationFrame(positionHelpPopover);close.focus({preventScroll:true});}
function createHelpButton(key,card){const b=node('button','help-button');b.type='button';b.append(svgIcon('help-circle-outline'));b.dataset.helpKey=key;b.setAttribute('aria-expanded','false');b.setAttribute('aria-label',helpButtonLabel(key));b.title=helpButtonLabel(key);b.addEventListener('click',event=>{event.stopPropagation();openHelpPopover(b.dataset.helpKey,b,card);});return b;}
function updateHelpButton(button,key){button.dataset.helpKey=key;button.setAttribute('aria-label',helpButtonLabel(key));button.title=helpButtonLabel(key);}
function appendCardHelp(card,key){if(!card)return;const head=card.querySelector(':scope > .panel-head');if(!head)return;let help=head.querySelector(':scope > .help-head-actions .help-button, :scope > .help-button');if(help){updateHelpButton(help,key);if(activeHelpPopover?.button===help&&activeHelpPopover.key!==key)closeHelpPopover();return;}help=createHelpButton(key,card);const children=[...head.children];const last=children.at(-1);if(last&&last!==children[0]&&last.tagName==='DIV'){last.classList.add('help-head-actions');last.append(help);}else{const actions=node('div','help-head-actions');actions.append(help);head.append(actions);}}
function currentSettingsHelpKey(){const tabs=$('settings-tabs'),items=settingsTabItems();const index=Number(tabs?.dataset.selected||0);const item=items[index];if(!item)return'station';if(item.key==='settings.application_web')return'application';const key=item.key.replace(/^settings\./,'');return key==='interfaces'?'settingsInterfaces':key;}
function appendSettingsHelp(){const card=document.querySelector('#view-settings .settings-form');const head=card?.querySelector(':scope > .panel-head');if(!head)return;const key=currentSettingsHelpKey();let help=head.querySelector(':scope > .help-head-actions .help-button, :scope > .help-button');if(!help){help=createHelpButton(key,card);const actions=node('div','help-head-actions');actions.append(help);head.append(actions);}else{updateHelpButton(help,key);if(activeHelpPopover?.button===help&&activeHelpPopover.key!==key)closeHelpPopover();}}
function bindAllCardHelp(){const cards=[['#view-terminal .conversation','terminal'],['#view-terminal .monitor','terminal-monitor'],['#view-terminal .heard-preview','terminal-heard'],['#view-heard > .panel','heard'],['#view-mail .mailbox-panel','mail'],['#view-files > .panel','files'],['#view-interfaces .interfaces-panel','interfaces'],['#view-stats .stats-view-panel','stats'],['#view-monitor > .panel','monitor'],['#view-diagnostics > .panel','diagnostics']];cards.forEach(([selector,key])=>document.querySelectorAll(selector).forEach(card=>appendCardHelp(card,key)));appendSettingsHelp();}
document.addEventListener('pointerdown',event=>{if(activeHelpPopover&&!activeHelpPopover.popover.contains(event.target)&&event.target!==activeHelpPopover.button)closeHelpPopover();});
document.addEventListener('keydown',event=>{if(event.key==='Escape')closeHelpPopover();});
window.addEventListener('resize',positionHelpPopover);window.addEventListener('scroll',positionHelpPopover,true);
function focusSettingsField(field,{scroll=true}={}){if(!field)return;field.focus({preventScroll:true});if(scroll)field.scrollIntoView({behavior:'smooth',block:'center'});field.setAttribute('aria-invalid','true');setTimeout(()=>field.removeAttribute('aria-invalid'),5000);}
function settingsError(message,field){const exception=Error(message);exception.field=field;return exception;}
function focusSettingsError(exception){
  if(state.view!=='settings')return;
  let field=exception?.field;
  const message=String(exception?.message||exception||'');
  if(!field){
    if(/callsign|znak/i.test(message))field=$('settings-callsign');
    else if(/locator|lokator/i.test(message))field=$('settings-locator');
    else if(/KISS port|Port KISS/i.test(message))field=document.querySelector('#interface-forms .if-port');
    else if(/priority|Priorytet/i.test(message))field=document.querySelector('#interface-forms .if-priority');
    else if(/endpoint|host:port|Adres/i.test(message))field=document.querySelector('#interface-forms .if-endpoint');
    else if(/baud|COM|Serial device|Prędkość/i.test(message))field=document.querySelector('#interface-forms .if-endpoint');
    else if(/interface|interfejs/i.test(message))field=document.querySelector('#interface-forms .if-name');
  }
  let section=field?.closest('.form-section');
  if(!section&&/notification|powiadom|telegram/i.test(message))section=document.querySelector('.form-section[data-settings-key="application"]');
  if(!section&&/WWW|web|bind|token|presence|obecność|heartbeat/i.test(message))section=document.querySelector('.form-section[data-settings-key="web"]');
  if(section&&typeof settingsTabItems==='function'&&typeof selectSettingsTab==='function'){
    const index=settingsTabItems().findIndex(item=>Array.isArray(item.element)?item.element.includes(section):item.element===section);
    if(index>=0)selectSettingsTab(index);
  }
  const card=field?.closest('.interface-form,.settings-collapsible,.panel')||section;
  if(card?.classList.contains('collapsed')){
    card.classList.remove('collapsed');
    const toggle=card.querySelector(':scope > .section-title .interface-toggle, :scope > .section-title .text-button');
    if(toggle){toggle.textContent=interfaceToggleText(true);toggle.setAttribute('aria-expanded','true');}
  }
  if(card){
    card.classList.add('settings-error-target');
    setTimeout(()=>card.classList.remove('settings-error-target'),5000);
    requestAnimationFrame(()=>{const rect=card.getBoundingClientRect();window.scrollTo({top:Math.max(0,window.scrollY+rect.top-18),behavior:'smooth'});});
  }
  focusSettingsField(field,{scroll:false});
}
function friendlyErrorMessage(exception){
  const raw=String(exception?.message||exception||'');
  if(/^Invalid notification setting:/i.test(raw))return 'Nieprawidłowa konfiguracja powiadomień. Odśwież ustawienia aplikacji i zapisz je ponownie.';
  if(/^Unknown config key:/i.test(raw))return 'Konfiguracja zawiera nieznane ustawienie. Załaduj ustawienia ponownie i spróbuj zapisać jeszcze raz.';
  if(/must be true\/false/i.test(raw))return 'Jedno z pól wyboru ma nieprawidłową wartość. Sprawdź zaznaczenia i zapisz ustawienia ponownie.';
  if(/must be 0-3/i.test(raw))return 'Liczba dzwonków musi mieścić się w zakresie od 0 do 3.';
  if(/interface|interfejs/i.test(raw))return 'Nieprawidłowe ustawienie interfejsu TNC. Sprawdź nazwę, transport i adres urządzenia.';
  if(/required|wymag|required field|brak/i.test(raw))return 'Brakuje wymaganej wartości. Sprawdź wskazane pole i uzupełnij je.';
  return raw||'Nie udało się zapisać ustawień. Sprawdź wskazane pola i spróbuj ponownie.';
}
function error(e){const message=friendlyErrorMessage(e);$('error').textContent=message;$('error').hidden=false;focusSettingsError({...e,message});}
function toast(text){$('toast').textContent=text;$('toast').hidden=false;clearTimeout(toast.timer);toast.timer=setTimeout(()=>$('toast').hidden=true,3500);}
let notificationAudioContext=null;
const notificationVolumeKey='upr-notification-volume';
let notificationVolume=50;
try{
  const stored=localStorage.getItem(notificationVolumeKey),value=Number(stored);
  if(stored!==null&&stored.trim()!==''&&Number.isFinite(value)&&value>=0&&value<=100)notificationVolume=Math.round(value);
}catch{}
function playNotificationSound(count){
  if(window.__UPR_DESKTOP__||!count||!notificationVolume||!window.AudioContext)return;
  try{
    notificationAudioContext??=new AudioContext();
    const context=notificationAudioContext;
    const schedule=()=>{
      if(!notificationVolume)return;
      // Use nearly the full output range, with headroom to avoid clipping.
      const peak=0.95*Math.pow(notificationVolume/100,2);
      let at=context.currentTime;
      for(let i=0;i<Number(count);i++){
        const oscillator=context.createOscillator();
        const gain=context.createGain();
        oscillator.frequency.value=640+i*90;
        gain.gain.setValueAtTime(0.0001,at);
        gain.gain.exponentialRampToValueAtTime(peak,at+0.015);
        gain.gain.setValueAtTime(peak,at+0.22);
        gain.gain.exponentialRampToValueAtTime(0.0001,at+0.30);
        oscillator.connect(gain).connect(context.destination);
        oscillator.start(at);
        oscillator.stop(at+0.32);
        at+=0.40;
      }
    };
    if(context.state==='suspended')context.resume().then(schedule).catch(()=>{});else schedule();
  }catch{}
}
function notificationTime(item){const time=Number(item?.time||0);return time?new Date(time*1000).toLocaleTimeString([], {hour:'2-digit',minute:'2-digit',second:'2-digit'}):'';}
function notificationIsError(item){return ['disconnection','critical'].includes(item?.kind)||(item?.kind==='tnc'&&/utrata|błąd|error/i.test(item?.title||''));}
const sessionStatusTranslations={
  'Connecting':'Łączenie',
  'Connecting (modulo 8; XID follows UA)':'Łączenie (modulo 8; po UA nastąpi XID)',
  'Connection cancelled':'Połączenie anulowane',
  'XID selected modulo 128; sending SABME':'Wybrano modulo 128 przez XID; wysyłanie SABME',
  'XID selected modulo 8; connected':'Wybrano modulo 8 przez XID; połączono',
  'Invalid XID response; connected in modulo 8':'Nieprawidłowa odpowiedź XID; połączono w modulo 8',
  'Connected (SABME; modulo 128; sequence state reset)':'Połączono (SABME; modulo 128; zresetowano stan sekwencji)',
  'Connected (SABM; sequence state reset)':'Połączono (SABM; zresetowano stan sekwencji)',
  'XID rejected; connected in modulo 8':'XID odrzucony; połączono w modulo 8',
  'SABME rejected; retrying with SABM in modulo 8':'SABME odrzucony; ponawianie z SABM w modulo 8',
  'Disconnected by peer':'Rozłączono przez stację zdalną',
  'Disconnected/refused (DM)':'Połączenie odrzucone (DM)',
  'Connected in modulo 8; negotiating AX.25 parameters (XID)':'Połączono w modulo 8; negocjowanie parametrów AX.25 (XID)',
  'Connected (modulo 128)':'Połączono (modulo 128)',
  'Connected':'Połączono',
  'Disconnected':'Rozłączono',
  'Invalid N(R); resetting link, queued data discarded':'Nieprawidłowe N(R); resetowanie połączenia, usunięto dane z kolejki',
  'Non-text PID received; payload not displayed':'Odebrano nietekstowy PID; treść nie została wyświetlona',
  'FRMR received; resetting link, queued data discarded':'Odebrano FRMR; resetowanie połączenia, usunięto dane z kolejki',
  'Retry limit reached; session closed, unsent data discarded':'Osiągnięto limit prób; sesja zamknięta, usunięto niewysłane dane',
  'XID timeout; connected in modulo 8':'Przekroczono czas XID; połączono w modulo 8',
  'TCP lost; session and queued data cleared':'Utracono TCP; sesja zamknięta i usunięto dane z kolejki'
};
function translateSessionStatus(text){if(state.language!=='pl')return text;if(sessionStatusTranslations[text])return sessionStatusTranslations[text];if(text.startsWith('YAPP: '))return`YAPP: ${text.slice(6)}`;if(text.startsWith('YAPP stopped: '))return`YAPP zatrzymany: ${text.slice(14)}`;if(text.startsWith('YAPP received: '))return`YAPP odebrany: ${text.slice(15)}`;if(text.startsWith('YAPP sent: '))return`YAPP wysłany: ${text.slice(11)}`;if(text==='Incoming YAPP: operator approval required')return'Przychodzący YAPP: wymagana zgoda operatora';if(text==='YAPP timeout')return'Przekroczono czas YAPP';return text;}
function renderPresenceBar(details=state.webDetails){const bar=$('presence-bar'),select=$('presence-bar-select');if(!bar||!select)return;state.webDetails=details||state.webDetails;const enabled=bool(state.webDetails?.presenceBarEnabled);const setting=$('settings-presence-bar');if(setting&&!setting.dataset.changed)setting.checked=enabled;bar.hidden=!enabled;if(enabled){const mode=state.webDetails?.presenceMode||'auto';if(select.value!==mode)select.value=mode;}}
async function setPresenceMode(mode){if(!['auto','always','dnd'].includes(mode))return;const select=$('presence-bar-select');if(select)select.disabled=true;try{const result=await request('presence/mode',{body:{mode}});state.webDetails={...(state.webDetails||{}),presenceMode:result.presenceMode||mode};renderPresenceBar();}catch(exception){if(select&&state.webDetails?.presenceMode)select.value=state.webDetails.presenceMode;error(exception);}finally{if(select)select.disabled=false;}}
function renderNotificationSounds(snapshot){
  // Older servers expose sound only on center entries. New servers provide an independent channel.
  const sounds=snapshot.notificationSounds??(snapshot.notifications||[]).map(item=>({id:item.id,count:item.sound}));
  const newest=Number(sounds.at(-1)?.id||0);
  if(renderNotificationSounds.last===undefined||newest<renderNotificationSounds.last){
    renderNotificationSounds.last=newest;return;
  }
  const pending=sounds.filter(item=>Number(item.id)>renderNotificationSounds.last);
  renderNotificationSounds.last=newest;
  // Coalesce a polling batch into its strongest signal instead of overlapping tones.
  const count=pending.reduce((max,item)=>Math.max(max,Number(item.count)||0),0);
  if(count>0)playNotificationSound(count);
}
function renderNotifications(snapshot){renderNotificationSounds(snapshot);const list=$('notification-list'),center=$('notification-center'),latest=$('notification-latest'),toggle=$('notification-toggle');if(!list||!center)return;const items=(snapshot.notifications||[]).filter(item=>Number(item.id)>state.notificationDismissed);const emptyLabel=state.language==='pl'?'Brak nowych powiadomień':'No new notifications';const key=JSON.stringify(items);center.hidden=false;center.classList.toggle('is-expanded',!!state.notificationExpanded);if(toggle)toggle.setAttribute('aria-expanded',String(!!state.notificationExpanded));const newest=items.at(-1);if(latest){latest.className=`notification-latest${notificationIsError(newest)?' is-error':''}`;latest.textContent=newest?`${newest.title} · ${newest.body} · ${notificationTime(newest)}`:emptyLabel;}if(list.dataset.renderKey!==key){rows(list,items.length?items.slice(-8).reverse().map(item=>{const row=node('article',`notification-item${notificationIsError(item)?' is-error':''}`);row.append(node('strong','',item.title),node('p','',item.body),node('small','',notificationTime(item)));return row;}):[node('div','notification-empty',`${emptyLabel}.`)]);list.dataset.renderKey=key;} }
$('notification-toggle')?.addEventListener('click',()=>{state.notificationExpanded=!state.notificationExpanded;if(state.snapshot)renderNotifications(state.snapshot);});
$('notification-clear')?.addEventListener('click',()=>{state.notificationDismissed=Number(state.snapshot?.notifications?.at(-1)?.id||0);state.notificationExpanded=false;if(state.snapshot)renderNotifications(state.snapshot);});
$('presence-bar-select')?.addEventListener('change',event=>setPresenceMode(event.target.value));
 $('settings-presence-bar')?.addEventListener('change',event=>{event.target.dataset.changed='true';renderPresenceBar({...state.webDetails,presenceBarEnabled:event.target.checked});});
document.addEventListener('change',event=>{if(event.target?.id==='settings-presence-bar')event.target.dataset.changed='true';});
async function guard(fn,onError){try{$('error').hidden=true;await fn();}catch(e){if(onError)onError(e);else error(e);}}
function headers(){const h={'X-Station-Request':'1'};if(state.token)h.Authorization=`Bearer ${state.token}`;return h;}
async function request(path,{body,raw,query,method='POST'}={}){
  if(path==='web/details')method='POST';
  const h=headers();if(method==='POST'&&!raw)h['Content-Type']='application/x-www-form-urlencoded';
  const response=await fetch(`/api/${path}${query?'?'+new URLSearchParams(query):''}`,{method,headers:h,credentials:'same-origin',body:method==='GET'?undefined:raw||new URLSearchParams(body||{}),signal:AbortSignal.timeout(35000)});
  if(response.status===401){const dialog=$('token-dialog');if(dialog&&!dialog.open)dialog.showModal();throw Error('Zaloguj się, aby otworzyć stację.');}
  if(!response.ok){const text=await response.text();let msg=text;try{msg=JSON.parse(text).error||text;}catch{}throw Error(msg);}
  const result=response.headers.get('content-type')?.includes('application/json')?await response.json():await response.text();
  if(path==='web/details'&&result&&typeof result==='object')state.webDetails=result;
  return result;
}
async function syncBuildVersion(){
  try{
    const info=await request('build-info',{method:'GET'});
    const version=String(info?.version||'').trim();
    if(version)document.querySelectorAll('[data-build-version]').forEach(element=>{element.textContent=version;});
  }catch{}
}
function authError(id,message){const el=$(id);if(el){el.textContent=message;el.hidden=!message;}}
async function authResponse(path,body){const response=await fetch(`/api/${path}`,{method:'POST',headers:{'X-Station-Request':'1','Content-Type':'application/x-www-form-urlencoded'},credentials:'same-origin',body:new URLSearchParams(body)});const text=await response.text();let data={};try{data=text?JSON.parse(text):{};}catch{}if(!response.ok)throw Error(data.error||'Nie udało się zalogować.');return data;}
$('token-form').onsubmit=async event=>{event.preventDefault();authError('token-error','');const button=document.querySelector('#token-form button');if(button)button.disabled=true;try{await authResponse('auth/login',{username:$('token-login').value.trim(),password:$('token').value});$('token').value='';$('token-dialog').close();toast('Zalogowano.');}catch(exception){authError('token-error',exception.message||String(exception));}finally{if(button)button.disabled=false;}};
$('logout')?.addEventListener('click',()=>guard(async()=>{await request('auth/logout');state.token='';sessionStorage.removeItem('station-token');location.reload();}));
function collapseSettingsSections(){
  document.querySelectorAll('#view-settings .interface-form').forEach(card=>{
    card.classList.add('collapsed');
    const toggle=card.querySelector('.interface-toggle');
    if(toggle){toggle.textContent=interfaceToggleText(false);toggle.setAttribute('aria-expanded','false');}
  });
  document.querySelectorAll('#view-settings details').forEach(details=>{details.open=false;});
}
function showView(view){
  closeHelpPopover();
  if(view==='mail'&&!bool(state.snapshot?.mailboxEnabled))view='terminal';
  if(state.view==='settings'||view==='settings')collapseSettingsSections();
  if(state.view==='heard'&&view!=='heard'){
    heardExpanded.clear();
    delete $('heard-table').dataset.renderKey;
    if(state.snapshot)renderHeard(state.snapshot);
  }
  state.view=view;
  document.querySelectorAll('.view').forEach(n=>n.hidden=n.id!==`view-${view}`);
  document.querySelectorAll('.nav').forEach(n=>n.classList.toggle('active',n.dataset.view===view));
  $('page-name').textContent=names[view][0];
  $('heading')?.replaceChildren(document.createTextNode(names[view][1]),node('span','','.'));
  if($('subtitle'))$('subtitle').textContent=names[view][2];
  window.scrollTo({top:0,left:0,behavior:'auto'});
  document.documentElement.scrollTop=0;document.body.scrollTop=0;
  if(view==='settings')guard(async()=>{await loadSettings();if(state.view==='settings')collapseSettingsSections();});
  if(view==='files')guard(loadFiles);
  if(view==='diagnostics')guard(loadDiagnostics);
}
const connectButton=$('connect-open'),beaconButton=$('beacon'),terminalHead=document.querySelector('#view-terminal .conversation .panel-head');
if(terminalHead){const headActions=node('div','terminal-head-actions');if(connectButton){connectButton.classList.add('small');headActions.append(connectButton);}if(beaconButton){beaconButton.classList.add('small');headActions.append(beaconButton);}if(headActions.childElementCount)terminalHead.append(headActions);}
document.querySelectorAll('[data-view]').forEach(b=>b.addEventListener('click',()=>showView(b.dataset.view)));
document.querySelectorAll('#connect-dialog').forEach((dialog,index)=>{if(index>0)dialog.remove();});
$('setup-note')?.addEventListener('click',()=>{showView('settings');setTimeout(()=>$('settings-callsign')?.focus(),0);});
$('setup-note')?.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();$('setup-note').click();}});
document.addEventListener('click',event=>{const button=event.target.closest?.('[data-close]');const dialog=button?.closest('dialog');if(dialog?.open)dialog.close();});
for(let i=0;i<8;i++){const b=button('',()=>selectSession(i),'session-tab');b.id=`tab-${i}`;b.role='tab';$('session-tabs').append(b);}
function selectSession(id){state.drafts[state.selected]=$('message').value;state.selected=id;$('message').value=state.drafts[id];state.historyKey='';if(state.snapshot)render(state.snapshot);$('message').focus();}
function rows(container,items){container.replaceChildren(...items);}
function renderInterfacesTable(container,items){if(!container)return;const key=JSON.stringify(items||[]);if(container.dataset.renderKey===key)return;const wrap=node('div','table-wrap');const table=node('table','interfaces-table');const head=node('thead');const header=node('tr');['Nazwa','Opis','Adres / urządzenie','Status','Błąd'].forEach(label=>header.append(node('th','',label)));head.append(header);const body=node('tbody');(items||[]).forEach(item=>{const row=node('tr');const status=item.enabled===false?['interface-disabled','WYŁĄCZONY']:item.online?['online','● ONLINE']:['offline','○ OFFLINE'];row.append(node('td','interface-name-cell',item.name||'—'),node('td','',item.description||'—'),node('td','interface-endpoint-cell',item.endpoint||'—'),node('td',`interface-status ${status[0]}`,status[1]),node('td',item.error?'interface-error':'',item.error||'—'));body.append(row);});if(!items?.length){const row=node('tr');const cell=node('td','quiet-empty','Brak skonfigurowanych interfejsów.');cell.colSpan=5;row.append(cell);body.append(row);}table.append(head,body);wrap.append(table);container.replaceChildren(wrap);container.dataset.renderKey=key;}
function syncMailboxView(snapshot){
  const enabled=bool(snapshot?.mailboxEnabled),view=$('view-mail'),nav=document.querySelector('[data-view="mail"]');
  if(nav)nav.hidden=!enabled;
  if(!view)return;
  if(!enabled){
    if(state.view==='mail'){state.view='terminal';showView('terminal');}
    view.hidden=true;
  }else if(state.view==='mail'){
    view.hidden=false;
  }
}
function formatHeardAge(seconds){const age=Math.max(0,Number(seconds)||0);return age<60?`${age}s temu`:`${Math.floor(age/60)} min temu`;}
function formatHeardAgeLabels(){document.querySelectorAll('#heard-preview .heard-item>small').forEach(item=>{if(/^\d+s temu$/.test(item.textContent)||/^\d+ min temu$/.test(item.textContent)){const seconds=item.textContent.includes('min')?Number.parseInt(item.textContent,10)*60:Number.parseInt(item.textContent,10);item.textContent=formatHeardAge(seconds);}});document.querySelectorAll('#heard-table tr').forEach(row=>{const cell=row.children[3];if(!cell)return;const text=cell.textContent.trim();if(/^\d+s$/.test(text)||/^\d+ min$/.test(text)){const seconds=text.includes('min')?Number.parseInt(text,10)*60:Number.parseInt(text,10);cell.textContent=formatHeardAge(seconds).replace(' temu','');}});}
function render(s){
  requestAnimationFrame(formatHeardAgeLabels);
  if(state.language)setTimeout(translateDom,0);
  const hasActive=s.sessions.some(session=>session.state!=='Idle')||Number(s.mailboxSessions||0)>0;
  if($('config-save')){$('config-save').disabled=false;$('config-save').title=hasActive?'Zmiana ustawień wymagających przebudowy konfiguracji wyświetli prośbę o rozłączenie':'';}
  state.snapshot=s;syncMailboxView(s);renderNotifications(s);if(!state.configText&&$('config-text')?.value)state.configText=$('config-text').value;$('callsign').textContent=s.callsign||'NIE USTAWIONO';$('brand-callsign').textContent=s.callsign||'—';$('operator').textContent=s.operator||'Operator stacji';$('setup-note').hidden=!!s.callsign;
  const online=s.interfaces.filter(i=>i.online).length,active=s.sessions.filter(i=>i.state!=='Idle').length;
  $('tnc-stat').replaceChildren(document.createTextNode(`${online} `),node('em','',`/ ${s.interfaces.length}`));$('tnc-caption').textContent=online?'Połączone z TNC':'Brak połączenia z TNC';$('session-stat').replaceChildren(document.createTextNode(`${active} `),node('em','','/ 8'));$('rx-stat').textContent=s.rx;$('tx-stat').textContent=s.tx;$('uptime').textContent=`Czas pracy: ${Math.floor(s.uptime/3600)}h ${Math.floor(s.uptime/60)%60}m`;
  const session=s.sessions[state.selected];
  s.sessions.forEach((v,i)=>{const b=$(`tab-${i}`);const last=v.history.at(-1)?.id||0;if(i===state.selected)state.seen[i]=last;const key=JSON.stringify({call:v.call,state:v.state,last,selected:i===state.selected,seen:state.seen[i]});if(b.dataset.renderKey!==key){b.classList.toggle('active',i===state.selected);b.classList.toggle('unread',i!==state.selected&&last>state.seen[i]);b.setAttribute('aria-selected',String(i===state.selected));b.replaceChildren(node('span','session-label','Sesja '+(i+1)),node('small','session-peer',v.call||'wolna'));b.title=v.call?`${v.call} · ${v.state}`:'Wolna sesja';b.dataset.renderKey=key;}});
  $('peer').textContent=session.call||'Brak połączenia';$('peer-state').textContent={Idle:'Wybierz stację, aby rozpocząć',Connected:'Połączono',Connecting:'Łączenie…',Negotiating:'Negocjacja AX.25…',Disconnecting:'Rozłączanie…'}[session.state]||session.state;$('session-dot').className='dot'+(session.state==='Connected'?' green':'');
  $('disconnect').hidden=session.state==='Idle';$('message').disabled=session.state!=='Connected'||!!session.transfer;$('send-button').disabled=$('message').disabled;$('upload-button').disabled=$('message').disabled||!bool(s.yappEnabled);$('queue').textContent=`Kolejka TX: ${session.queued} B`;
  const key=JSON.stringify(session.history)+`|${state.language}`;if(key!==state.historyKey){const h=$('history');const bottom=h.scrollHeight-h.scrollTop-h.clientHeight<60;state.historyKey=key;if(session.history.length){rows(h,session.history.map(l=>node('div',l.kind,l.kind==='info'?translateSessionStatus(l.text||''):l.text||' ')));}else{const e=node('div','empty');const mark=node('div','empty-mark');mark.innerHTML='<svg class="icon" aria-hidden="true"><use href="/icons.svg#console"></use></svg>';e.append(mark,node('h3','','Gotowy na nową łączność'),node('p','','Połącz się ze stacją lub poczekaj na połączenie przychodzące.'));rows(h,[e]);}if(bottom)h.scrollTop=h.scrollHeight;}
  $('transfer').hidden=!session.transfer;if(session.transfer){const t=session.transfer;const items=[node('span','',t.confirm?'Oczekiwanie na zgodę na odbiór YAPP':`${t.sending?'↑':'↓'} ${t.name||'YAPP'} · ${t.done} / ${t.size} B`)];if(t.confirm){items.push(button('Przyjmij',()=>request('yapp/accept',{body:{session:state.selected,accept:true}})),button('Odrzuć',()=>request('yapp/accept',{body:{session:state.selected,accept:false}})));}else{const p=node('progress');p.max=t.size||1;p.value=t.done;items.push(p,button('Anuluj',()=>request('yapp/cancel',{body:{session:state.selected}})));}rows($('transfer'),items);}
  renderBroadcastInterfaces(s.interfaces);renderInterfacesTable($('interfaces'),s.interfaces);
  $('heard-count').textContent=s.heard.length;$('mail-count').textContent=s.mail.filter(m=>!m.read).length;
  renderHeard(s);
  $('mail-status').textContent=bool(s.mailboxEnabled)?`RADIO ON · ${s.mailboxSessions}/2`:'RADIO OFF';renderMail(s);
  if(!state.frozen)renderMonitor(s);
  if(!state.frozen)renderMonitorView(s);
}
function monitorRows(data){return data.length?data.slice().reverse().map(l=>node('div',l.kind,l.text)):[node('div','','Brak ramek. Połącz TNC, aby obserwować ruch.')];}
function renderMonitor(s){const data=s.monitor;const key=JSON.stringify(data);if(key===state.monitorKey){renderMonitorDialog(s);return;}state.monitorKey=key;rows($('monitor'),monitorRows(data));renderMonitorDialog(s);}
function renderMonitorView(s){const data=s.monitor;const body=$('monitor-view-body');if(!body)return;const key=JSON.stringify(data);if(body.dataset.renderKey===key)return;rows(body,monitorRows(data));body.dataset.renderKey=key;}
function renderMonitorDialog(s){const body=$('monitor-dialog-body');if(!body)return;const data=s.monitor;rows(body,data.length?data.slice().reverse().map(l=>node('div',l.kind,l.text)):[node('div','','Brak ramek. Połącz TNC, aby obserwować ruch.')]);}
const heardExpanded=new Set();
function heardEntryKey(entry){return JSON.stringify([entry.call,entry.interface,entry.port??0]);}
function heardUsedDigis(heard){return String(heard?.via||'').split(/\s+/).filter(item=>item.endsWith('*')).map(item=>item.replace(/\*+$/,''));}
function heardForCall(call,interfaceId){const wanted=(call||'').trim().toUpperCase();return wanted?(state.snapshot?.heard||[]).find(item=>String(item.call||'').toUpperCase()===wanted&&(interfaceId===undefined||String(item.interface)===String(interfaceId))):null;}
function reverseHeardRoute(heard){return heardUsedDigis(heard).reverse().join(' ');}
function renderHeard(snapshot){
  const pl=state.language==='pl',key=JSON.stringify({heard:snapshot.heard,interfaces:snapshot.interfaces.map(item=>({id:item.id,online:item.online,name:item.name})),language:state.language});
  const headings=pl?['Stacja','Interfejs','Ramki','Ostatnio','Cel','Trasa do stacji','']:['Station','Interface','Frames','Last heard','Destination','Route to station',''];
  document.querySelectorAll('.heard-stations-table th').forEach((heading,index)=>heading.textContent=headings[index]);
  const foot=$('view-heard').querySelector('.panel-foot');
  foot.textContent=pl?'Źródło ramki może znajdować się za digipeaterem. Proponowana trasa nie gwarantuje zestawienia połączenia.':'The frame source may be behind digipeaters. The suggested route does not guarantee a connection.';
  const activeKeys=new Set(snapshot.heard.map(heardEntryKey));
  for(const expanded of heardExpanded)if(!activeKeys.has(expanded))heardExpanded.delete(expanded);
  const interfaceFor=entry=>snapshot.interfaces.find(item=>String(item.id)===String(entry.interface));
  if($('heard-preview').dataset.renderKey!==key){
    rows($('heard-preview'),snapshot.heard.length?snapshot.heard.slice(0,4).map(entry=>{
      const item=node('div','heard-item'),iface=interfaceFor(entry),available=Boolean(iface?.online),content=node('div');
      const station=node('strong',available?'station-dblclick':'',entry.call),lastDigi=heardUsedDigis(entry).at(-1);
      station.title=available?(pl?'Kliknij dwukrotnie, aby połączyć':'Double-click to connect'):(pl?'TNC niedostępny':'TNC unavailable');
      if(available)station.ondblclick=()=>connectDirect(entry.call,entry.interface,entry);
      const address=node('div','heard-address');address.append(station);
      if(lastDigi)address.append(node('span','heard-last-digi',` via ${lastDigi}`));
      content.append(address,node('small',available?'':'interface-error',available?iface.name:(pl?'TNC niedostępny':'TNC unavailable')));
      item.append(content,node('small','',formatHeardAge(entry.age)));return item;
    }):[node('div','quiet-empty',pl?'Oczekiwanie na pierwsze ramki':'Waiting for the first frames')]);
    $('heard-preview').dataset.renderKey=key;
  }
  if($('heard-table').dataset.renderKey===key)return;
  rows($('heard-table'),snapshot.heard.flatMap((entry,index)=>{
    const tr=node('tr','heard-station-row'),detail=node('tr','heard-frame-row'),entryKey=heardEntryKey(entry),iface=interfaceFor(entry),available=Boolean(iface?.online);
    const route=reverseHeardRoute(entry),station=node('td'),toggle=node('button','heard-toggle');toggle.type='button';
    detail.id=`heard-frame-${index}`;toggle.className='heard-toggle';toggle.setAttribute('aria-controls',detail.id);
    const setExpanded=expanded=>{detail.hidden=!expanded;toggle.textContent=`${expanded?'▾':'▸'} ${entry.call}`;toggle.setAttribute('aria-expanded',String(expanded));if(expanded)heardExpanded.add(entryKey);else heardExpanded.delete(entryKey);};
    setExpanded(heardExpanded.has(entryKey));toggle.onclick=()=>setExpanded(detail.hidden);station.append(toggle);
    tr.onclick=event=>{if(!event.target.closest('button'))setExpanded(detail.hidden);};
    tr.append(station,node('td',available?'':'interface-error',available?iface.name:(pl?'TNC niedostępny':'TNC unavailable')),node('td','',entry.frames),node('td','',formatHeardAge(entry.age).replace(' temu','').replace(' ago','')),node('td','',entry.destination));
    const routeCell=node('td','heard-return-route');
    if(route)route.split(' ').forEach((call,i)=>routeCell.append(node('span','',`${i?'→ ':''}${call}`),document.createTextNode(' ')));
    else routeCell.textContent=pl?'Bezpośrednio':'Direct';
    const actionCell=node('td'),action=button(pl?'Połącz ↗':'Connect ↗',()=>connectDirect(entry.call,entry.interface,entry));action.disabled=!available;action.title=available?(pl?'Połącz przez zapamiętany interfejs i trasę':'Connect using the recorded interface and route'):(pl?'TNC niedostępny':'TNC unavailable');actionCell.append(action);tr.append(routeCell,actionCell);
    const details=node('td');details.colSpan=7;
    details.append(node('div','heard-frame-heading',`${pl?'Ostatnia odebrana ramka':'Last received frame'} · ${entry.utc||'—'} UTC · ${iface?.name||entry.interface} / port ${entry.port??0}`));
    details.append(node('pre','heard-last-frame rx',entry.last_frame|| (pl?'Podgląd niedostępny po restarcie — oczekiwanie na kolejną ramkę.':'Preview unavailable after restart — waiting for the next frame.')));
    detail.append(details);return [tr,detail];
  }));
  $('heard-table').dataset.renderKey=key;
}
function findConnectRoute(){const status=$('connect-route-status'),call=$('connect-call').value.trim();if(!call){status.textContent='Wpisz znak stacji.';$('connect-call').focus();return;}const heard=heardForCall(call);if(!heard){status.textContent='Trasa do stacji jest nieznana. Wprowadź ręcznie parametry.';return;}const iface=(state.snapshot?.interfaces||[]).find(item=>String(item.id)===String(heard.interface));if(!iface||!iface.online){status.textContent='Trasa do stacji jest nieznana. Wprowadź ręcznie parametry.';return;}$('connect-interface').value=String(iface.id);const route=reverseHeardRoute(heard);$('connect-via').value=route;status.textContent=route?`Znaleziono trasę: ${iface.name} · ${route}`:`Znaleziono interfejs: ${iface.name}`;}
function openConnect(call='',route){$('connect-call').value=call;$('connect-via').value='';$('connect-route-status').textContent='';rows($('connect-interface'),(state.snapshot?.interfaces||[]).map(i=>{const o=node('option','',`${i.name}${i.online?'':' (offline)'}`);o.value=i.id;o.disabled=!i.online;return o;}));const preferred=route??state.snapshot?.interfaces.find(i=>i.online)?.id;if(preferred!==undefined)$('connect-interface').value=preferred;$('connect-dialog').showModal();$('connect-call').focus();}
function connectDirect(call,route,entry){guard(async()=>{const interfaces=state.snapshot?.interfaces||[],heard=entry??heardForCall(call,route),preferred=route!==undefined?interfaces.find(item=>String(item.id)===String(route)&&item.online):interfaces.find(item=>heard&&String(item.id)===String(heard.interface)&&item.online)||interfaces.find(item=>item.online);if(!preferred)throw Error('Brak dostępnego interfejsu TNC.');const r=await request('connect',{body:{session:state.selected,call:call.trim(),interface:preferred.id,via:reverseHeardRoute(heard?.interface===preferred.id?heard:heardForCall(call,preferred.id))}});selectSession(r.session);showView('terminal');toast(`Rozpoczęto łączenie z ${call.trim()}`);});}
$('connect-open').onclick=()=>openConnect();$('connect-find-route').onclick=findConnectRoute;$('connect-form').onsubmit=e=>{e.preventDefault();guard(async()=>{const r=await request('connect',{body:{session:state.selected,call:$('connect-call').value.trim(),interface:$('connect-interface').value,via:$('connect-via').value}});selectSession(r.session);$('connect-dialog').close();toast('Rozpoczęto łączenie');});};
$('disconnect').onclick=()=>guard(()=>request('disconnect',{body:{session:state.selected}}));$('clear').onclick=()=>guard(()=>request('clear',{body:{session:state.selected}}));
$('send-form').onsubmit=e=>{e.preventDefault();guard(async()=>{const id=state.selected,text=$('message').value;if(!text.trim())return;await request('send',{body:{session:id,text}});state.drafts[id]='';if(state.selected===id&&$('message').value===text)$('message').value='';});};
$('message').oninput=()=>state.drafts[state.selected]=$('message').value;
$('beacon').onclick=()=>guard(async()=>{await request('beacon');toast('Beacon dodany do kolejki TNC');});$('retry').onclick=()=>guard(()=>request('retry'));
  $('freeze').onclick=()=>{state.frozen=!state.frozen;$('freeze').textContent=state.frozen?'Wznów widok':'Wstrzymaj widok';$('monitor-view-freeze')?.replaceChildren(document.createTextNode(state.frozen?'Wznów widok':'Wstrzymaj widok'));document.querySelectorAll('.live-label,.monitor-view-live').forEach(el=>el.textContent=state.frozen?'PAUSED':'● LIVE');if(!state.frozen&&state.snapshot){renderMonitor(state.snapshot);renderMonitorView(state.snapshot);}};
function download(data,name,type='text/plain;charset=utf-8'){const blob=data instanceof Blob?data:new Blob([data],{type});const url=URL.createObjectURL(blob),a=node('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
async function saveExport(route,name,successMessage,type='text/plain;charset=utf-8'){
  const extension=name.includes('.')?`.${name.split('.').pop().toLowerCase()}`:'.txt';
  if(typeof window.showSaveFilePicker==='function'){
    let handle;
    try{
      handle=await window.showSaveFilePicker({
        suggestedName:name,
        types:[{description:'Plik tekstowy',accept:{'text/plain':[extension]}}]
      });
    }catch(exception){
      if(exception?.name==='AbortError')return;
      throw exception;
    }
    const blob=new Blob([await request(route)],{type});
    const writable=await handle.createWritable();
    try{await writable.write(blob);}finally{await writable.close();}
  }else{
    download(await request(route),name,type);
  }
  toast(successMessage);
}
$('export').onclick=()=>guard(()=>saveExport('monitor/export','station-monitor.txt','Eksport monitora zapisany.'));
function renderBroadcastInterfaces(items){const select=$('monitor-broadcast-interface');if(!select)return;const current=select.value;select.replaceChildren(...(items||[]).map(i=>{const option=node('option','',`${i.name}${i.online?'':' (offline)'}`);option.value=i.id;option.disabled=!i.online;return option;}));if((items||[]).some(i=>String(i.id)===current))select.value=current;else{const preferred=(items||[]).find(i=>i.online);if(preferred)select.value=String(preferred.id);}}
function field(id,value){const element=$(id);if(element)element.value=value??'';}
function bool(value){return value===true||value===1||String(value).toLowerCase()==='true';}
function checked(id,value){if($(id))$(id).checked=bool(value);}
function renderInterfaceForms(items){const root=$('interface-forms');root.replaceChildren();(items||[]).forEach((item,index)=>root.append(interfaceForm(item,index)));decorateSettingsControls();loadSerialPorts();}
function syncInterfaceTransport(card){const transport=card.querySelector('.if-transport')?.value||'tcp';const baud=card.querySelector('.if-baud')?.closest('label');const endpoint=card.querySelector('.if-endpoint');const devices=card.querySelector('.if-device-select');const endpointLabel=endpoint?.closest('label');const endpointCaption=endpointLabel?.querySelector('span');const serial=transport==='serial';if(baud)baud.hidden=!serial;if(endpoint)endpoint.hidden=serial;if(devices)devices.hidden=!serial;if(endpointCaption)endpointCaption.textContent=serial?(state.language==='pl'?'Urządzenie szeregowe':'Serial device'):(state.language==='pl'?'Adres TCP/IP (host:port)':'TCP/IP address (host:port)');if(endpointLabel)endpointLabel.dataset.help=serial?(state.language==='pl'?'Wybierz wykryte urządzenie szeregowe używane przez TNC.':'Choose the detected serial device used by the TNC.'):(state.language==='pl'?'Adres host:port zdalnego lub lokalnego TNC.':'Host:port address of the remote or local TNC.');if(serial&&devices?.value)endpoint.value=devices.value;card.classList.toggle('serial-transport',serial);card.classList.toggle('tcp-transport',!serial);}
async function loadSerialPorts(){try{const ports=await request('serial/ports');document.querySelectorAll('#interface-forms > .interface-form').forEach(card=>{const select=card.querySelector('.if-device-select'),endpoint=card.querySelector('.if-endpoint');if(!select||!endpoint)return;const current=endpoint.value;const configuredSerial=/^COM\d+$/i.test(current)||current.startsWith('/dev/');select.replaceChildren();(ports||[]).forEach(port=>{const option=node('option','',port);option.value=port;select.append(option);});if(configuredSerial&&!ports.includes(current)){const option=node('option','',`${current} (konfiguracja)`);option.value=current;select.append(option);}if(!select.options.length){const option=node('option','', 'Brak wykrytych urządzeń');option.disabled=true;select.append(option);}select.value=(configuredSerial?current:'')||select.options[0]?.value||'';syncInterfaceTransport(card);});}catch{}}
function interfaceToggleText(expanded){return state.language==='pl'?(expanded?'Zwiń':'Rozwiń'):(expanded?'Collapse':'Expand');}
function refreshInterfaceToggles(){document.querySelectorAll('.interface-form').forEach(card=>{const toggle=card.querySelector('.interface-toggle');if(toggle)toggle.textContent=interfaceToggleText(!card.classList.contains('collapsed'));});}
function scrollExpandedSettingsCard(card){
  requestAnimationFrame(()=>requestAnimationFrame(()=>{
    const rect=card.getBoundingClientRect();
    const desired=window.scrollY+rect.top-Math.max(0,(window.innerHeight-rect.height)/2);
    const documentElement=document.documentElement;
    const maxBefore=Math.max(0,documentElement.scrollHeight-window.innerHeight);
    if(desired>maxBefore){
      const content=card.closest('.settings-form')?.querySelector(':scope > .panel-content');
      if(content){
        let spacer=content.querySelector(':scope > .settings-scroll-spacer');
        if(!spacer){spacer=node('div','settings-scroll-spacer');spacer.setAttribute('aria-hidden','true');content.append(spacer);}
        spacer.style.height=`${Math.ceil(desired-maxBefore+24)}px`;
      }
    }
    const maxScroll=Math.max(0,documentElement.scrollHeight-window.innerHeight);
    window.scrollTo({top:Math.min(Math.max(0,desired),maxScroll),behavior:'smooth'});
  }));
}
function interfaceForm(item={},index=0){const card=node('div','interface-form');card.dataset.index=index;const title=node('div','section-title');const toggle=node('button','text-button interface-toggle',interfaceToggleText(false));toggle.type='button';toggle.setAttribute('aria-expanded','false');const strong=node('strong','',`Interfejs ${index+1}`);const summary=node('span','interface-name-summary',item.name||`TNC${index+1}`);const toggleCard=event=>{if(event.target.closest('button'))return;toggle.click();};toggle.onclick=()=>{const collapsed=card.classList.toggle('collapsed');toggle.textContent=interfaceToggleText(!collapsed);toggle.setAttribute('aria-expanded',String(!collapsed));if(!collapsed)scrollExpandedSettingsCard(card);};title.addEventListener('click',toggleCard);title.append(strong,summary,toggle);const remove=node('button','text-button','Usuń');remove.type='button';remove.onclick=()=>{card.remove();[...$('interface-forms').children].forEach((n,i)=>n.querySelector('strong').textContent=`Interfejs ${i+1}`);};title.append(remove);card.append(title);const body=node('div','interface-body');const grid=node('div','form-grid');const add=(label,id,type='text',value='',placeholder='')=>{const wrap=node('label','');wrap.append(node('span','',label));const input=node(type==='select'?'select':'input');input.id=`${id}-${index}`;input.className=id;if(type==='select'){[['tcp','TCP/IP'],['serial','Serial']].forEach(([v,t])=>{const o=node('option','',t);o.value=v;input.append(o);});input.value=value||'tcp';}else{input.type=type;input.value=value??'';if(placeholder)input.placeholder=placeholder;}wrap.append(input);grid.append(wrap);return input;};const name=add('Nazwa','if-name','text',item.name||'','np. VHF');name.maxLength=16;name.addEventListener('input',()=>{summary.textContent=name.value.trim()||`TNC${index+1}`;});add('Opis','if-description','text',item.description||'');const transport=add('Transport','if-transport','select',item.transport||'tcp');const endpoint=add('Adres / urządzenie','if-endpoint','text',item.endpoint||'','np. 127.0.0.1:8001 lub /dev/ttyUSB0');const endpointLabel=endpoint.closest('label');const devices=document.createElement('select');devices.className='if-device-select';devices.setAttribute('aria-label','Dostępne urządzenie szeregowe');devices.onchange=()=>{endpoint.value=devices.value;};endpointLabel.append(devices);add('Prędkość portu (baud)','if-baud','number',item.baud||9600);add('Port KISS','if-port','number',item.port??0);add('Priorytet','if-priority','number',item.priority??index);transport.onchange=()=>syncInterfaceTransport(card);body.append(grid);const checks=node('div','check-grid');const en=node('label');const ei=node('input');ei.type='checkbox';ei.className='if-enabled';ei.checked=item.enabled!==false;en.append(ei,' Włączony');checks.append(en);const be=node('label');const bi=node('input');bi.type='checkbox';bi.className='if-beacon';bi.checked=!!item.beaconEnabled;be.append(bi,' Beacon na interfejsie');checks.append(be);body.append(checks);const path=node('label','');path.append(node('span','', 'Ścieżka beaconu (opcjonalnie)'));const pi=node('input');pi.className='if-path';pi.value=item.beaconPath||'';path.append(pi);body.append(path);card.append(body);card.classList.add('collapsed');syncInterfaceTransport(card);return card;}
$('template-select').onchange=()=>$('template-text').value=state.templates[Number($('template-select').value)]||'';
$('template-save').onclick=()=>guard(async()=>{const index=$('template-select').value,text=$('template-text').value;await request('templates/save',{body:{index,text}});state.templates[index]=text;toast('Szablon zapisany');});
$('mail-new').onclick=()=>{$('mail-form').reset();$('mail-dialog').showModal();};
$('mail-form').onsubmit=e=>{e.preventDefault();guard(async()=>{await request('mail/create',{body:{to:$('mail-to').value.trim(),kind:$('mail-kind').value,subject:$('mail-subject').value,body:$('mail-body').value}});$('mail-dialog').close();toast('Wiadomość zapisana');});};
async function readMail(id){const m=await request('mail/read',{body:{id}});$('read-subject').textContent=m.subject;$('read-from').textContent=`Od: ${m.from}`;$('read-body').textContent=m.body;$('read-dialog').showModal();$('mail-reply').onclick=()=>{$('read-dialog').close();$('mail-form').reset();$('mail-to').value=m.from;$('mail-subject').value=`Re: ${m.subject}`.slice(0,80);$('mail-dialog').showModal();};}
$('upload-button').onclick=()=>$('upload').click();$('upload').onchange=()=>guard(async()=>{const f=$('upload').files[0];if(!f)return;if(f.size>16*1024*1024)throw Error('Limit wysyłania przez WWW: 16 MiB');await request('yapp/send',{raw:f,query:{session:state.selected,name:f.name}});$('upload').value='';toast('Rozpoczęto transfer YAPP');});

function renderMail(snapshot){
  const folderBar=document.querySelector('#view-mail .mail-folders');
  const allFolder=folderBar?.querySelector('[data-mail-folder="all"]');
  if(folderBar&&allFolder)folderBar.prepend(allFolder);
  const messages=Array.isArray(snapshot?.mail)?snapshot.mail:[];
  const folder=state.mailFolder||'inbox';
  const localCall=String(snapshot?.callsign||'').toUpperCase();
  const counts={inbox:0,sent:0,all:messages.length,unread:0};
  messages.forEach(message=>{if(!message.read)counts.unread++;if(localCall&&String(message.to||'').toUpperCase()===localCall)counts.inbox++;if(localCall&&String(message.from||'').toUpperCase()===localCall)counts.sent++;});
  Object.entries(counts).forEach(([key,value])=>{const el=$(`mail-${key}-count`);if(el)el.textContent=value;});
  document.querySelectorAll('#view-mail [data-mail-folder]').forEach(button=>button.classList.toggle('active',button.dataset.mailFolder===folder));
  const value=id=>($(id)?.value||'').trim().toLowerCase();
  const search=value('mail-search'),from=value('mail-filter-from'),to=value('mail-filter-to'),subject=value('mail-filter-subject'),read=$('mail-filter-read')?.value||'all';
  const visible=messages.filter(message=>{
    const sender=String(message.from||'').toLowerCase(),recipient=String(message.to||'').toLowerCase(),title=String(message.subject||'').toLowerCase(),body=String(message.body||'').toLowerCase();
    if(folder==='inbox'&&localCall&&recipient!==localCall)return false;
    if(folder==='sent'&&localCall&&sender!==localCall)return false;
    if(folder==='unread'&&message.read)return false;
    if(search&&!`${sender} ${recipient} ${title} ${body}`.includes(search))return false;
    if(from&&!sender.includes(from))return false;if(to&&!recipient.includes(to))return false;if(subject&&!title.includes(subject))return false;
    if(read==='unread'&&message.read)return false;if(read==='read'&&!message.read)return false;return true;
  });
  const table=$('mail-table');if(!table)return;const renderKey=JSON.stringify({messages,folder,search,from,to,subject,read});
  if(table.dataset.renderKey!==renderKey){rows(table,visible.length?visible.map(message=>{const row=node('tr',message.read?'':'unread');row.append(node('td','',message.read?'':'•'),node('td','',message.from||'—'),node('td','',message.to||'—'));const title=node('td');title.append(button(message.subject||'(bez tematu)',()=>readMail(message.id)));row.append(title,node('td','',message.kind||'—'),node('td','',message.time?new Date(Number(message.time)*1000).toLocaleString():''));const actions=node('td');actions.append(button('Podgląd',()=>readMail(message.id)),button('Usuń',async()=>{if(confirm(`Usunąć wiadomość #${message.id}?`)){await request('mail/delete',{body:{id:message.id}});if(state.snapshot)render(state.snapshot);}}));row.append(actions);return row;}):[Object.assign(node('tr'),{innerHTML:'<td colspan="7" class="quiet-empty">Brak wiadomości dla wybranych filtrów.</td>'})]);table.dataset.renderKey=renderKey;}
  if(!$('mail-table').dataset.bound){$('mail-table').dataset.bound='1';document.querySelectorAll('#view-mail [data-mail-folder]').forEach(button=>button.onclick=()=>{state.mailFolder=button.dataset.mailFolder;if(state.snapshot)renderMail(state.snapshot);});['mail-search','mail-filter-from','mail-filter-to','mail-filter-subject','mail-filter-read'].forEach(id=>$(id)?.addEventListener('input',()=>state.snapshot&&renderMail(state.snapshot)));$('mail-filter-clear')?.addEventListener('click',()=>{['mail-search','mail-filter-from','mail-filter-to','mail-filter-subject'].forEach(id=>field(id,''));if($('mail-filter-read'))$('mail-filter-read').value='all';if(state.snapshot)renderMail(state.snapshot);});$('mail-new')?.addEventListener('click',()=>{$('mail-form')?.reset();$('mail-dialog')?.showModal();});}
}

function notificationSetting(kind,channel,value){const services={mailbox:'settings-mailbox',yapp:'settings-yapp'};return services[kind]&&!$(services[kind])?.checked?(channel==='sound'?'0':false):value;}
function updateModuleNotificationState(){const services={mailbox:'settings-mailbox',yapp:'settings-yapp'};Object.entries(services).forEach(([kind,id])=>{const enabled=$(id)?.checked===true;const row=document.querySelector('[data-notification-kind="'+kind+'"]');row?.classList.toggle('is-disabled',!enabled);row?.querySelectorAll('input,select').forEach(control=>{control.disabled=!enabled;});});}
document.addEventListener('change',event=>{if(['settings-mailbox','settings-yapp'].includes(event.target?.id))updateModuleNotificationState();});
$('interface-add').onclick=()=>{const items=[...document.querySelectorAll('#interface-forms > .interface-form')];const card=interfaceForm({},items.length);$('interface-forms').append(card);card.classList.remove('collapsed');const toggle=card.querySelector('.interface-toggle');if(toggle){toggle.textContent=interfaceToggleText(true);toggle.setAttribute('aria-expanded','true');}scrollExpandedSettingsCard(card);card.querySelector('.if-name')?.focus({preventScroll:true});loadSerialPorts();};
function activeSessions(snapshot=state.snapshot){return Boolean(snapshot?.sessions?.some(session=>session.state!=='Idle')||Number(snapshot?.mailboxSessions||0)>0);}
async function ensureStationSettingsLoaded(){if(state.configText)return;await loadSettings();}
function ensureConfigRestartDialog(){
  if($('config-restart-dialog'))return $('config-restart-dialog');
  const dialog=node('dialog');dialog.id='config-restart-dialog';
  dialog.innerHTML='<div class="dialog-head"><h2>Wymagane rozłączenie</h2><button type="button" class="close" aria-label="Zamknij"><svg class="icon" aria-hidden="true"><use href="/icons.svg#close-circle-outline"></use></svg></button></div><p>Zmieniono krytyczne dane konfiguracji. Zapisanie wymaga rozłączenia aktywnych sesji terminala lub MailBoxa.</p><div class="actions"><button type="button" class="button primary" data-config-restart-confirm>Rozłącz i zapisz</button><button type="button" class="button secondary" data-config-restart-cancel>Anuluj</button></div>';
  document.body.append(dialog);return dialog;
}
function confirmConfigRestart(){const dialog=ensureConfigRestartDialog();return new Promise(resolve=>{const finish=value=>{dialog.close();resolve(value);};dialog.querySelector('[data-config-restart-confirm]').onclick=()=>finish(true);dialog.querySelector('[data-config-restart-cancel]').onclick=()=>finish(false);dialog.querySelector('.close').onclick=()=>finish(false);dialog.showModal();});}
async function waitForSessionsClosed(){for(let attempt=0;attempt<40;attempt++){const snapshot=await request('state',{method:'GET'});state.snapshot=snapshot;if(!activeSessions(snapshot))return;await new Promise(resolve=>setTimeout(resolve,250));}throw Error('Sesje nie zostały rozłączone w wymaganym czasie. Konfiguracja nie została zapisana.');}
async function saveStationConfiguration(force=false){
  await ensureStationSettingsLoaded();
  validateSettings();
  // Keep older notification rows compatible with the canonical station.conf
  // key. Some previously cached UI code used `in-app` in the key name;
  // normalize it before sending so one stale fragment cannot reject the full
  // settings form.
  const text=settingsConfigText().replace(/(notification_[^=\r\n]+)-in-app=/g,'$1_in_app=');
  const impact=await request('config/check',{body:{text}});
  if(bool(impact.requiresDisconnect)&&!force){if(!await confirmConfigRestart())return false;await request('disconnect-all',{body:{}});await waitForSessionsClosed();}
  $('config-text').value=text;
  await request('config/save',{body:{text}});
  state.configText=text;
  await loadLanguage($('settings-language').value);
  return true;
}
function settingsSnapshotReady(snapshot){
  if(!snapshot)return false;
  const same=(left,right)=>String(left??'')===String(right??'');
  const expectedInterfaces=[...document.querySelectorAll('#interface-forms > .interface-form .if-name')].map(input=>input.value.trim());
  const actualInterfaces=(snapshot.interfaces||[]).map(item=>String(item.name||'').trim());
  return same(snapshot.callsign,$('settings-callsign')?.value.trim())
    && bool(snapshot.mailboxEnabled)===$('settings-mailbox')?.checked
    && bool(snapshot.yappEnabled)===$('settings-yapp')?.checked
    && expectedInterfaces.length===actualInterfaces.length
    && expectedInterfaces.every((name,index)=>name===actualInterfaces[index]);
}
async function refreshConfigurationUi(){
  await loadSettings();
  let refreshed;
  for(let attempt=0;attempt<10;attempt++){
    refreshed=await request('state',{method:'GET'});
    if(settingsSnapshotReady(refreshed)||attempt===9)break;
    await new Promise(resolve=>setTimeout(resolve,150));
  }
  render(refreshed);
  await syncStatsView();
}async function saveAllConfiguration(force=false){
  if(state.configSaveInProgress)return false;
  state.configSaveInProgress=true;
  try{const saved=await saveStationConfiguration(force);if(!saved)return false;const webResult=await saveWebConfiguration();await refreshConfigurationUi();collapseSettingsSections();toast(webResult?.restartRequired==='true'||webResult?.restartRequired===true?'Ustawienia zapisane. Zrestartuj aplikację, aby zastosować zmiany dostępu WWW.':'Ustawienia zapisane.');return true;}finally{state.configSaveInProgress=false;}
}
$('config-save').onclick=()=>guard(()=>saveAllConfiguration(false));
$('settings-language').addEventListener('change',()=>guard(()=>loadLanguage($('settings-language').value)));
$('config-export').onclick=()=>guard(()=>saveExport('config','station.conf','Konfiguracja wyeksportowana.'));
const settingsHelp={
  'settings-callsign':['Znak stacji używany w adresowaniu AX.25.','Station callsign used for AX.25 addressing.'],
  'settings-myname':['Czytelna nazwa widoczna w informacjach i wiadomościach.','Friendly name shown in station information and messages.'],
  'settings-language':['Język interfejsu WWW; zmiana działa od razu.','Web interface language; changes apply immediately.'],
  'settings-web-bind':['Adres i port, na którym serwer WWW przyjmuje połączenia.','Address and port where the web server accepts connections.'],
  'settings-web-token-file':['Opcjonalny plik tokenu operatora.','Optional operator token file.'],
  'settings-web-timeout':['W trybie Auto brak sygnałów obecności z przeglądarki (heartbeat) przez ten czas oznacza nieobecność operatora.','In Auto mode, no browser heartbeat for this duration marks the operator as absent.'],
  'settings-t1':['Czas oczekiwania na potwierdzenie ramki AX.25.','AX.25 frame acknowledgement timeout.'],
  'settings-t3':['Interwał kontroli bezczynnego połączenia AX.25.','Idle AX.25 connection check interval.'],
  'settings-retries':['Maksymalna liczba ponowień transmisji.','Maximum number of transmission retries.'],
  'settings-paclen':['Maksymalny rozmiar danych w jednej ramce.','Maximum payload size in one frame.'],
  'settings-beacon-seconds':['Odstęp między beaconami w minutach.','Interval between beacons in minutes.'],
  'settings-beacon-to':['Adres docelowy dla beaconów.','Destination address for beacons.'],
  'settings-mailbox-call':['Znak wywoławczy używany przez MailBox.','Callsign used by the MailBox.'],
  'settings-yapp-dir':['Katalog zapisu odebranych plików YAPP.','Directory for received YAPP files.'],
  'settings-welcome':['Wysyła wiadomość powitalną po połączeniu.','Sends the welcome message after connection.'],
  'settings-goodbye':['Wysyła wiadomość pożegnalną przed zamknięciem sesji.','Sends the goodbye message before closing a session.'],
  'settings-remote':['Pozwala używać zdalnych komend /help, /info, /ports i /mh.','Allows remote /help, /info, /ports and /mh commands.'],
  'settings-beacon':['Włącza okresową identyfikację stacji.','Enables periodic station identification frames.'],
  'settings-mailbox':['Włącza odbiór i przechowywanie wiadomości.','Enables receiving and storing messages.'],
  'settings-yapp':['Włącza transfer plików YAPP.','Enables YAPP file transfers.'],
  'settings-yapp-auto':['Automatycznie przyjmuje pliki YAPP.','Automatically accepts YAPP files.'],
  'settings-yapp-location':['Pyta o lokalizację przy zapisie pliku.','Asks for a location when saving a file.']
};
const settingsSectionHelp=[['Podstawowe dane stacji i język interfejsu.','Station identity and interface language.'],['Adres nasłuchu i ochrona dostępu panelu WWW.','Listen address and web panel access protection.'],['Połączenia TCP/Serial z TNC i ich priorytety.','TCP/Serial TNC connections and priorities.'],['Parametry transmisji i połączeń AX.25.','Transmission and AX.25 connection parameters.'],['Usługi dostępne operatorowi i stacjom zdalnym.','Services available to the operator and remote stations.']];
function settingsLanguage(){return state.language==='pl'?'pl':'en';}
function setHelpDot(label,text){if(!label)return;let dot=label.querySelector(':scope > .help-dot');if(!dot){dot=node('button','help-dot');dot.type='button';dot.append(svgIcon('help-circle-outline'));dot.addEventListener('mousedown',event=>event.preventDefault());dot.addEventListener('click',event=>event.preventDefault());label.append(dot);}else if(!dot.querySelector('.icon')){dot.append(svgIcon('help-circle-outline'));}dot.dataset.help=text;dot.setAttribute('aria-label',text);dot.title=text;label.dataset.help=text;label.removeAttribute('title');}
function decorateSettingsControls(){const lang=settingsLanguage();Object.entries(settingsHelp).forEach(([id,text])=>{const label=$(id)?.closest('label');if(label)setHelpDot(label,text[lang==='pl'?0:1]);});document.querySelectorAll('#interface-forms > .interface-form label').forEach(label=>{const control=label.querySelector('input,select');if(!control)return;const text={'if-name':['Nazwa używana przez komendy /ports i konfigurację.','Name used by /ports commands and configuration.'],'if-description':['Opis interfejsu widoczny w panelu.','Interface description shown in the panel.'],'if-transport':['Wybierz TCP dla sieci lub Serial dla urządzenia szeregowego.','Choose TCP for network or Serial for a serial device.'],'if-endpoint':['Adres host:port albo ścieżka urządzenia szeregowego.','Host:port address or serial device path.'],'if-baud':['Prędkość portu szeregowego.','Serial port speed.'],'if-port':['Numer portu KISS używany przez TNC.','KISS port number used by the TNC.'],'if-priority':['Niższa wartość oznacza wyższy priorytet.','Lower value means higher priority.'],'if-path':['Opcjonalna ścieżka digipeater dla beaconu.','Optional digipeater path for the beacon.']}[control.className||control.id];if(text)setHelpDot(label,text[lang==='pl'?0:1]);});}
function decorateSettingsSections(){const form=document.querySelector('.settings-form');if(!form)return;[...form.querySelectorAll('.form-section')].forEach((section,index)=>{section.dataset.settingsIndex=index;let help=section.querySelector(':scope > .section-help');if(!help){help=node('p','section-help');section.querySelector(':scope > h3')?.after(help);}const text=settingsSectionHelp[index];if(text)help.textContent=text[settingsLanguage()==='pl'?0:1];});const side=document.querySelector('#view-settings .settings-side')||document.querySelector('#view-settings .settings-grid>div');side?.classList.add('settings-side');decorateSettingsControls();}
 function ensureWebSettings(){if($('settings-web-bind'))return;const form=document.querySelector('.settings-form .panel-content');if(!form)return;const section=node('div','form-section');section.dataset.settingsKey='application';section.innerHTML='<h3>WWW</h3><p class="section-help">Adres panelu, dostęp operatora, obecność oraz ustawienia wspólne dla aplikacji desktopowej i serwera.</p><div class="form-grid"><label>Adres nasłuchu WWW<input id="settings-web-bind" placeholder="127.0.0.1:8080"></label><label>Plik tokenu operatora<input id="settings-web-token-file" placeholder="operator.token"></label><label><input id="settings-web-auth" type="checkbox"> Wymagaj logowania operatora</label><label>Obecność operatora<select id="settings-web-presence"><option value="auto">Auto</option><option value="always">Zawsze dostępny</option><option value="dnd">Nie przeszkadzać</option></select></label><label>Profil nieobecności<select id="settings-web-profile"><option value="1">1 — operator dostępny</option><option value="2">2 — informuj i komendy</option><option value="3">3 — poinformuj i rozłącz</option><option value="4">4 — stacja zamknięta</option></select></label><label>Limit heartbeat (s)<input id="settings-web-timeout" type="number" min="15" max="3600" step="5"></label></div><div class="check-grid"><label><input id="settings-statistics" type="checkbox"> Zbieranie statystyk stacji</label><label><input id="settings-presence-bar" type="checkbox"> Wyświetl pasek obecności operatora</label></div><p id="settings-web-warning" class="notice" hidden>Logowanie jest wyłączone, a panel nasłuchuje poza adresem lokalnym.</p>';const actions=form.querySelector(':scope > .actions');form.insertBefore(section,actions||null);}function collapseApplicationSettings(application){
  if(!application||application.dataset.settingsGroupsReady==='true')return;
  const serverGrid=$('settings-web-bind')?.closest('.form-grid');
  if(serverGrid?.parentElement===application){
    serverGrid.dataset.webGroup='server';
    const presenceGrid=node('div','form-grid');
    presenceGrid.dataset.webGroup='presence';
    ['settings-web-presence','settings-web-profile','settings-web-timeout','settings-presence-bar'].forEach(id=>{
      const label=$(id)?.closest('label');
      if(label)presenceGrid.append(label);
    });
    serverGrid.after(presenceGrid);
    $('settings-web-auth')?.closest('label')?.classList.add('web-login-option');
    $('settings-presence-bar')?.closest('label')?.classList.add('web-presence-bar-option');
    const timeoutLabel=$('settings-web-timeout')?.closest('label');
    const caption=timeoutLabel&&[...timeoutLabel.childNodes].find(child=>child.nodeType===3&&child.textContent.trim());
    if(caption)caption.textContent='Czas uznania operatora za nieobecnego (s)';
    if(timeoutLabel)setHelpDot(timeoutLabel,settingsHelp['settings-web-timeout'][settingsLanguage()==='pl'?0:1]);
    $('settings-web-presence')?.addEventListener('change',syncPresenceTimeout);
    syncPresenceTimeout();
  }
  // Never wrap a card that has already been grouped.  This function is called
  // again after language/runtime refreshes, so selecting all direct children
  // would make the same section nest inside itself on every pass.
  const directChildren=[...application.children].filter(item=>!item.classList.contains('settings-collapsible'));
  const group=(title,predicate,open=false)=>{
    const items=directChildren.filter(predicate).filter(item=>item.parentElement===application);
    if(!items.length)return;
    const card=node('div','interface-form settings-collapsible');
    const header=node('div','section-title');
    const strong=node('strong','',title);
    const toggle=node('button','text-button interface-toggle',interfaceToggleText(open));
    toggle.type='button';
    toggle.setAttribute('aria-expanded',String(open));
    const body=node('div','interface-body');
    const toggleCard=event=>{if(event.target.closest('button'))return;toggle.click();};
    toggle.onclick=()=>{
      const collapsed=card.classList.toggle('collapsed');
      toggle.textContent=interfaceToggleText(!collapsed);
      toggle.setAttribute('aria-expanded',String(!collapsed));
      if(!collapsed)scrollExpandedSettingsCard(card);
    };
    header.addEventListener('click',toggleCard);
    header.append(strong,toggle);
    application.insertBefore(card,items[0]);
    body.append(...items);
    card.append(header,body);
    if(!open)card.classList.add('collapsed');
  };
  group('Serwer WWW i logowanie',item=>item.dataset.webGroup==='server'||item.id==='settings-web-warning');
  group('Obecność operatora',item=>item.dataset.webGroup==='presence');
  group('Statystyki',item=>item.matches('.check-grid')&&item.querySelector('#settings-statistics'));
  group('Dostęp operatora',item=>item.matches('.access-panel'));
  group('Telegram',item=>item.matches('.telegram-settings'));
  group('Powiadomienia',item=>item.matches('.notification-settings'));
  application.dataset.settingsGroupsReady='true';
}function organizeSettingsSections(){const form=document.querySelector('.settings-form');if(!form||form.dataset.sectionsOrganized==='true')return;const sections=[...form.querySelectorAll(':scope > .panel-content > .form-section')];const find=id=>$(id)?.closest('.form-section');const identity=find('settings-callsign'),interfaces=find('interface-forms'),ax25=find('settings-t1'),services=find('settings-welcome'),web=find('settings-web-bind'),application=web;if(!identity||!interfaces||!ax25||!services)return;form.dataset.sectionsOrganized='true';const mark=(section,key,title)=>{section.dataset.settingsKey=key;const heading=section.querySelector(':scope > h3');if(heading)heading.textContent=title;};mark(identity,'station','Stacja');mark(interfaces,'interfaces','Interfejsy TNC');mark(ax25,'ax25','AX.25 i ruch');if(web)mark(web,'application','Aplikacja i WWW');const panelContent=form.querySelector(':scope > .panel-content');const actions=form.querySelector(':scope > .panel-content > .actions');const section=(key,title,kind='form-grid')=>{const value=node('div','form-section');value.dataset.settingsKey=key;value.innerHTML=`<h3>${title}</h3><p class="section-help"></p><div class="${kind}"></div>`;panelContent.insertBefore(value,actions||null);return value;};const messages=section('messages','Komunikaty i komendy','check-grid'),beacon=section('beacon','Beacon','check-grid'),mailbox=section('mailbox','MailBox','check-grid'),yapp=section('yapp','YAPP','check-grid');const telegram=services.querySelector('.telegram-settings'),notifications=services.querySelector('.notification-settings');const formGrid=key=>{const target=key.querySelector('.form-grid');if(target)return target;const value=node('div','form-grid');key.append(value);return value;};const move=(id,target)=>{const value=$(id)?.closest('label');if(value)target.append(value);};['settings-welcome','settings-goodbye','settings-remote'].forEach(id=>move(id,messages));['settings-beacon'].forEach(id=>move(id,beacon));['settings-mailbox'].forEach(id=>move(id,mailbox));['settings-yapp','settings-yapp-auto','settings-yapp-location'].forEach(id=>move(id,yapp));['settings-beacon-seconds','settings-beacon-to'].forEach(id=>move(id,formGrid(beacon)));move('settings-mailbox-call',formGrid(mailbox));move('settings-yapp-dir',formGrid(yapp));const max=node('label','','Limit plików YAPP');const maxInput=node('input');maxInput.id='settings-yapp-max-files';maxInput.type='number';maxInput.min='1';maxInput.max='100000';maxInput.value='1000';max.append(maxInput);formGrid(yapp).append(max);const cleanup=node('label');cleanup.innerHTML='<input id="settings-yapp-cleanup" type="checkbox"> Automatycznie usuwaj najstarsze pliki po osiągnięciu limitu';yapp.append(cleanup);const side=form.querySelector(':scope > .panel-content > .settings-side');const templates=side?.querySelector('#template-select')?.closest('.panel');const access=side?.querySelector('#logout')?.closest('.panel');if(templates)messages.append(templates);if(access&&application)application.append(access);if(telegram&&application)application.append(telegram);if(notifications&&application)application.append(notifications);if(side&&!side.querySelector('.panel'))side.remove();services.remove();collapseApplicationSettings(application);const oldGrids=services.querySelectorAll('.form-grid,.check-grid');oldGrids.forEach(grid=>{if(!grid.children.length)grid.remove();});}
function applySettingsFullWidth(){const form=document.querySelector('.settings-form');const grid=document.querySelector('#view-settings .settings-grid');if(!form||!grid)return;grid.style.display='block';form.style.width='100%';form.querySelector(':scope > .panel-content > .settings-side')?.setAttribute('hidden','');}
function collapseTemplateSettings(application){
  const panel=application?.querySelector(':scope > .panel');
  if(!panel||panel.dataset.settingsCollapseReady==='true')return;
  const content=panel.querySelector(':scope > .panel-content');
  if(!content)return;
  const title=panel.querySelector(':scope > .panel-head h2')?.textContent.trim()||'Szablony wiadomości';
  const card=node('div','interface-form settings-collapsible');
  const header=node('div','section-title');
  const strong=node('strong','',title);
  const toggle=node('button','text-button interface-toggle',interfaceToggleText(false));
  toggle.type='button';
  toggle.setAttribute('aria-expanded','false');
  const body=node('div','interface-body');
  const help=document.createElement('details');
  help.className='mqtt-help';
  help.innerHTML='<summary>Jak skonfigurować MQTT i Home Assistant?</summary><ol><li>MQTT nie wymaga bota. Potrzebny jest broker MQTT, np. Mosquitto albo broker z dodatku Home Assistant.</li><li>W brokerze utwórz użytkownika i hasło dla stacji. Nie używaj konta administratora Home Assistant.</li><li>W polu <b>Broker MQTT</b> wpisz adres osiągalny z Alpine, np. <code>mqtt://192.168.1.20:1883</code>. Dla szyfrowania użyj <code>mqtts://</code> albo zaznacz TLS.</li><li>Wpisz użytkownika, zapisz hasło przyciskiem <b>Zapisz hasło MQTT</b> i włącz MQTT.</li><li>Zaznacz <b>Home Assistant Discovery</b>, jeśli broker jest używany przez Home Assistant. Po restarcie encje stacji pojawią się automatycznie.</li><li>Zapisz ustawienia i zrestartuj aplikację. Publikowane są tylko status aplikacji, stan TNC i lista MHeard.</li></ol><p class="hint">MQTT jest wyłączone domyślnie. Aplikacja nie odbiera poleceń MQTT i nie publikuje treści ramek radiowych.</p>';
  body.append(help);
  const toggleCard=event=>{if(event.target.closest('button'))return;toggle.click();};
  toggle.onclick=()=>{
    const collapsed=card.classList.toggle('collapsed');
    toggle.textContent=interfaceToggleText(!collapsed);
    toggle.setAttribute('aria-expanded',String(!collapsed));
    if(!collapsed)scrollExpandedSettingsCard(card);
  };
  header.addEventListener('click',toggleCard);
  header.append(strong,toggle);
  body.append(...content.childNodes);
  card.append(header,body);
  panel.replaceWith(card);
  card.classList.add('collapsed');
  card.dataset.settingsCollapseReady='true';
}
function settingsTabItems(){const form=document.querySelector('.settings-form');if(!form)return[];const labels={station:['settings.station','Stacja'],interfaces:['settings.interfaces','Interfejsy TNC'],beacon:['settings.beacon','Beacon'],mailbox:['settings.mailbox','MailBox'],messages:['settings.messages','Komunikaty i komendy'],yapp:['settings.yapp','YAPP'],ax25:['settings.ax25','AX.25 i ruch']};const order=['station','interfaces','beacon','mailbox','messages','yapp','ax25'];const sections=[...form.querySelectorAll('.form-section[data-settings-key]')];const byKey=key=>sections.find(element=>element.dataset.settingsKey===key);const items=order.map(type=>{const element=byKey(type);if(!element)return null;const [key,fallback]=labels[type];return {element,key,fallback,section:true};}).filter(Boolean);const web=byKey('web'),application=byKey('application');if(web||application)items.push({element:[web,application].filter(Boolean),key:'settings.application_web',fallback:'Aplikacja i WWW',section:true});return items;}
function ensureSettingsTabs(force=false){const form=document.querySelector('.settings-form');if(!form)return;const items=settingsTabItems();let tabs=$('settings-tabs');if(tabs&&!force&&tabs.children.length===items.length)return;if(tabs)tabs.remove();tabs=node('div','settings-tabs');tabs.id='settings-tabs';items.forEach((item,index)=>{const b=node('button','settings-tab');b.type='button';b.dataset.index=index;b.append(svgIcon(settingsTabIcons[item.key.replace(/^settings\./,'').replace('application_web','application')]||'cog-outline'),node('span','settings-tab-label',state.catalog[item.key]||item.fallback));b.onclick=()=>selectSettingsTab(index);tabs.append(b);});const anchor=Array.isArray(items[0]?.element)?items[0].element[0]:items[0]?.element;anchor?.before(tabs);selectSettingsTab(Math.min(Number(tabs.dataset.selected||0),items.length-1));}
function selectSettingsTab(index){const form=document.querySelector('.settings-form');if(!form)return;const items=settingsTabItems();if(!items.length)return;closeHelpPopover();const selected=Math.max(0,Math.min(index,items.length-1));const active=Array.isArray(items[selected].element)?items[selected].element: [items[selected].element];const tabs=$('settings-tabs');if(tabs)tabs.dataset.selected=selected;form.querySelectorAll('.form-section[data-settings-key]').forEach(section=>{section.hidden=!active.includes(section);});const isSection=Boolean(items[selected].section);form.querySelector('.actions')?.toggleAttribute('hidden',!isSection);form.querySelector('.advanced-config')?.toggleAttribute('hidden',!isSection);document.querySelectorAll('.settings-tab').forEach((button,i)=>button.classList.toggle('active',i===selected));appendSettingsHelp();}
function refreshSettingsTabs(){settingsTabItems().forEach((item,index)=>{const button=document.querySelector(`.settings-tab[data-index="${index}"]`);const label=button?.querySelector('.settings-tab-label');if(label)label.textContent=state.catalog[item.key]||item.fallback;});}
function fixSettingsHeadings(){const keys={station:'settings.station',web:'settings.web',application:'settings.application',interfaces:'settings.interfaces',ax25:'settings.ax25',messages:'settings.messages',beacon:'settings.beacon',mailbox:'settings.mailbox',yapp:'settings.yapp'};ensureSettingsTabs();document.querySelectorAll('.settings-form .form-section[data-settings-key] h3').forEach(heading=>{const key=keys[heading.closest('.form-section').dataset.settingsKey];if(key&&state.catalog[key])heading.textContent=state.catalog[key];});refreshSettingsTabs();decorateSettingsSections();collapseApplicationSettings(document.querySelector('.settings-form .form-section[data-settings-key="application"]'));document.querySelectorAll('#interface-forms > .interface-form').forEach(syncInterfaceTransport);refreshInterfaceToggles();bindAllCardHelp();}
ensureWebSettings();

function ensureLocatorField(){if($('settings-locator'))return;const grid=$('settings-callsign')?.closest('.form-grid');if(!grid)return;const label=node('label','', 'Lokator QTH (opcjonalny)');const input=node('input');input.id='settings-locator';input.maxLength=6;input.placeholder='JO92AA';label.replaceChildren(document.createTextNode('Lokator QTH (opcjonalny)'),input);grid.insertBefore(label,$('settings-language')?.closest('label')||null);}
ensureLocatorField();
setInterval(()=>{if(!activeHelpPopover)fixSettingsHeadings();},1000);
function enhanceSettingsLayout(){if($('settings-style'))return;const style=document.createElement('style');style.id='settings-style';style.textContent='.settings-form .panel-content{padding:24px 26px 28px}.settings-form .form-section{padding:4px 0 26px;margin-bottom:24px}.settings-form .form-section:last-of-type{padding-bottom:4px;margin-bottom:0;border-bottom:0}.settings-form .form-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px 18px}.settings-form .form-grid>label{display:flex;flex-direction:column;align-items:stretch;position:relative}.settings-form .form-grid>label input,.settings-form .form-grid>label select{width:100%;margin-top:6px;min-height:40px}.settings-form .section-help{font-size:11px;line-height:1.65;color:#899580;margin:-4px 0 18px;max-width:680px}.settings-form label[data-help]{position:relative;padding-right:22px}.settings-form label[data-help]::after{content:"i";position:absolute;right:0;top:0;width:16px;height:16px;border:1px solid #cbd8bf;border-radius:50%;color:#76905f;background:#f6f9f1;text-align:center;font:11px/15px Georgia,serif}.settings-form label[data-help]::before{content:attr(data-help);position:absolute;z-index:20;right:0;top:24px;width:240px;padding:8px 10px;border:1px solid #dce6d4;border-radius:6px;background:#263b29;color:#f4f7ed;box-shadow:0 8px 20px #1d302430;font-size:10px;line-height:1.45;opacity:0;pointer-events:none;transform:translateY(-3px);transition:opacity .12s,transform .12s}.settings-form label[data-help]:hover::before,.settings-form label[data-help]:focus-within::before{opacity:1;transform:translateY(0)}.settings-form .check-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px 16px;margin-top:18px;padding:14px 16px;border:1px solid #edf1e8;border-radius:8px;background:#fafcf8}.settings-form .check-grid label{padding:8px 4px;margin:0;line-height:1.35}.settings-form .check-grid label[data-help]::before{right:auto;left:0;top:34px}.settings-form .interface-form{padding:18px 18px 16px;margin-bottom:14px;border-radius:9px;background:linear-gradient(145deg,#fbfdf9,#f6faf2);box-shadow:0 2px 8px #304a2610}.settings-form .interface-form .form-grid>label{display:flex;flex-direction:column}.settings-form .interface-form>label{margin-top:16px}.settings-form .interface-form>label input{width:100%;min-height:40px;margin-top:6px}.settings-form .section-title{margin-bottom:14px}.settings-form .section-title strong{font-size:12px}.settings-tabs{display:flex;gap:8px;flex-wrap:wrap;margin:-4px 0 24px;padding:0 0 14px;border-bottom:1px solid #edf0e9}.settings-tab{border:1px solid #dbe4d2;border-radius:7px;background:#f8faf5;color:#627455;padding:9px 13px;font-size:11px;transition:all .15s}.settings-tab:hover{border-color:#b8cca6;background:#f0f6e9}.settings-tab.active{background:#d6e9b7;border-color:#c7dda4;color:#2d4828;font-weight:650;box-shadow:0 2px 5px #50743a18}.settings-form .form-section[hidden]{display:none!important}.settings-side{display:flex;flex-direction:column;gap:18px}.settings-side .panel{box-shadow:0 2px 10px #304a2608}.settings-side .panel-content{padding:22px}.settings-side .panel-content>p:first-of-type{line-height:1.65}.settings-side .settings-card-help{margin:0;padding:12px 22px 0;font-size:11px;line-height:1.55;color:#899580}.settings-side .hint{padding:10px 12px;border-radius:6px;background:#f7faf3;border:1px solid #e7eee0}.settings-side #template-text{min-height:220px}.settings-side .access-panel{margin-top:0}.settings-side .access-panel .panel-content{background:linear-gradient(145deg,#fbfdf9,#f5f9ef)}#monitor-dialog{width:min(900px,calc(100% - 30px));padding:22px}#monitor-dialog .monitor-body{height:55vh}';document.head.append(style);const section=$('settings-web-bind')?.closest('.form-section');const anchor=document.querySelector('.settings-form .form-section');if(section&&anchor&&section!==anchor)anchor.parentElement.insertBefore(section,anchor.nextElementSibling);}
function organizeSettingsPanels(){const formContent=document.querySelector('.settings-form .panel-content');const side=document.querySelector('#view-settings .settings-grid>div');if(!formContent||!side||side.dataset.organized==='true')return;side.dataset.organized='true';side.classList.add('settings-side');formContent.append(side);}
enhanceSettingsLayout();
document.getElementById('settings-style')?.remove();
organizeSettingsPanels();
organizeSettingsSections();

applySettingsFullWidth();
// The final tab set is created after the application/WWW split below.
// Creating the legacy combined tab here caused a transient duplicate layout.
bindAllCardHelp();
// Web settings are loaded together with the complete settings form. Keeping a
// second asynchronous loader here could overwrite a user's checkbox change
// while the form was still being opened.
async function saveWebConfiguration(){const result=await request('web/save',{body:{bind:$('settings-web-bind')?.value.trim()||'127.0.0.1:8080',token_file:$('settings-web-token-file')?.value.trim()||'',auth_enabled:String(Boolean($('settings-web-auth')?.checked)),presence_mode:$('settings-web-presence')?.value||'auto',presence_bar_enabled:String(Boolean($('settings-presence-bar')?.checked)),absence_profile:$('settings-web-profile')?.value||'2',heartbeat_timeout:$('settings-web-timeout')?.value||'90',statistics_enabled:String(Boolean($('settings-statistics')?.checked))}});state.webDetails={...(state.webDetails||{}),presenceBarEnabled:Boolean($('settings-presence-bar')?.checked),presenceMode:$('settings-web-presence')?.value||state.webDetails?.presenceMode||'auto'};delete $('settings-presence-bar')?.dataset.changed;renderPresenceBar(state.webDetails);return result;}async function poll(){try{const [snapshotResult,webDetailsResult]=await Promise.allSettled([request('state',{method:'GET'}),request('web/details',{method:'GET'})]);if(snapshotResult.status!=='fulfilled')throw snapshotResult.reason;const snapshot=snapshotResult.value;state.snapshot=snapshot;$('server-light').classList.add('online');$('server-state').textContent='Panel połączony';if(webDetailsResult.status==='fulfilled'){state.webDetails=webDetailsResult.value;renderPresenceBar(webDetailsResult.value);}try{if(!activeHelpPopover&&state.view!=='settings')render(snapshot);}catch{} }catch(exception){$('server-light').classList.remove('online');$('server-state').textContent='Łączenie z panelem…';}finally{setTimeout(poll,1500);}}
async function saveWebConfiguration(){const result=await request('web/save',{body:{bind:$('settings-web-bind')?.value.trim()||'127.0.0.1:8080',token_file:$('settings-web-token-file')?.value.trim()||'',auth_enabled:String(Boolean($('settings-web-auth')?.checked)),presence_mode:$('settings-web-presence')?.value||'auto',presence_bar_enabled:String(Boolean($('settings-presence-bar')?.checked)),absence_profile:$('settings-web-profile')?.value||'2',heartbeat_timeout:$('settings-web-timeout')?.value||'90',statistics_enabled:String(Boolean($('settings-statistics')?.checked)),mqtt_enabled:String(Boolean($('settings-mqtt-enabled')?.checked)),mqtt_broker:$('settings-mqtt-broker')?.value.trim()||'',mqtt_username:$('settings-mqtt-username')?.value.trim()||'',mqtt_tls:String(Boolean($('settings-mqtt-tls')?.checked)),mqtt_client_id:$('settings-mqtt-client-id')?.value.trim()||'ultimatepr-station',mqtt_topic_prefix:$('settings-mqtt-topic-prefix')?.value.trim()||'ultimatepr',mqtt_discovery_topic:$('settings-mqtt-discovery-topic')?.value.trim()||'homeassistant'}});state.webDetails={...(state.webDetails||{}),presenceBarEnabled:Boolean($('settings-presence-bar')?.checked),presenceMode:$('settings-web-presence')?.value||state.webDetails?.presenceMode||'auto'};delete $('settings-presence-bar')?.dataset.changed;renderPresenceBar(state.webDetails);return result;}
function syncMqttSettings(){const d=state.webDetails,card=$('settings-mqtt-card');if(!$('settings-mqtt-enabled')||!d||card?.dataset.mqttLoaded==='true')return;checked('settings-mqtt-enabled',d.mqttEnabled);field('settings-mqtt-broker',d.mqttBroker||'');field('settings-mqtt-username',d.mqttUsername||'');checked('settings-mqtt-tls',d.mqttTls);field('settings-mqtt-client-id',d.mqttClientId||'ultimatepr-station');field('settings-mqtt-topic-prefix',d.mqttTopicPrefix||'ultimatepr');field('settings-mqtt-discovery-topic',d.mqttDiscoveryTopic||'homeassistant');const status=$('settings-mqtt-password-status');if(status)status.textContent=`Hasło: ${d.mqttPasswordPresent?'zapisane w osobnym pliku':'nieustawione'}`;if(card)card.dataset.mqttLoaded='true';}
setInterval(syncMqttSettings,500);
poll();
syncBuildVersion();
let heartbeatTimer;
const sendHeartbeat=()=>{if(document.visibilityState==='visible')request('presence/heartbeat',{body:{}}).catch(()=>{});};
document.addEventListener('visibilitychange',sendHeartbeat);
heartbeatTimer=setInterval(sendHeartbeat,30000);
request('config/details').then(details=>loadLanguage(details.language)).catch(()=>loadLanguage('en'));
function ensureDiagnostics(){if($('view-diagnostics'))return;const nav=node('button','nav');nav.dataset.view='diagnostics';nav.innerHTML='<svg class="icon" aria-hidden="true"><use href="/icons.svg#pulse"></use></svg> Diagnostyka';nav.addEventListener('click',()=>showView('diagnostics'));document.querySelector('nav').append(nav);const view=node('section','view');view.id='view-diagnostics';view.hidden=true;const panel=node('section','panel');panel.innerHTML='<div class="panel-head"><h2>Diagnostyka stacji</h2><button id="diagnostics-refresh" class="button secondary small" type="button">Odśwież</button></div><div class="panel-content"><div id="diagnostics-body" class="monitor-body">Ładowanie…</div></div>';view.append(panel);document.querySelector('main').append(view);$('diagnostics-refresh').onclick=()=>guard(loadDiagnostics);}
async function loadDiagnostics(){ensureDiagnostics();const [s,w,b]=await Promise.all([request('state',{method:'GET'}),request('web/details'),request('build-info',{method:'GET'})]);const values=[`Wersja aplikacji: ${b.version}`,`Tryb demo: ${s.demo?'tak':'nie'}`,`Znak stacji: ${s.callsign||'nie ustawiono'}`,`Czas pracy: ${s.uptime}s`,`RX / TX: ${s.rx} / ${s.tx}`,`Sesje aktywne: ${s.sessions.filter(x=>x.state!=='Idle').length} / ${s.sessions.length}`,`Interfejsy: ${s.interfaces.filter(x=>x.online).length} online / ${s.interfaces.length}`,`Plik WWW: ${w.path}`,`Nasłuch WWW: ${w.bind}`,`Plik tokenu: ${w.tokenFile||'brak (tylko lokalny dostęp)'}`];rows($('diagnostics-body'),values.map(x=>node('div','',x)));}
ensureDiagnostics();
document.querySelector('#config-import')?.addEventListener('click',()=>{$('config-file')?.click();});
document.querySelector('#config-file')?.addEventListener('change',async event=>{const file=event.target.files?.[0];if(!file)return;try{await ensureStationSettingsLoaded();importConfigForm(await file.text());$('config-text').value=settingsConfigText();toast('Zaimportowano konfigurację do formularza. Sprawdź ją i kliknij „Zapisz ustawienia”.');}catch(exception){error(exception);}finally{event.target.value='';}});
function ensureStatsView(){if(!state.statsEnabled||$('view-stats'))return;const metrics=document.querySelector('.metrics');if(!metrics)return;const nav=node('button','nav');nav.dataset.view='stats';nav.innerHTML='<svg class="icon" aria-hidden="true"><use href="/icons.svg#chart-line"></use></svg> Statystyki';nav.addEventListener('click',()=>showView('stats'));const view=node('section','view');view.id='view-stats';view.hidden=true;const panel=node('section','panel stats-view-panel');panel.innerHTML='<div class="panel-head"><div><h2>Statystyki stacji</h2><span class="tag">LIVE</span></div><span class="hint-inline">Historia ostatnich 30 dni</span></div><div class="stats-content"><section id="stats-days" class="stats-block"></section><section id="stats-stations" class="stats-block"></section><section id="stats-interfaces" class="stats-block"></section></div>';metrics.hidden=false;panel.insertBefore(metrics,panel.querySelector('.stats-content'));view.append(panel);document.querySelector('main').append(view);document.querySelector('nav').insertBefore(nav,document.querySelector('[data-view="settings"]')||document.querySelector('[data-view="monitor"]')||document.querySelector('[data-view="heard"]'));}
function renderStats(data){if(!data?.enabled)return;ensureStatsView();const formatStatsDay=value=>{const day=Number(value);return Number.isFinite(day)?new Date(day*86400000).toISOString().slice(0,10):value;};const dayRows=(data.days||[]).slice().reverse().map(d=>`<tr><td>${formatStatsDay(d.date)}</td><td>${d.rx}</td><td>${d.tx}</td><td>${d.stations}</td><td>${d.sessions}</td></tr>`).join('');const stationRows=(data.stations||[]).sort((a,b)=>b.frames-a.frames).slice(0,20).map(d=>`<tr><td>${d.call}</td><td>${d.frames}</td></tr>`).join('');const interfaceRows=(data.interfaces||[]).map(d=>`<tr><td>${d.name}</td><td>${d.rx}</td><td>${d.tx}</td></tr>`).join('');if($('stats-days'))$('stats-days').innerHTML=`<div class="stats-block-head"><h3>Ostatnie dni</h3><span class="tag">UTC</span></div><div class="table-wrap"><table><thead><tr><th>Dzień UTC</th><th>RX</th><th>TX</th><th>Stacje</th><th>Sesje</th></tr></thead><tbody>${dayRows||'<tr><td colspan="5" class="quiet-empty">Brak danych</td></tr>'}</tbody></table></div>`;if($('stats-stations'))$('stats-stations').innerHTML=`<div class="stats-block-head"><h3>Najczęściej słyszane stacje</h3><span class="tag">30 DNI</span></div><div class="table-wrap"><table><thead><tr><th>Stacja</th><th>Ramki</th></tr></thead><tbody>${stationRows||'<tr><td colspan="2" class="quiet-empty">Brak danych</td></tr>'}</tbody></table></div>`;if($('stats-interfaces'))$('stats-interfaces').innerHTML=`<div class="stats-block-head"><h3>Interfejsy TNC</h3><span class="tag">RX / TX</span></div><div class="table-wrap"><table><thead><tr><th>Interfejs</th><th>RX</th><th>TX</th></tr></thead><tbody>${interfaceRows||'<tr><td colspan="3" class="quiet-empty">Brak danych</td></tr>'}</tbody></table></div>`;}
async function syncStatsView(){try{const data=await request('stats',{method:'GET'});state.statsEnabled=bool(data.enabled);const view=$('view-stats'),nav=document.querySelector('[data-view="stats"]');if(!state.statsEnabled){if(state.view==='stats'){state.view='terminal';showView('terminal');}const metrics=view?.querySelector('.metrics')||document.querySelector('.metrics'),terminal=document.querySelector('#view-terminal');if(metrics&&terminal){terminal.parentElement.insertBefore(metrics,terminal);metrics.hidden=true;}view?.remove();nav?.remove();return;}ensureStatsView();renderStats(data);}catch{} }
syncStatsView();
setInterval(syncStatsView,5000);
function ensureMonitorView(){if($('view-monitor'))return;const nav=node('button','nav');nav.dataset.view='monitor';nav.innerHTML='<svg class="icon" aria-hidden="true"><use href="/icons.svg#lan-connect"></use></svg> Monitor';nav.addEventListener('click',()=>showView('monitor'));const view=node('section','view');view.id='view-monitor';view.hidden=true;const panel=node('section','panel','');panel.innerHTML='<div class="panel-head monitor-view-head"><div><h2>Monitor ramek</h2><span class="monitor-view-live live-label">● LIVE</span></div><div class="monitor-actions"><button id="monitor-view-freeze" class="text-button" type="button">Wstrzymaj widok</button><button id="monitor-view-export" class="text-button" type="button">Eksport ↓</button></div></div><div id="monitor-view-body" class="monitor-body monitor-view-body"></div><div class="panel-foot"><span>UTC · ostatnie 500 wierszy</span><span>Odbiór działa również przy zamkniętej przeglądarce</span></div>';view.append(panel);document.querySelector('main').append(view);const heardNav=document.querySelector('[data-view="heard"]');document.querySelector('nav').insertBefore(nav,heardNav?.nextSibling||null);$('monitor-view-freeze').onclick=()=>$('freeze').click();$('monitor-view-export').onclick=()=>$('export').click();}
ensureMonitorView();
const monitorViewPanel=document.querySelector('#view-monitor .panel');if(monitorViewPanel){const broadcastForm=node('form','composer monitor-broadcast');broadcastForm.id='monitor-broadcast-form';broadcastForm.innerHTML='<label class="sr-only" for="monitor-broadcast-interface">Interfejs TNC</label><select id="monitor-broadcast-interface" required aria-label="Interfejs TNC"></select><label class="sr-only" for="monitor-broadcast-destination">Adres docelowy</label><input id="monitor-broadcast-destination" value="CQ" maxlength="9" required placeholder="Cel, np. CQ"><label class="sr-only" for="monitor-broadcast-via">Ścieżka digi</label><input id="monitor-broadcast-via" maxlength="80" placeholder="Ścieżka digi, opcjonalnie"><label class="sr-only" for="monitor-broadcast-text">Treść broadcastu</label><input id="monitor-broadcast-text" maxlength="256" required placeholder="Wpisz wiadomość broadcast…"><button class="button primary" type="submit">Wyślij <svg class="icon" aria-hidden="true"><use href="/icons.svg#send"></use></svg></button>';monitorViewPanel.insertBefore(broadcastForm,monitorViewPanel.querySelector('#monitor-view-body'));broadcastForm.onsubmit=e=>{e.preventDefault();guard(async()=>{await request('broadcast',{body:{interface:$('monitor-broadcast-interface').value,destination:$('monitor-broadcast-destination').value.trim(),via:$('monitor-broadcast-via').value.trim(),text:$('monitor-broadcast-text').value}});$('monitor-broadcast-text').value='';toast('Broadcast wysłany.');});};}
function ensureMonitorDialog(){if($('monitor-dialog'))return;const panel=document.querySelector('.monitor');const head=panel?.querySelector('.panel-head>div:last-child');if(!panel||!head)return;const open=node('button','text-button','Otwórz monitor');open.type='button';open.onclick=()=>{$('monitor-dialog').showModal();if(state.snapshot)renderMonitorDialog(state.snapshot);};head.prepend(open);const dialog=node('dialog');dialog.id='monitor-dialog';dialog.innerHTML='<div class="dialog-head"><h2>Monitor ramek</h2><button class="close" type="button" aria-label="Zamknij"><svg class="icon" aria-hidden="true"><use href="/icons.svg#close-circle-outline"></use></svg></button></div><div id="monitor-dialog-body" class="monitor-body"></div>';dialog.querySelector('.close').onclick=()=>dialog.close();document.body.append(dialog);}
ensureMonitorDialog();

const yappTextExtensions=new Set(['txt','csv','json','log','md','yapp','xml','ini','conf']);
function yappExtension(name){return String(name).split('.').pop().toLowerCase();}
function yappFileKind(name){const ext=yappExtension(name);if(yappTextExtensions.has(ext))return 'TXT';if(['png','jpg','jpeg','gif','webp','bmp'].includes(ext))return 'IMG';return 'FILE';}
async function previewYappFile(name){
  let dialog=$('yapp-preview-dialog');
  if(!dialog){dialog=node('dialog');dialog.id='yapp-preview-dialog';dialog.innerHTML='<div class="dialog-head"><h2 id="yapp-preview-title"></h2><button class="close" type="button" aria-label="Zamknij"><svg class="icon" aria-hidden="true"><use href="/icons.svg#close-circle-outline"></use></svg></button></div><p id="yapp-preview-note" class="hint"></p><pre id="yapp-preview-body" class="message-body"></pre><div id="yapp-preview-image" hidden></div>';dialog.querySelector('.close').onclick=()=>dialog.close();document.body.append(dialog);}
  $('yapp-preview-title').textContent=`Podgląd: ${name}`;$('yapp-preview-body').hidden=false;$('yapp-preview-image').hidden=true;$('yapp-preview-body').textContent='Ładowanie podglądu…';dialog.showModal();
  const response=await fetch('/api/file?'+new URLSearchParams({name}),{headers:headers()});if(!response.ok)throw Error('Nie udało się odczytać pliku');const blob=await response.blob();const kind=yappFileKind(name);
  if(kind==='TXT'&&blob.size<=512*1024){$('yapp-preview-note').textContent='Podgląd tekstu';$('yapp-preview-body').textContent=await blob.text();return;}
  $('yapp-preview-note').textContent=kind==='IMG'?'Ten typ pliku można pobrać, ale podgląd obrazu nie jest jeszcze dostępny.':'Podgląd jest dostępny dla plików tekstowych do 512 KiB.';$('yapp-preview-body').textContent=`Plik: ${name}\nRozmiar: ${Number(blob.size).toLocaleString()} B`;
}
async function loadFiles(){
  const result=await request('files');
  const files=Array.isArray(result)?result:(result.files||[]);
  const meta=Array.isArray(result)?null:result;
  const active=state.snapshot?.sessions?.some(session=>session.transfer);
  const list=$('files-list');
  let toolbar=$('files-toolbar');
  if(!toolbar){toolbar=node('div','files-toolbar');toolbar.id='files-toolbar';const search=node('input');search.type='search';search.placeholder='Szukaj pliku';search.id='files-search';const sort=node('select');sort.id='files-sort';sort.innerHTML='<option value="name">Nazwa</option><option value="newest">Najnowsze</option><option value="largest">Największe</option>';const views=node('div','files-view-toggle');const listView=button('Lista',()=>{state.filesView='list';localStorage.setItem('yapp-view','list');loadFiles();});listView.id='files-view-list';const gridView=button('Kafelki',()=>{state.filesView='grid';localStorage.setItem('yapp-view','grid');loadFiles();});gridView.id='files-view-grid';views.append(listView,gridView);toolbar.append(search,sort,views);list.parentElement.insertBefore(toolbar,list);}
  state.filesView=state.filesView||localStorage.getItem('yapp-view')||'list';$('files-view-list').classList.toggle('active',state.filesView==='list');$('files-view-grid').classList.toggle('active',state.filesView==='grid');list.className=`files-list ${state.filesView==='grid'?'files-list-grid':'files-list-list'}`;
  const query=($('files-search')?.value||'').trim().toLowerCase();const sortBy=$('files-sort')?.value||'name';const filtered=files.filter(f=>String(f.name).toLowerCase().includes(query)).slice().sort((a,b)=>sortBy==='newest'?Number(b.modified||0)-Number(a.modified||0):sortBy==='largest'?Number(b.size||0)-Number(a.size||0):String(a.name).localeCompare(String(b.name)));
  let summary=$('files-summary');if(!summary){summary=node('div','files-summary');summary.id='files-summary';list.parentElement.insertBefore(summary,list);}summary.textContent=meta?`Pliki: ${meta.count} / ${meta.maxFiles} · zajęte miejsce: ${Number(meta.usedBytes||0).toLocaleString()} B`:'';
  let listHeader=$('files-list-header');if(!listHeader){listHeader=node('div','files-list-header');listHeader.id='files-list-header';listHeader.innerHTML='<span>Typ</span><span>Nazwa</span><span>Rozmiar</span><span>Odebrano</span><span>Akcje</span>';list.parentElement.insertBefore(listHeader,list);}listHeader.hidden=state.filesView!=='list';
  const items=filtered.map(f=>{const row=node('div','file-row');const date=f.modified?new Date(Number(f.modified)*1000).toLocaleString():'';const kind=node('span','file-type-icon',yappFileKind(f.name));const info=node('div','file-info');info.append(node('strong','',f.name));const size=node('span','file-size',Number(f.size||0).toLocaleString()+' B');const received=node('span','file-date',date);const actions=node('div','file-actions');actions.append(button('Podgląd',()=>previewYappFile(f.name)),button('Pobierz ↓',async()=>{const res=await fetch('/api/file?'+new URLSearchParams({name:f.name}),{headers:headers()});if(!res.ok)throw Error('Nie udało się pobrać pliku');download(await res.blob(),f.name);}));const del=button('Usuń',async()=>{if(active)throw Error('Nie można usuwać pliku podczas aktywnego transferu');await request('files/delete',{body:{name:f.name}});await loadFiles();});del.disabled=active;actions.append(del);row.append(kind,info,size,received,actions);return row;});
  rows(list,items.length?items:[node('div','quiet-empty','Nie odebrano jeszcze żadnych plików')]);
  $('files-search').oninput=()=>loadFiles();$('files-sort').onchange=()=>loadFiles();
}

// The settings model is kept in one place so every control is serialized identically.
function telegramFeedback(message,failed=false){const el=$('telegram-feedback');if(!el)return;el.textContent=message;el.hidden=!message;el.classList.toggle('error',failed);}
function updateTelegramStatus(){const el=$('telegram-status');if(!el)return;const enabled=$('settings-telegram-enabled')?.checked;const chat=$('settings-telegram-chat-id')?.value.trim();el.textContent=!enabled?'Wyłączony':!state.telegramTokenPresent?'Brak tokenu':!chat?'Oczekiwanie na czat':'Skonfigurowany';}
async function saveTelegramToken(){const token=$('settings-telegram-token')?.value.trim()||'';await request('telegram/token',{body:{token}});telegramFeedback('Token Telegrama zapisany.');state.telegramTokenPresent=Boolean(token);$('settings-telegram-token').value='';updateTelegramStatus();toast(token?'Token Telegrama zapisany.':'Token Telegrama usunięty.');}
async function discoverTelegramChats(){const chats=await request('telegram/discover');const list=$('telegram-chat-list');list.replaceChildren(new Option('Wybierz wykryty czat',''));(chats||[]).forEach(chat=>list.append(new Option(chat.name+' ('+chat.kind+') · '+chat.id,chat.id)));list.hidden=!(chats||[]).length;if(!chats?.length)throw Error('Nie znaleziono czatów. Wyślij /start do bota i spróbuj ponownie.');telegramFeedback('Wybierz wykryty czat z listy.');toast('Wybierz wykryty czat z listy.');}
$('settings-telegram-enabled')?.addEventListener('change',updateTelegramStatus);$('settings-telegram-chat-id')?.addEventListener('input',updateTelegramStatus);$('telegram-save-token')?.addEventListener('click',()=>guard(saveTelegramToken,e=>telegramFeedback(e.message||String(e),true)));$('telegram-discover')?.addEventListener('click',()=>guard(discoverTelegramChats,e=>telegramFeedback(e.message||String(e),true)));$('telegram-chat-list')?.addEventListener('change',e=>{if(e.target.value){$('settings-telegram-chat-id').value=e.target.value;updateTelegramStatus();}});$('telegram-test')?.addEventListener('click',()=>guard(async()=>{await request('telegram/test',{body:{chat_id:$('settings-telegram-chat-id').value.trim()}});telegramFeedback('Wiadomość testowa Telegrama została wysłana.');toast('Wiadomość testowa Telegrama została wysłana.');}),e=>telegramFeedback(e.message||String(e),true));
function settingsConfigText(){const v=id=>$(id)?.value.trim()||'',b=id=>Boolean($(id)?.checked);const notificationLines=["incoming_connection","outgoing_connection","terminal_message","disconnection","mailbox","yapp","tnc","critical"].flatMap(k=>[`notification_${k}_in_app=${notificationSetting(k,'in-app',b(`notification-${k}-in-app`))}`,`notification_${k}_native=${notificationSetting(k,'native',b(`notification-${k}-native`))}`,`notification_${k}_telegram=${notificationSetting(k,'telegram',b(`notification-${k}-telegram`))}`,`notification_${k}_sound=${notificationSetting(k,'sound',v(`notification-${k}-sound`)||0)}`]);const lines=[...notificationLines,`telegram_enabled=${b('settings-telegram-enabled')}`,`telegram_chat_id=${v('settings-telegram-chat-id')}`,`telegram_token_file=telegram.bot-token`,`callsign=${v('settings-callsign')}`,`myname=${v('settings-myname')}`,`locator=${v('settings-locator')}`,`language=${v('settings-language')}`,`welcome_enabled=${b('settings-welcome')}`,`goodbye_enabled=${b('settings-goodbye')}`,`remote_commands=${b('settings-remote')}`,`beacon_enabled=${b('settings-beacon')}`,`beacon_seconds=${Math.round(Number(v('settings-beacon-seconds'))*60)}`,`beacon_to=${v('settings-beacon-to')}`,`digipeat_enabled=${b('settings-digipeat')}`,`t1_ms=${v('settings-t1')}`,`t3_ms=${v('settings-t3')}`,`retries=${v('settings-retries')}`,`paclen=${v('settings-paclen')}`,`accept_incoming=${b('settings-accept')}`,`ax25_autonegotiation=${b('settings-autoneg')}`,`mailbox_enabled=${b('settings-mailbox')}`,`mailbox_callsign=${v('settings-mailbox-call')}`,`yapp_enabled=${b('settings-yapp')}`,`yapp_auto_receive=${b('settings-yapp-auto')}`,`yapp_ask_location=${b('settings-yapp-location')}`,`yapp_download_dir=${v('settings-yapp-dir')}`,`yapp_max_files=${v('settings-yapp-max-files')||1000}`,`yapp_auto_cleanup=${b('settings-yapp-cleanup')}`];document.querySelectorAll('#interface-forms > .interface-form').forEach(card=>{const q=s=>card.querySelector(s),name=q('.if-name')?.value.trim()||'',transport=q('[id^="if-transport"]')?.value||'tcp',endpoint=q('[id^="if-endpoint"]')?.value.trim()||'',baud=q('[id^="if-baud"]')?.value||9600,encoded=transport==='serial'?`serial:${endpoint}@${baud}`:endpoint;lines.push(`interface_${name}=${encoded},${q('[id^="if-port"]')?.value||0},${q('[id^="if-priority"]')?.value||0},${q('.if-enabled')?.checked!==false},${q('[id^="if-description"]')?.value.trim()||''}`);lines.push(`beacon_interface_${name}=${q('.if-beacon')?.checked!==false},${q('.if-path')?.value.trim()||''}`);});return lines.join('\n')+'\n';}
async function loadSettings(){const [details,templates,webDetails]=await Promise.all([request('config/details'),request('templates'),request('web/details')]);await loadLanguage(details.language);ensureWebSettings();field('settings-web-bind',webDetails.bind);field('settings-web-token-file',webDetails.tokenFile||'');checked('settings-web-auth',webDetails.authEnabled);field('settings-web-presence',webDetails.presenceMode||'auto');field('settings-web-profile',webDetails.absenceProfile||2);field('settings-web-timeout',webDetails.heartbeatTimeout||90);checked('settings-statistics',webDetails.statisticsEnabled);const warning=$('settings-web-warning');if(warning)warning.hidden=bool(webDetails.authEnabled)||String(webDetails.bind||'').startsWith('127.0.0.1:')||String(webDetails.bind||'').startsWith('localhost:');ensureLocatorField();field('settings-callsign',details.callsign);field('settings-myname',details.myname);field('settings-locator',details.locator);field('settings-language',details.language);[['welcome','welcome'],['goodbye','goodbye'],['remoteCommands','remote'],['beacon','beacon'],['digipeat','digipeat'],['accept','accept'],['ax25Autonegotiation','autoneg'],['mailboxEnabled','mailbox'],['yappEnabled','yapp'],['yappAutoReceive','yapp-auto'],['yappAskLocation','yapp-location']].forEach(([key,id])=>checked(`settings-${id}`,details[key]));field('settings-beacon-seconds',Math.max(10,Math.round(Number(details.beaconSeconds||600)/60)));field('settings-beacon-to',details.beaconTo);field('settings-t1',details.t1);field('settings-t3',details.t3);field('settings-retries',details.retries);field('settings-paclen',details.paclen);field('settings-mailbox-call',details.mailboxCall);field('settings-yapp-dir',details.yappDownloadDir);state.telegramTokenPresent=Boolean(details.telegramTokenPresent);field('settings-yapp-max-files',details.yappMaxFiles||1000);checked('settings-yapp-cleanup',details.yappAutoCleanup);checked('settings-telegram-enabled',details.telegramEnabled);field('settings-telegram-chat-id',details.telegramChatId||'');updateTelegramStatus();(details.notifications||[]).forEach(item=>{checked(`notification-${item.kind}-in-app`,item.inApp);checked(`notification-${item.kind}-native`,item.native);checked(`notification-${item.kind}-telegram`,item.telegram);field(`notification-${item.kind}-sound`,item.sound);});updateModuleNotificationState();renderInterfaceForms(details.interfaces);state.templates=templates;$('template-text').value=templates[Number($('template-select').value)]||'';$('config-text').value=settingsConfigText();state.configText=$('config-text').value;}
function validateSettings(){const locator=v=>$(v)?.value.trim()||'',beaconMinutes=Number(locator('settings-beacon-seconds'));if(!Number.isInteger(beaconMinutes)||beaconMinutes<10||beaconMinutes>1440)throw settingsError('Interwał beaconu musi wynosić od 10 do 1440 minut.',$('settings-beacon-seconds'));if(locator('settings-locator')&&!/^[A-Ra-r]{2}[0-9]{2}([A-Xa-x]{2})?$/.test(locator('settings-locator')))throw settingsError('Lokator QTH musi mieć format Maidenhead, np. JO92AA.',$('settings-locator'));const cards=[...document.querySelectorAll('#interface-forms > .interface-form')],names=[];if(cards.length>10)throw settingsError('Można skonfigurować maksymalnie 10 interfejsów TNC.',cards[10]?.querySelector('.if-name'));for(const card of cards){const name=card.querySelector('.if-name'),transport=card.querySelector('.if-transport'),endpoint=card.querySelector('.if-endpoint'),baud=card.querySelector('.if-baud'),port=card.querySelector('.if-port'),priority=card.querySelector('.if-priority'),nameValue=name?.value.trim()||'';if(!/^[A-Za-z0-9_]{1,16}$/.test(nameValue))throw settingsError('Nazwa interfejsu musi mieć 1-16 znaków ASCII: litery, cyfry lub _.',name);if(names.includes(nameValue))throw settingsError('Nazwy interfejsów TNC muszą być unikalne.',name);names.push(nameValue);const endpointValue=endpoint?.value.trim()||'';if(transport?.value==='serial'){if(!endpointValue||(!/^COM[0-9]+$/i.test(endpointValue)&&!/^\/dev\/[^\s]+$/.test(endpointValue)))throw settingsError('Podaj poprawne urządzenie szeregowe, np. COM3 lub /dev/ttyUSB0.',endpoint);const baudValue=Number(baud?.value);if(![1200,2400,4800,9600,19200,38400,57600,115200].includes(baudValue))throw settingsError('Wybierz obsługiwaną prędkość portu szeregowego.',baud);}else{const match=endpointValue.match(/^(.+):([0-9]+)$/);if(!match||Number(match[2])<1||Number(match[2])>65535)throw settingsError('Adres TCP/IP musi mieć format host:port.',endpoint);}const kissPort=Number(port?.value);if(!Number.isInteger(kissPort)||kissPort<0||kissPort>15)throw settingsError('Port KISS musi mieć wartość od 0 do 15.',port);const interfacePriority=Number(priority?.value);if(!Number.isInteger(interfacePriority)||interfacePriority<0||interfacePriority>255)throw settingsError('Priorytet interfejsu musi mieć wartość od 0 do 255.',priority);}}

function importConfigForm(text){const values=new Map(),interfaces=[],profiles=new Map();for(const raw of text.split(/\r?\n/)){const line=raw.trim();if(!line||line.startsWith('#'))continue;const [key,...rest]=line.split('=');const value=rest.join('=');if(key.startsWith('interface_')){const parts=value.split(',',5),endpoint=parts[0]||'',serial=endpoint.startsWith('serial:'),match=serial?endpoint.slice(7).match(/^(.*)@(\d+)$/):null;interfaces.push({name:key.slice(10),description:parts[4]||'',endpoint:match?match[1]:endpoint,transport:serial?'serial':'tcp',baud:match?Number(match[2]):9600,port:Number(parts[1]||0),priority:Number(parts[2]||0),enabled:parts[3]!=='false'});}else if(key.startsWith('beacon_interface_')){const [enabled,...route]=value.split(',');profiles.set(key.slice(17),{beaconEnabled:enabled==='true',beaconPath:route.join(',').trim()});}else values.set(key,value);}interfaces.forEach(item=>Object.assign(item,profiles.get(item.name)||{}));const set=(id,key)=>field(id,values.get(key));ensureLocatorField();set('settings-callsign','callsign');set('settings-myname','myname');set('settings-locator','locator');set('settings-language','language');set('settings-t1','t1_ms');set('settings-t3','t3_ms');set('settings-retries','retries');set('settings-paclen','paclen');set('settings-beacon-seconds','beacon_seconds');field('settings-beacon-seconds',Math.max(10,Math.round(Number($('settings-beacon-seconds').value||600)/60)));set('settings-beacon-to','beacon_to');set('settings-mailbox-call','mailbox_callsign');set('settings-yapp-dir','yapp_download_dir');set('settings-yapp-max-files','yapp_max_files');checked('settings-yapp-cleanup',values.get('yapp_auto_cleanup')==='true');checked('settings-telegram-enabled',values.get('telegram_enabled')==='true');set('settings-telegram-chat-id','telegram_chat_id');updateTelegramStatus();['incoming_connection','outgoing_connection','terminal_message','disconnection','mailbox','yapp','tnc','critical'].forEach(k=>{checked(`notification-${k}-in-app`,values.get(`notification_${k}_in_app`)==='true');checked(`notification-${k}-native`,values.get(`notification_${k}_native`)!=='false');checked(`notification-${k}-telegram`,values.get(`notification_${k}_telegram`)==='true');field(`notification-${k}-sound`,values.get(`notification_${k}_sound`)||0);});[['welcome','welcome_enabled'],['goodbye','goodbye_enabled'],['remote','remote_commands'],['beacon','beacon_enabled'],['digipeat','digipeat_enabled'],['accept','accept_incoming'],['autoneg','ax25_autonegotiation'],['mailbox','mailbox_enabled'],['yapp','yapp_enabled'],['yapp-auto','yapp_auto_receive'],['yapp-location','yapp_ask_location']].forEach(([id,key])=>checked(`settings-${id}`,values.get(key)==='true'));renderInterfaceForms(interfaces);}

Object.assign(staticTranslations,{
  'Serwer WWW i logowanie':'Web server and login',
  'Czas uznania operatora za nieobecnego (s)':'Time before marking the operator absent (s)',
  'Obsługa modulo 128':'Modulo 128 support',
  'Negocjuj parametry AX.25 (XID)':'Negotiate AX.25 parameters (XID)'
});
function ensureAx25ModeControl(){
  const negotiation=$('settings-autoneg');
  if(!negotiation)return;
  if(!$('settings-modulo-128')){
    const label=document.createElement('label');
    label.innerHTML='<input id="settings-modulo-128" type="checkbox"> Obsługa modulo 128';
    negotiation.closest('label')?.before(label);
  }
  [['settings-modulo-128','Obsługa modulo 128'],['settings-autoneg','Negocjuj parametry AX.25 (XID)']].forEach(([id,caption])=>{
    const label=$(id)?.closest('label');
    const text=label&&[...label.childNodes].find(child=>child.nodeType===3&&child.textContent.trim());
    if(text)text.textContent=' '+caption;
  });
  const grid=negotiation.closest('.check-grid');
  const labels=['settings-accept','settings-modulo-128','settings-digipeat','settings-autoneg'].map(id=>$(id)?.closest('label'));
  if(grid&&labels.every(Boolean))grid.append(...labels);
  translateDom();
}
function syncPresenceTimeout(){
  const timeout=$('settings-web-timeout');
  if(timeout)timeout.disabled=$('settings-web-presence')?.value!=='auto';
}
function movePresenceSetting(){
  const setting=$('settings-presence-bar');
  const target=$('settings-web-presence')?.closest('.form-grid');
  const label=setting?.closest('label');
  if(target&&label){label.classList.add('web-presence-bar-option');target.append(label);}
  syncPresenceTimeout();
}
function ensureStatisticsRetentionControl(){if($('settings-statistics-retention'))return;const statistics=$('settings-statistics'),grid=statistics?.closest('.check-grid');if(!grid)return;const label=document.createElement('label');label.innerHTML='<span>Przechowywanie historii statystyk<select id="settings-statistics-retention"><option value="continuous">Ciągły</option><option value="7">7 dni</option><option value="30">30 dni</option><option value="90">90 dni</option><option value="365">365 dni</option></select></span>';grid.append(label);translateDom();}
const loadSettingsBase=loadSettings;
loadSettings=async function(){await loadSettingsBase();ensureAx25ModeControl();movePresenceSetting();ensureStatisticsRetentionControl();const [details,webDetails]=await Promise.all([request('config/details'),request('web/details')]);checked('settings-modulo-128',details.ax25Modulo128??true);field('settings-statistics-retention',webDetails.statisticsRetentionDays||'continuous');$('config-text').value=settingsConfigText();state.configText=$('config-text').value;};
const settingsConfigTextBase=settingsConfigText;
settingsConfigText=function(){const text=settingsConfigTextBase();return text.replace(/ax25_autonegotiation=(true|false)\n/,match=>match+`ax25_modulo_128=${Boolean($('settings-modulo-128')?.checked)}\n`);};
const importConfigFormBase=importConfigForm;
importConfigForm=function(text){importConfigFormBase(text);checked('settings-modulo-128',text.split(/\r?\n/).some(line=>line.trim()==='ax25_modulo_128=true'));};

// Keep all form examples neutral; real station data comes only from the user's profile.
function sanitizePublicExamples(){document.querySelectorAll('input[placeholder]').forEach(input=>{
  input.placeholder=input.placeholder.replace(/SR5DDD-10/g,'RMS-1').replace(/SR5DDD-1/g,'N0CALL-1');
});}
sanitizePublicExamples();
new MutationObserver(sanitizePublicExamples).observe(document.body,{childList:true,subtree:true});

async function testNotificationChannel(channel){
  if(channel==='telegram'){
    await request('telegram/test',{body:{chat_id:$('settings-telegram-chat-id')?.value.trim()||''}});
    toast('Testowa wiadomość Telegrama została wysłana.');
    return;
  }
  if(channel==='sound'&&!window.__UPR_DESKTOP__){playNotificationSound(1);toast('Test dźwięku w przeglądarce wykonany.');return;}
  await request('notifications/test',{body:{channel}});
  if(channel==='in-app'){
    const snapshot=await request('state',{method:'GET'});
    state.snapshot=snapshot;
    renderNotifications(snapshot);
  }
  toast('Test kanału powiadomień wykonany.');
}

// Keep notification channel names explicit without changing the configuration keys.
function normalizeNotificationLabels(){
  const section=document.querySelector('.notification-settings');
  if(!section)return;
  ensureNotificationVolumeControl(section);
  const description=section.querySelector(':scope > .hint');
  if(description)description.textContent=state.language==='en'?'Choose channels for each event. System notifications work in the desktop app; sound works in the desktop app and in the browser after audio is unlocked; statuses are also available in the web panel.':'Wybierz kanały dla każdego zdarzenia. Dymki działają w aplikacji desktopowej, dźwięk działa w aplikacji desktopowej i po odblokowaniu audio w przeglądarce, a statusy są dostępne także w panelu WWW.';
  const heading=section.querySelector(':scope > h3');
  if(heading&&!heading.querySelector('.notification-test-hint')){
    const hint=node('span','hint-inline',' (kliknięcie nazwy kanału wysyła test)');
    hint.className='hint-inline notification-test-hint';
    heading.append(hint);
  }
  const labels=state.language==='en'?['Notification center','System notification','Telegram','Sound']:['Centrum powiadomień','Dymek','Telegram','Dźwięk'];
  const channels=['in-app','native','telegram','sound'];
  section.querySelectorAll('.notification-table th').forEach((header,index)=>{
    if(index===0||!labels[index-1])return;
    let button=header.querySelector('.notification-channel-test');
    if(!button){
      button=node('button','notification-channel-test');
      button.type='button';
      button.addEventListener('click',()=>guard(()=>testNotificationChannel(channels[index-1])));
      header.replaceChildren(button);
    }
    button.textContent=labels[index-1];
  });
}
function ensureNotificationVolumeControl(section){
  let control=$('notification-volume-control');
  if(window.__UPR_DESKTOP__){if(control)control.hidden=true;return;}
  if(!control){
    control=node('div','notification-volume-control');control.id='notification-volume-control';
    const copy=node('div','notification-volume-copy'),label=node('label'),hint=node('p','hint');
    label.htmlFor='notification-volume';label.id='notification-volume-label';
    hint.id='notification-volume-hint';copy.append(label,hint);
    const actions=node('div','notification-volume-actions'),slider=node('input'),value=node('output');
    slider.id='notification-volume';slider.type='range';slider.min='0';slider.max='100';slider.step='1';
    slider.setAttribute('aria-describedby',hint.id);value.id='notification-volume-value';value.htmlFor=slider.id;
    slider.addEventListener('input',()=>{
      notificationVolume=Number(slider.value);
      try{localStorage.setItem(notificationVolumeKey,String(notificationVolume));}catch{}
      ensureNotificationVolumeControl(section);
    });
    const test=button('',()=>playNotificationSound(1),'button secondary small');test.id='notification-volume-test';
    actions.append(slider,value,test);control.append(copy,actions);section.append(control);
  }
  control.hidden=false;
  const pl=state.language==='pl';
  $('notification-volume-label').textContent=pl?'Głośność powiadomień w przeglądarce':'Browser notification volume';
  $('notification-volume-hint').textContent=pl?'Zapisywane automatycznie tylko w tej przeglądarce. 0% wycisza dźwięk.':'Saved automatically in this browser only. 0% mutes sound.';
  $('notification-volume').value=notificationVolume;
  $('notification-volume-value').textContent=`${notificationVolume}%`;
  $('notification-volume').setAttribute('aria-valuetext',notificationVolume===0?(pl?'Wyciszone':'Muted'):`${notificationVolume}%`);
  const test=$('notification-volume-test');test.textContent=pl?'Odtwórz test':'Play test';
  test.disabled=!window.AudioContext||notificationVolume===0;
}
normalizeNotificationLabels();
setInterval(normalizeNotificationLabels,1000);

function ensureMqttSettingsPanel(){
  const web=document.querySelector('.settings-form .form-section[data-settings-key="web"]');
  if(!web||$('settings-mqtt-card'))return;
  const card=node('div','interface-form settings-collapsible collapsed');card.id='settings-mqtt-card';
  const header=node('div','section-title'),strong=node('strong','', 'MQTT'),toggle=node('button','text-button interface-toggle',interfaceToggleText(false));
  toggle.type='button';toggle.setAttribute('aria-expanded','false');
  const body=node('div','interface-body');
  body.innerHTML='<p class="section-help">Publikuje wyłącznie żywotność aplikacji, stan interfejsów TNC i listę MHeard. MQTT jest wyłączone domyślnie. Po zmianie ustawień wymagany jest restart aplikacji.</p><div class="form-grid"><label><input id="settings-mqtt-enabled" type="checkbox"> Włącz MQTT</label><label>Broker MQTT<input id="settings-mqtt-broker" placeholder="mqtt://adres:1883" maxlength="256"></label><label>Użytkownik (opcjonalnie)<input id="settings-mqtt-username" maxlength="128"></label><label>Identyfikator klienta<input id="settings-mqtt-client-id" maxlength="128" placeholder="ultimatepr-station"></label><label>Prefiks tematów<input id="settings-mqtt-topic-prefix" maxlength="128" placeholder="ultimatepr"></label><label>Temat Discovery<input id="settings-mqtt-discovery-topic" maxlength="128" placeholder="homeassistant"></label><label><input id="settings-mqtt-tls" type="checkbox"> TLS (mqtts)</label></div><div class="actions"><input id="settings-mqtt-password" type="password" autocomplete="new-password" maxlength="256" placeholder="hasło nie jest zapisywane w web.conf"><button id="settings-mqtt-password-save" type="button" class="button secondary small">Zapisz hasło MQTT</button><small id="settings-mqtt-password-status" class="hint">Hasło: nieustawione</small></div><div class="actions mqtt-discovery-actions"><button id="settings-mqtt-discovery-send" type="button" class="button secondary small">Wyślij Home Assistant Discovery</button><small class="hint">Wysyła konfigurację encji jednorazowo na żądanie.</small></div>';
  const toggleCard=event=>{if(event.target.closest('button'))return;toggle.click();};
  toggle.onclick=()=>{const collapsed=card.classList.toggle('collapsed');toggle.textContent=interfaceToggleText(!collapsed);toggle.setAttribute('aria-expanded',String(!collapsed));};
  header.addEventListener('click',toggleCard);header.append(strong,toggle);card.append(header,body);web.append(card);appendCardHelp(card,'mqtt');
  $('settings-mqtt-password-save').onclick=()=>guard(async()=>{const password=$('settings-mqtt-password').value;await request('mqtt/password',{body:{password}});$('settings-mqtt-password').value='';const present=Boolean(password);$('settings-mqtt-password-status').textContent=`Hasło: ${present?'zapisane w osobnym pliku':'nieustawione'}`;state.webDetails={...(state.webDetails||{}),mqttPasswordPresent:present};toast(present?'Hasło MQTT zapisane.':'Hasło MQTT usunięte.');});
  $('settings-mqtt-discovery-send').onclick=()=>guard(async()=>{await request('mqtt/discovery');toast('Home Assistant Discovery wysłane.');});
}
let updateStatusTimer=null;
let updateStatusRequest=null;
let updateStatusLast=null;
function ensureUpdateSettingsPanel(){
  const web=document.querySelector('.settings-form .form-section[data-settings-key="web"]');
  if(!web)return;
  const existing=$('settings-update-card');
  if(existing){
    // Keep this card at the WWW-section level. This repairs older sessions
    // where a repeated layout pass placed it inside another collapsible card.
    if(existing.parentElement!==web)web.append(existing);
    return;
  }
  const card=node('div','interface-form settings-collapsible');
  card.id='settings-update-card';
  card.classList.add('collapsed');
  const header=node('div','section-title');
  const strong=node('strong','', 'Aktualizacja');
  const toggle=node('button','text-button interface-toggle',interfaceToggleText(false));
  toggle.type='button';
  toggle.setAttribute('aria-expanded','false');
  const body=node('div','interface-body');
  body.innerHTML='<p class="section-help">Wybierz paczkę ZIP przygotowaną dla Alpine. Instalacja zatrzyma usługę, sprawdzi paczkę i uruchomi ją ponownie.</p><p class="hint update-version" id="settings-update-version">Obecna wersja: sprawdzanie… <button id="settings-update-version-check" type="button" class="button secondary small">Sprawdź wersję</button></p><div class="actions"><button id="settings-update-choose" type="button" class="button secondary">Wybierz ZIP</button><button id="settings-update-install" type="button" class="button primary" disabled>Zainstaluj aktualizację</button><input id="settings-update-file" type="file" accept=".zip" hidden></div><p id="settings-update-file-info" class="hint">Nie wybrano paczki.</p><pre id="settings-update-terminal" class="update-terminal" role="status" aria-live="polite">Status aktualizacji: nie sprawdzono.</pre>';
  const toggleCard=event=>{if(event.target.closest('button'))return;toggle.click();};
  toggle.onclick=()=>{
    const collapsed=card.classList.toggle('collapsed');
    toggle.textContent=interfaceToggleText(!collapsed);
    toggle.setAttribute('aria-expanded',String(!collapsed));
    if(!collapsed){scrollExpandedSettingsCard(card);window.refreshUpdateStatus?.();}
  };
  header.addEventListener('click',toggleCard);
  header.append(strong,toggle);
  card.append(header,body);
  web.append(card);
  const file=$('settings-update-file'),choose=$('settings-update-choose'),install=$('settings-update-install');
  const info=$('settings-update-file-info'),terminal=$('settings-update-terminal'),versionInfo=$('settings-update-version'),versionCheck=$('settings-update-version-check');
  const refreshUpdateVersion=async()=>{
    if(!versionInfo)return;
    try{
      const result=await request('build-info',{method:'GET'});
      versionInfo.firstChild.textContent=`Obecna wersja: ${result?.version||'nieznana'} `;
    }catch{versionInfo.firstChild.textContent='Obecna wersja: niedostępna ';}
  };
  versionCheck?.addEventListener('click',refreshUpdateVersion);
  refreshUpdateVersion();
  const renderUpdateStatus=status=>{
    if(!status||!terminal)return;
    updateStatusLast=status;
    const stateName=status.state||'idle';
    const version=status.version||'—';
    terminal.textContent=`Stan: ${stateName}\nWersja: ${version}\n${status.log||'Brak statusu.'}`;
    if(stateName==='completed'||stateName==='failed'||stateName==='rolled-back'){
      if(updateStatusTimer){clearInterval(updateStatusTimer);updateStatusTimer=null;}
    }
  };
  window.refreshUpdateStatus=async()=>{
    if(updateStatusRequest)return updateStatusRequest;
    updateStatusRequest=(async()=>{
      try{
        renderUpdateStatus(await request('update/status',{method:'GET'}));
      }catch(exception){
        if(terminal){
          const prefix=updateStatusLast?'Status chwilowo niedostępny — usługa może być restartowana.':'Nie można jeszcze pobrać statusu aktualizacji.';
          terminal.textContent=`${prefix}\nPonawiam sprawdzanie…`;
        }
      }finally{updateStatusRequest=null;}
    })();
    return updateStatusRequest;
  };
  const startStatusPolling=()=>{
    if(updateStatusTimer)clearInterval(updateStatusTimer);
    updateStatusTimer=setInterval(()=>window.refreshUpdateStatus?.(),2000);
    window.refreshUpdateStatus?.();
  };
  choose.onclick=()=>file.click();
  file.onchange=()=>guard(async()=>{
    const selected=file.files?.[0];
    if(!selected)return;
    choose.disabled=true;install.disabled=true;
    info.textContent=`Wybrano: ${selected.name} · ${Math.ceil(selected.size/1024)} KiB`;
    try{
      const result=await request('update/upload',{raw:selected});
      install.disabled=false;
      renderUpdateStatus({state:'uploaded',version:'—',log:`Paczka przesłana. SHA-256: ${result.sha256||'brak'}`});
      toast('Paczka aktualizacji przesłana.');
    }finally{choose.disabled=false;file.value='';}
  });
  install.onclick=()=>guard(async()=>{
    if(!window.confirm('Zainstalować wybraną paczkę? Usługa zostanie chwilowo zatrzymana.'))return;
    install.disabled=true;
    await request('update/install',{body:{confirm:'yes'}});
    renderUpdateStatus({state:'running',version:'—',log:'Uruchomiono aktualizację…'});
    startStatusPolling();
  });
}

// Split the existing combined settings section without changing any field ids or config keys.
function splitApplicationAndWebSettings(){
  const form=document.querySelector('.settings-form');
  const web=form?.querySelector('.form-section[data-settings-key="application"]');
  if(!form||!web||form.dataset.applicationWebSplit==='true')return;
  web.dataset.settingsKey='web';
  const heading=web.querySelector(':scope > h3');
  if(heading)heading.textContent='WWW';
  const help=web.querySelector(':scope > .section-help');
  if(help)help.textContent='Panel WWW, dostęp operatora i ustawienia serwera.';

  const application=node('div','form-section');
  application.dataset.settingsKey='application';
  application.innerHTML='<h3>Aplikacja</h3><p class="section-help">Powiadomienia, dźwięki i Telegram.</p>';
  web.parentElement.insertBefore(application,web);

  [...web.querySelectorAll(':scope > .settings-collapsible')].forEach(card=>{
    if(card.querySelector('.telegram-settings,.notification-settings'))application.append(card);
  });
  const messages=form.querySelector('.form-section[data-settings-key="messages"]');
  const template=messages?.querySelector('#template-select')?.closest('.panel');
  if(template&&messages)messages.append(template);
  collapseTemplateSettings(messages);
  application.dataset.settingsGroupsReady='true';
  form.dataset.applicationWebSplit='true';
}

function settingsTabItemsSeparated(){
  const form=document.querySelector('.settings-form');
  if(!form)return[];
  const labels={station:['settings.station','Stacja'],interfaces:['settings.interfaces','Interfejsy TNC'],beacon:['settings.beacon','Beacon'],mailbox:['settings.mailbox','MailBox'],messages:['settings.messages','Komunikaty i komendy'],yapp:['settings.yapp','YAPP'],ax25:['settings.ax25','AX.25 i ruch'],application:['settings.application','Aplikacja'],web:['settings.web','WWW']};
  const order=['station','interfaces','beacon','mailbox','messages','yapp','ax25','application','web'];
  const sections=[...form.querySelectorAll('.form-section[data-settings-key]')];
  return order.map(type=>{
    const element=sections.find(section=>section.dataset.settingsKey===type);
    if(!element)return null;
    if(type==='web'&&(state.webDetails?.runtime==='desktop'||state.webDetails?.webPanel===false))return null;
    const [key,fallback]=labels[type];
    return {element,key,fallback,section:true};
  }).filter(Boolean);
}

settingsTabItems=settingsTabItemsSeparated;
helpDescriptions.application={pl:{title:'Aplikacja',summary:'Powiadomienia, dźwięki i Telegram.',items:['Centrum powiadomień pokazuje zdarzenia na górnym pasku aplikacji.','Dymek oznacza systemowe powiadomienie desktopowe.','Dźwięk zależy od środowiska: system desktopowy albo przeglądarka.']},en:{title:'Application',summary:'Notifications, sounds and Telegram.',items:['The notification center shows events in the application header.','System notification means a desktop notification.','Sound depends on the environment: desktop system or browser.']}};
helpDescriptions.web={pl:{title:'WWW',summary:'Panel WWW, dostęp operatora i ustawienia serwera.',items:['Desktop korzysta z lokalnego, dynamicznie wybranego portu.','Wariant server używa skonfigurowanego adresu, domyślnie portu 8080.','Ustawienia obecności i heartbeat dotyczą komunikacji panelu z backendem.']},en:{title:'Web',summary:'Web panel, operator access and server settings.',items:['Desktop uses a local dynamically selected port.','The server variant uses the configured address, port 8080 by default.','Presence and heartbeat settings control panel-to-backend communication.']}};
helpDescriptions.mqtt={pl:{title:'MQTT',summary:'Bezpieczna publikacja statusu stacji do brokera MQTT.',items:['MQTT nie wymaga bota — potrzebny jest broker, np. Mosquitto lub broker Home Assistant.','Włącz MQTT dopiero po wpisaniu adresu brokera oraz opcjonalnych danych użytkownika.','Przycisk „Wyślij Home Assistant Discovery” publikuje konfigurację encji jednorazowo; po zmianie prefiksu lub znaku stacji użyj go ponownie.','Aplikacja publikuje tylko status aplikacji, TNC i MHeard; nie odbiera poleceń MQTT.']},en:{title:'MQTT',summary:'Secure publication of station status to an MQTT broker.',items:['MQTT does not use a bot — it requires a broker such as Mosquitto or a Home Assistant broker.','Enable MQTT after entering the broker address and optional credentials.','The “Send Home Assistant Discovery” button publishes entity configuration once; use it again after changing the prefix or station callsign.','The application publishes only application status, TNC and MHeard; it does not accept MQTT commands.']}};
splitApplicationAndWebSettings();
ensureMqttSettingsPanel();
ensureUpdateSettingsPanel();
ensureSettingsTabs(true);
staticTranslations['Przechowywanie historii statystyk']='Statistics history retention';staticTranslations['Ciągły']='Continuous';staticTranslations['7 dni']='7 days';staticTranslations['30 dni']='30 days';staticTranslations['90 dni']='90 days';staticTranslations['365 dni']='365 days';

const saveWebConfigurationBase=saveWebConfiguration;
saveWebConfiguration=async function(){const result=await saveWebConfigurationBase();const retention=$('settings-statistics-retention');if(retention){await request('web/save',{body:{bind:$('settings-web-bind')?.value.trim()||'127.0.0.1:8080',token_file:$('settings-web-token-file')?.value.trim()||'',statistics_enabled:String(Boolean($('settings-statistics')?.checked)),statistics_retention_days:retention.value||'continuous'}});}return result;};

function syncSettingsRuntime(){
  const form=document.querySelector('.settings-form');
  const runtime=state.webDetails?.runtime;
  if(!form||!runtime||form.dataset.settingsRuntime===runtime)return;
  form.dataset.settingsRuntime=runtime;
  ensureSettingsTabs(true);
}
setInterval(syncSettingsRuntime,500);

// Notification event split: keep legacy configuration readable while exposing separate controls.
(function(){
  // Application-started notifications are generated before the settings form
  // is available and are not part of the editable station configuration.
  // Do not serialize that row: older server binaries reject the key and then
  // block saving every other setting in the form.
  const rows=[['tnc_connected','Połączenie z TNC'],['tnc_disconnected','Rozłączenie TNC'],['mheard_new_station','Nowa stacja w MHeard']];
  function ensureNotificationRows(){
    const tbody=document.querySelector('.notification-table tbody');
    if(!tbody)return;
    tbody.querySelector('[data-notification-kind="tnc"]')?.remove();
    const critical=tbody.querySelector('[data-notification-kind="critical"]');
    rows.forEach(([kind,label])=>{
      if(tbody.querySelector('[data-notification-kind="'+kind+'"]'))return;
      const row=document.createElement('tr');
      row.dataset.notificationKind=kind;
      row.innerHTML='<td>'+label+'</td><td><input id="notification-'+kind+'-in-app" type="checkbox"></td><td><input id="notification-'+kind+'-native" type="checkbox"></td><td><input id="notification-'+kind+'-telegram" type="checkbox"></td><td><select id="notification-'+kind+'-sound"><option value="0">Brak</option><option value="1">1 dzwonek</option><option value="2">2 dzwonki</option><option value="3">3 dzwonki</option></select></td>';
      const anchor=kind==='mheard_new_station'?tbody.querySelector('[data-notification-kind="mailbox"]'):critical;
      anchor?tbody.insertBefore(row,anchor):tbody.append(row);
    });
  }
  function notificationLines(){
    const fields=[['in_app','in-app'],['native','native'],['telegram','telegram'],['sound','sound']];
    return rows.flatMap(([kind])=>fields.map(([fieldName,channel])=>`notification_${kind}_${fieldName}=${channel==='sound'?($('notification-'+kind+'-sound')?.value||0):notificationSetting(kind,channel,$('notification-'+kind+'-'+channel)?.checked||false)}`));
  }
  const oldSettingsConfigText=settingsConfigText;
  settingsConfigText=function(){
    const legacy=oldSettingsConfigText().split('\n').filter(line=>!line.startsWith('notification_tnc_'));
    return [...legacy,...notificationLines()].join('\n');
  };
  const oldImportConfigForm=importConfigForm;
  importConfigForm=function(text){
    oldImportConfigForm(text);
    const values=new Map();
    text.split(/\r?\n/).forEach(raw=>{const line=raw.trim(),index=line.indexOf('=');if(index>0)values.set(line.slice(0,index),line.slice(index+1));});
    const legacy=field=>values.get('notification_tnc_'+field);
    rows.forEach(([kind])=>['in_app','native','telegram','sound'].forEach(fieldName=>{
      const value=values.get('notification_'+kind+'_'+fieldName)??(kind==='mheard_new_station'?(fieldName==='sound'?'0':'false'):legacy(fieldName));
      if(value===undefined)return;
      if(fieldName==='sound')field('notification-'+kind+'-sound',value);else checked('notification-'+kind+'-'+(fieldName==='in_app'?'in-app':fieldName),value==='true');
    }));
  };
  const oldLoadSettings=loadSettings;
  loadSettings=async function(){ensureNotificationRows();return oldLoadSettings();};
  document.addEventListener('DOMContentLoaded',ensureNotificationRows);
})();








