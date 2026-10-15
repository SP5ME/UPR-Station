const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const vm=require('node:vm');
const source=fs.readFileSync(path.join(__dirname,'../../web/app.js'),'utf8');
function extract(start,end){const a=source.indexOf(start),b=source.indexOf(end,a);assert(a>=0&&b>a);return source.slice(a,b);}
async function check(flag,runtime,desktop){
  const sections=['station','interfaces','beacon','mailbox','messages','yapp','ax25','notifications','application','web'].map(settingsKey=>({dataset:{settingsKey}}));
  const form={querySelectorAll:()=>sections};
  const rows=[0,1,2].map(()=>({children:Array.from({length:5},()=>({hidden:false}))}));
  const telegram={toggleAttribute(name,value){this[name]=value;}};
  const control={checked:false,disabled:true},label={},hint={},heading={};
  const fields={'settings-desktop-autostart':control,'settings-desktop-autostart-label':label,'settings-desktop-autostart-hint':hint};
  const requests=[];
  const context=vm.createContext({window:{__UPR_DESKTOP__:flag},state:{webDetails:{runtime},catalog:{},language:'pl'},
    $:id=>fields[id],document:{
      querySelector(selector){if(selector==='.settings-form')return form;if(selector==='.telegram-settings')return{closest:()=>telegram};return heading;},
      querySelectorAll:()=>rows,
    },request:async route=>{requests.push(route);return{enabled:true};},
  });
  vm.runInContext(extract('function settingsTabItemsSeparated(){','\nsettingsTabItems=settingsTabItemsSeparated;')+
    extract('function settingsDesktopRuntime(){','\nsetInterval(syncSettingsRuntime,500);'),context);
  const tabs=context.settingsTabItemsSeparated();
  const keys=Array.from(tabs,item=>item.key);
  assert(keys.includes('settings.notifications'));
  assert.equal(keys.includes('settings.application'),desktop);
  assert.equal(keys.includes('settings.web'),!desktop);
  if(desktop)assert.equal(tabs.find(item=>item.key==='settings.application').section,false,'Autostart saves independently');
  context.syncNotificationChannels();
  for(const row of rows){assert.equal(row.children[2].hidden,!desktop);assert.equal(row.children[3].hidden,desktop);assert.equal(row.children[4].hidden,false);}
  assert.equal(telegram.hidden,desktop);
  assert.equal(heading.textContent,'Powiadomienia');
  await context.loadDesktopApplicationSettings();
  assert.equal(requests.length,desktop?1:0,'Server must never request desktop preferences');
  if(desktop){assert.equal(control.checked,true);assert.equal(control.disabled,false,'Autostart is available after preferences load');}
  context.state.language='en';context.syncNotificationChannels();assert.equal(heading.textContent,'Notifications');
}
(async()=>{
  await check(true,'desktop',true);await check(false,'desktop',true);await check(false,'server',false);
  const desktop=fs.readFileSync(path.join(__dirname,'../../desktop/src/main.rs'),'utf8');
  assert(!desktop.includes('("autostart", "Start po zalogowaniu"'));
  assert(!desktop.includes('&checks["autostart"]'));
  assert(source.includes('control.checked=previous;throw exception;'),'Restore checkbox after save failure');
  console.log('PASS: variant tabs, channel columns, Telegram card, autostart read/error handling and removed menu entry.');
})().catch(error=>{console.error(error);process.exitCode=1;});
