// Browser audio level regression tests; no browser or dependencies required.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const vm=require('node:vm');
const source=fs.readFileSync(path.join(__dirname,'../web/app.js'),'utf8');
const start=source.indexOf('let notificationAudioContext=null;');
const end=source.indexOf('function notificationTime',start);
assert(start>=0&&end>start);
function audio(stored=null,{desktop=false,blocked=false}={}){
  const peaks=[];
  class AudioContext {
    currentTime=0;state='running';destination={};
    createOscillator(){return {frequency:{},connect(){return {connect(){}};},start(){},stop(){}};}
    createGain(){return {gain:{setValueAtTime(){},exponentialRampToValueAtTime(value){peaks.push(value);}}};}
  }
  const context=vm.createContext({window:{AudioContext,__UPR_DESKTOP__:desktop},AudioContext,
    localStorage:{getItem(){if(blocked)throw Error('storage unavailable');return stored;}}});
  vm.runInContext(source.slice(start,end),context);
  context.playNotificationSound(1);
  return peaks;
}
assert.equal(audio()[0],0.2375);
assert.equal(audio('100')[0],0.95);
assert.equal(audio('25')[0],0.059375);
assert.deepEqual(audio('0'),[]);
assert.deepEqual(audio('100',{desktop:true}),[]);
for(const invalid of ['', 'invalid', '-1', '101', 'Infinity'])assert.equal(audio(invalid)[0],0.2375);
assert.equal(audio(null,{blocked:true})[0],0.2375);
console.log('PASS: browser volume, louder maximum, mute, defaults, invalid/blocked storage and desktop exclusion.');
