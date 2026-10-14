// No browser or additional packages needed: exercise the route helpers used by the UI.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname, '../../web/app.js'), 'utf8');
const start = source.indexOf('const heardExpanded=');
const end = source.indexOf('function findConnectRoute', start);
assert(start >= 0 && end > start);
const context = vm.createContext({state:{snapshot:{heard:[]}}});
vm.runInContext(source.slice(start, end), context);
const route = via => context.reverseHeardRoute({via});
assert.equal(route('DIGA* DIGB* DIGC* DIGD* UNUSED'), 'DIGD DIGC DIGB DIGA');
assert.equal(route('DIGA DIGB'), '');
assert.equal(route(''), '');
assert.equal(route('DIGA* DIGB'), 'DIGA');
assert.equal(route('DIGA-1* DIGB-7*'), 'DIGB-7 DIGA-1');
assert.equal(context.heardUsedDigis({via:'DIGA* DIGB* UNUSED'}).at(-1), 'DIGB');
context.state.snapshot.heard = [
    {call:'PEER-1',interface:1,via:'DIGA*'},
    {call:'PEER-1',interface:0,via:'DIGB*'},
    {call:'PEER-2',interface:0,via:''},
];
assert.equal(context.heardForCall(' peer-1 ', 0).via, 'DIGB*');
assert.equal(context.heardForCall('PEER-1', 1).via, 'DIGA*');
assert.equal(context.heardForCall('PEER-1', 2), undefined);
assert.equal(context.heardForCall('PEER-2', 0).via, '');
assert.notEqual(context.heardEntryKey({call:'PEER',interface:0,port:0}), context.heardEntryKey({call:'PEER',interface:0,port:1}));
console.log('PASS: used digipeaters only, reverse order, SSIDs, direct route, last digi, interface and port identity.');
