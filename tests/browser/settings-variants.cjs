// Product-specific settings contracts; no browser or third-party modules required.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const vm=require('node:vm');
const source=fs.readFileSync(path.join(__dirname,'../../web/app.js'),'utf8');
const html=fs.readFileSync(path.join(__dirname,'../../web/index.html'),'utf8');
function extract(start,end){const a=source.indexOf(start),b=source.indexOf(end,a);assert(a>=0&&b>a);return source.slice(a,b);}
const sections=['station','interfaces','beacon','mailbox','messages','yapp','ax25','notifications','application','web'].map(settingsKey=>({dataset:{settingsKey}}));
const form={querySelectorAll:()=>sections};
const control={checked:false,disabled:true},heading={},label={},hint={};
const requests=[];
const context=vm.createContext({window:{},state:{webDetails:{runtime:'unexpected'},catalog:{},language:'pl'},
  $:id=>({'settings-desktop-autostart':control,'settings-desktop-autostart-label':label,'settings-desktop-autostart-hint':hint}[id]),
  document:{querySelector:selector=>selector==='.settings-form'?form:heading},
  request:async route=>{requests.push(route);return{enabled:true};},
});
vm.runInContext(extract('function settingsTabItemsSeparated(){','\nsettingsTabItems=settingsTabItemsSeparated;')+
 extract('function settingsDesktopRuntime(){','\nsetInterval(syncSettingsRuntime,500);'),context);
const keys=Array.from(context.settingsTabItemsSeparated(),item=>item.key);
assert(keys.includes('settings.notifications'));
assert.equal(keys.includes('settings.application'),false);
assert.equal(keys.includes('settings.web'),true);
context.syncNotificationChannels();
assert.equal(heading.textContent,'Powiadomienia');
context.state.language='en';context.syncNotificationChannels();assert.equal(heading.textContent,'Notifications');
assert(!html.match(/id="notification-[^"]*-native"/));
assert(source.includes("const channels=['in-app','telegram','sound'];"));
assert(!source.includes("request('desktop/"));
assert(!source.includes('id="settings-desktop-autostart"'));
assert(!fs.existsSync(path.join(__dirname,'../../desktop/Cargo.toml')));
console.log('PASS: Station settings, Telegram/browser audio and no native desktop controls.');
