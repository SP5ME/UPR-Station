const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const vm=require('node:vm');
const source=fs.readFileSync(path.join(__dirname,'../web/app.js'),'utf8');
const start=source.indexOf('function syncProductIdentity('),end=source.indexOf('\nsyncProductIdentity();',start);
assert(start>=0&&end>start);
function check(desktop,runtime,product){
  const family={},variant={},brand={setAttribute(key,value){this[key]=value;}},input={value:'Operator'};
  const version={nodeType:1,textContent:'0.1.3'},dot={nodeType:1};
  const build={childNodes:[{nodeType:3,textContent:'Old'},version]},footer={childNodes:[dot,{nodeType:3,textContent:'Old'}]};
  const elements={'.brand-name':family,'.brand-project':variant,'.sidebar .brand':brand,'.sidebar .build':build,'footer > span:first-child':footer};
  const document={querySelector:selector=>elements[selector]};
  const context=vm.createContext({window:{__UPR_DESKTOP__:desktop},state:{webDetails:{runtime}},document,$:()=>input});
  vm.runInContext(source.slice(start,end),context);context.syncProductIdentity();
  assert.equal(document.title,'UPR '+product);assert.equal(family.textContent,'UPR');
  assert.equal(variant.textContent,product);assert.equal(brand['aria-label'],'UPR '+product);
  assert.equal(build.childNodes[0].textContent,'UPR '+product+' ');
  assert.equal(footer.childNodes[1].textContent,' UPR '+product.toUpperCase());
  assert.equal(input.value,'Operator');assert.equal(input.placeholder,'UPR '+product);
  assert.equal(version.textContent,'0.1.3');assert.equal(footer.childNodes[0],dot);
}
check(true,'server','Terminal');check(false,'server','Station');check(false,'desktop','Terminal');
console.log('PASS: product titles, sidebar, footer and accessible names; user data and version preserved.');
