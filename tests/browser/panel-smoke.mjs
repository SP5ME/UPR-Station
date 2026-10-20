// Manual integration test against a locally started isolated Station and headless Chrome.
import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
const port=process.argv[2]||'19333';
const pages=await fetch(`http://127.0.0.1:${port}/json/list`).then(r=>r.json());
const page=pages.find(p=>p.type==='page'&&p.url.startsWith('http://127.0.0.1:19334/'));
assert(page,'Isolated Station page not found');
const ws=new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve,reject)=>{ws.addEventListener('open',resolve,{once:true});ws.addEventListener('error',reject,{once:true});});
let id=0;
const pending=new Map(),exceptions=[];
ws.addEventListener('message',e=>{const m=JSON.parse(e.data);if(m.method==='Runtime.exceptionThrown')exceptions.push(m.params.exceptionDetails);if(pending.has(m.id)){pending.get(m.id)(m);pending.delete(m.id);}});
async function rpc(method,params={}){
  const number=++id;
  const response=new Promise(resolve=>pending.set(number,resolve));
  ws.send(JSON.stringify({id:number,method,params}));
  const timer=setTimeout(()=>{console.error('CDP timeout: '+method);process.exit(1);},15000);
  const message=await response;clearTimeout(timer);assert(!message.error,JSON.stringify(message.error));return message.result;
}
async function evaluate(expression){const result=await rpc('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});assert(!result.exceptionDetails,JSON.stringify(result.exceptionDetails));return result.result.value;}
try{
  await rpc('Runtime.enable');await rpc('Page.navigate',{url:page.url});
  for(let i=0;i<60;i++){if(await evaluate('document.readyState==="complete" && !!document.querySelector("#settings-update-card")'))break;await new Promise(r=>setTimeout(r,200));}
  await evaluate('loadSettings()');
  assert.equal(await evaluate('document.title'),'UPR Station');
  assert.equal(await evaluate('!!document.querySelector("#settings-desktop-autostart")'),false);
  assert.equal(await evaluate('document.querySelectorAll(".form-section[data-settings-key=application]").length'),0);
  assert.equal(await evaluate('document.querySelectorAll(".notification-table thead th").length'),4);
  assert.equal(await evaluate('!!document.querySelector(".telegram-settings,#settings-mqtt-card,#settings-web-bind")'),true);
  assert.equal(await evaluate('document.querySelector("#settings-update-card").classList.contains("collapsed")'),true);
  assert.deepEqual(await evaluate('Array.from(document.querySelector("#settings-update-channel").options,o=>o.value)'),['test','stable']);
  assert.equal(await evaluate('document.querySelector("#settings-update-channel").value'),'stable');
  assert.equal(await evaluate('document.querySelector("#settings-update-download").disabled'),true);
  assert.equal(await evaluate('document.querySelector("#settings-update-notes").hidden'),true);
  assert.equal(await evaluate('document.querySelector("#settings-update-install").disabled'),true);
  const invalidCheck=await evaluate('request("update/check",{method:"GET",query:{channel:"invalid"}}).then(()=>"unexpected success",error=>error.message)');
  assert(invalidCheck.includes('kanał aktualizacji'));
  assert.equal(await evaluate('document.querySelector("#token-dialog").open'),false,'Checking versions on a public panel must not open login');
  const before=await evaluate('request("state",{method:"GET"})');
  assert(before.interfaces.every(i=>!i.enabled));
  await evaluate('field("settings-myname","Isolated Station split test")');
  assert.equal(await evaluate('saveAllConfiguration(false)'),true);
  assert.equal((await evaluate('request("config/details")')).myname,'Isolated Station split test');
  await rpc('Emulation.setDeviceMetricsOverride',{width:1440,height:1000,deviceScaleFactor:1,mobile:false});
  await evaluate('showView("settings")');await evaluate('loadSettings()');
  await evaluate('selectSettingsTab(settingsTabItems().findIndex(item=>item.key==="settings.web"));document.querySelector("#settings-update-card .interface-toggle").click();');
  await new Promise(resolve=>setTimeout(resolve,300));
  assert.equal(await evaluate('getComputedStyle(document.querySelector(".update-sources")).gridTemplateColumns.split(" ").length'),2);
  const toolbar=await evaluate('(()=>{const row=document.querySelector("#settings-update-card .update-toolbar"),select=row.querySelector("select").getBoundingClientRect(),button=row.querySelector("button").getBoundingClientRect();return Math.abs(select.bottom-button.bottom);})()');
  assert(toolbar<2,'Channel and check button must share a baseline');
  if(process.env.UPR_SMOKE_SCREENSHOT){
    const clip=await evaluate('(()=>{const r=document.querySelector("#settings-update-card").getBoundingClientRect();return {x:r.x+scrollX,y:r.y+scrollY,width:r.width,height:r.height,scale:1};})()');
    const screenshot=await rpc('Page.captureScreenshot',{format:'png',clip,captureBeyondViewport:true});
    await writeFile(process.env.UPR_SMOKE_SCREENSHOT,Buffer.from(screenshot.data,'base64'));
  }
  await rpc('Emulation.setDeviceMetricsOverride',{width:600,height:1100,deviceScaleFactor:1,mobile:false});
  assert.equal(await evaluate('getComputedStyle(document.querySelector(".update-sources")).gridTemplateColumns.split(" ").length'),1);
  assert.equal(await evaluate('(()=>{const card=document.querySelector("#settings-update-card");return card.scrollWidth<=card.clientWidth+1;})()'),true,'Update controls must not overflow on mobile');
  assert.deepEqual(exceptions,[],'No JavaScript errors during initialization and settings save');
  console.log('PASS: Station browser initialization, product channels, server settings and configuration save.');
}catch(error){console.error(JSON.stringify(exceptions));throw error;}finally{ws.close();}
