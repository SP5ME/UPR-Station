// CDP is enabled only by the Windows smoke-test harness, against an isolated demo.
import assert from 'node:assert/strict';
import { writeFile, readFile, mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
const port = process.argv[2] || '19222';
const pages = await fetch(`http://127.0.0.1:${port}/json/list`).then(r => r.json());
const page = pages.find(p => p.type === 'page' && p.url.startsWith('http://127.0.0.1:'));
assert(page, 'Local panel not found');
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { ws.addEventListener('open', resolve, { once: true }); ws.addEventListener('error', reject, { once: true }); });
let id = 0;
const pending = new Map();
ws.addEventListener('message', e => {
  const message = JSON.parse(e.data);
  if (pending.has(message.id)) { pending.get(message.id)(message); pending.delete(message.id); }
});
async function rpc(method, params = {}) {
  const number = ++id;
  const response = new Promise(resolve => pending.set(number, resolve));
  ws.send(JSON.stringify({ id: number, method, params }));
  const timeout = setTimeout(() => { console.error(`CDP timeout: ${method}`); process.exit(1); }, 15000);
  const message = await response;
  clearTimeout(timeout);
  assert(!message.error, JSON.stringify(message.error));
  return message.result;
}
async function evaluate(expression) {
  const result = await rpc('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  assert(!result.exceptionDetails, JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
try {
  for (let i = 0; i < 40; i++) {
    if (await evaluate('document.readyState === "complete" && !!document.querySelector("#settings-web-bind")')) break;
    await new Promise(r => setTimeout(r, 250));
  }
  assert.equal(await evaluate('window.stationDesktop'), true);
  assert.equal(await evaluate('!!document.querySelector("#view-terminal")'), true);
  assert.equal(await evaluate('document.querySelector("#settings-web-bind").disabled'), true);
  assert.equal((await fetch(new URL('/api/state', page.url))).status, 401);
  assert.equal(await evaluate('request("state",{method:"GET"}).then(s=>s.callsign)'), 'TEST-1');
  await evaluate('request("connect",{body:{session:0,interface:0,call:"DEMO",via:""}})');
  let connected = false;
  for (let i = 0; i < 30; i++) {
    connected = await evaluate('request("state",{method:"GET"}).then(s=>s.sessions.some(x=>x.state==="Connected"))');
    if (connected) break;
    await new Promise(r => setTimeout(r, 200));
  }
  assert(connected, 'Demo AX.25 connection did not establish');
  await evaluate('request("send",{body:{session:0,text:"DESKTOP-SMOKE"}})');
  let echoed = false;
  for (let i = 0; i < 30; i++) {
    echoed = await evaluate('request("state",{method:"GET"}).then(s=>JSON.stringify(s).includes("Echo: DESKTOP-SMOKE"))');
    if (echoed) break;
    await new Promise(r => setTimeout(r, 200));
  }
  assert(echoed, 'Demo echo missing');
  await mkdir('target/desktop-downloads',{recursive:true});
  await rpc('Browser.setDownloadBehavior',{behavior:'allow',downloadPath:resolve('target/desktop-downloads')});
  // Exercise the same blob-based download mechanism used by the existing panel.
  await evaluate('download(new Blob(["desktop-download-check"],{type:"text/plain"}),"desktop-smoke.txt")');
  for(let i=0;i<30;i++){try{if(await readFile('target/desktop-downloads/desktop-smoke.txt','utf8')==='desktop-download-check')break;}catch{}await new Promise(r=>setTimeout(r,200));}
  assert.equal(await readFile('target/desktop-downloads/desktop-smoke.txt','utf8'),'desktop-download-check');
  await evaluate('request("state",{method:"GET"}).then(render)');
  const screen = await rpc('Page.captureScreenshot', { format: 'png' });
  await writeFile('target/desktop-smoke.png', Buffer.from(screen.data, 'base64'));
  console.log('PASS: native WebView panel, scoped token, API authentication, desktop settings, AX.25 demo echo, blob download verified');
} catch (error) {
  console.error(await evaluate('JSON.stringify({url:location.href,ready:document.readyState,panel:!!document.querySelector("#view-terminal"),desktop:!!window.stationDesktop,token:!!sessionStorage.getItem("station-token")})'));
  throw error;
} finally { ws.close(); }
