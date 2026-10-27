// Actual pinned Form/Detail/Markdown/surface/view functions; real React+JSDOM,
// inert native/clipboard/dialog/timer producers. No browser or GUI is opened.
const path=require('node:path'),fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..'),deps=path.join(root,'.qa/renderer/node_modules');
const React=require(path.join(deps,'react')),ReactDOM=require(path.join(deps,'react-dom/client')),Test=require(path.join(deps,'react-dom/test-utils'));
const {JSDOM,ResourceLoader}=require(path.join(deps,'jsdom'));
let fetches=0;class Denied extends ResourceLoader{fetch(){fetches++;throw Error('QA network denied');}}
const dom=new JSDOM('<!DOCTYPE html><div id="app"></div>',{url:'https://inert.invalid',resources:new Denied()});
const win=dom.window;Object.assign(global,{window:win,document:win.document,HTMLElement:win.HTMLElement,HTMLTextAreaElement:win.HTMLTextAreaElement,Element:win.Element,IS_REACT_ACT_ENVIRONMENT:true});
const ts=require(path.join(root,'.qa/v1/node_modules/typescript'));
let clipboard=0,dialogs=0,blur=[],selection=null,pending=[],captured,refresh;
const timers=new Map();let timer=0;win.setTimeout=(fn)=>{timers.set(++timer,fn);return timer;};win.clearTimeout=id=>timers.delete(id);
win.tezbar=new Proxy({setSuppressBlurHide:async x=>{blur.push(x);},pickExtensionFile:async()=>{dialogs++;return selection;},extensionInvokeAction:p=>new Promise(resolve=>pending.push({p,resolve})),extensionDisposeSession:async()=>{},clipboardWriteText:async()=>{clipboard++;throw Error('clipboard denied');}}, {get(o,k){if(k in o)return o[k];if(k==='onExtensionSessionInvalidated'||k==='onWindowVisibility')return ()=>()=>{};throw Error('unexpected native producer '+String(k));}});
let app;
async function mount(element){if(app)await React.act(async()=>app.unmount());document.getElementById('app').replaceChildren();app=ReactDOM.createRoot(document.getElementById('app'));await React.act(async()=>app.render(element));}
async function rerender(element){await React.act(async()=>app.render(element));}
const simple=({children})=>React.createElement('span',null,children),primitives=new Proxy({cx:(...x)=>x.filter(Boolean).join(' ')},{get(o,k){return o[k]||simple;}});
function makeLoader(base,{viewCapture=false}={}){
 const cache={};function load(rel){if(cache[rel])return cache[rel];const source=fs.readFileSync(path.join(base,rel),'utf8'),exports={};
 const req=name=>{if(name==='react')return React;if(name==='react/jsx-runtime')return require(path.join(deps,'react/jsx-runtime'));if(name==='marked')return require(path.join(deps,'marked'));if(name==='marked-highlight')return require(path.join(deps,'marked-highlight'));if(name==='highlight.js/lib/common')return require(path.join(deps,'highlight.js/lib/common'));if(name==='dompurify')return require(path.join(deps,'dompurify'))(win);
 if(name.endsWith('/primitives')||name==='./ui/primitives')return primitives;
 if(name.endsWith('/colorConverter'))return {buildColorFormatRows(){throw Error('unexpected color helper');}};
 if(name==='./extensionSessionRefresh')return {startSessionRefresh:options=>{refresh=options.refresh;return ()=>{};}};
 if(name==='./src/raycast-api'&&viewCapture)return {ExtensionRuntimeSurface:props=>{captured=props;return React.createElement('div',null,props.root.props?.label||props.title);}};
 if(name==='./action-runtime')return {Action:{},ActionPanel:{},ActionPanelOverlay:()=>null};
 if(name==='./action-runtime-registry')return {ActionRegistryContext:React.createContext({})};
 if(name==='./hooks')return {NavigationContext:React.createContext({}),setRuntimeContext(){}};
 if(name==='./grid-runtime'||name==='./list-runtime')return {[name==='./grid-runtime'?'GridRuntime':'ListRuntime']:()=>{throw Error('unexpected grid/list');}};
 if(name.startsWith('.')){let dest=path.posix.normalize(path.posix.join(path.posix.dirname(rel),name));if(!/\.tsx?$/.test(dest))dest+='.tsx';if(fs.existsSync(path.join(base,dest)))return load(dest);const original=path.join(root,'qa/fixtures/host',dest);if(fs.existsSync(original))return makeLoader(path.join(root,'qa/fixtures/host')).load(dest);}
 throw Error('unexpected module '+name+' from '+rel);};
 const context=vm.createContext({exports,require:req,window:win,document:win.document,navigator:win.navigator,DOMParser:win.DOMParser,Element:win.Element,HTMLTextAreaElement:win.HTMLTextAreaElement,console,URL,Promise,Set,Map},{codeGeneration:{strings:false,wasm:false}});
 new vm.Script(ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,target:ts.ScriptTarget.ES2022,esModuleInterop:true}}).outputText,{filename:rel}).runInContext(context,{timeout:1000});return cache[rel]=exports;}
 return {load};
}
const formRoot=(type='base64')=>({type:'Form',props:{formToken:'1',formRevision:0},children:type==='base64'?[{type:'Form.TextArea',props:{id:'base64',title:'Base64',value:'',actionId:'f:1:change'}},{type:'Form.Dropdown',props:{id:'outputFormat',title:'Output Format',value:'png',actionId:'f:1:change',data:[{title:'PNG',value:'png'},{title:'JPEG',value:'jpeg'}]}}]:[{type:'Form.FilePicker',props:{id:'filePath',title:'Select Image',value:[],actionId:'f:1:change',nativeAbsolutePath:true,allowMultipleSelection:false}}]});
const h=React.createElement;
const props={title:'test',onBack(){},onOpenActions(){},onInvokeAction(){}};
let assertions=0;
(async()=>{for(const patched of [false,true]){
 const base=path.join(root,patched?'.qa/host-patched':'qa/fixtures/host'),load=makeLoader(base).load;
 const {FormRuntime}=load('src/renderer/src/raycast-api/form-runtime.tsx');let changes=[],submits=[];let fr=formRoot();
 const element=()=>h(FormRuntime,{...props,root:fr,onChangeField:(...args)=>changes.push(args),onSubmitForm:v=>submits.push(v)});
 await mount(element());const ta=document.querySelector('textarea');
 await React.act(async()=>{Test.Simulate.change(ta,{target:{value:'AAAA'}});Test.Simulate.change(ta,{target:{value:'BBBB'}});Test.Simulate.change(ta,{target:{value:'AAAA'}});});
 assert.equal(changes.length,3);assertions++;
 if(patched){assert.deepEqual(JSON.parse(JSON.stringify(changes[2][2])),{base64:'AAAA',outputFormat:'png',__formToken:'1',__formRevision:3});assertions++;}
 else {assert.equal(changes[2][2],undefined);assertions++;}
 await React.act(async()=>Test.Simulate.submit(document.querySelector('form')));
 assert.equal(submits[0].base64,'AAAA');assertions++;
 if(patched){fr={...fr,props:{formToken:'1',formRevision:1},children:fr.children.map(n=>({...n,props:{...n.props,value:n.props.id==='base64'?'BBBB':'png'}}))};await rerender(element());assert.equal(document.querySelector('textarea').value,'AAAA');assertions++;}
 // Pinned native FilePicker before: HTML file lacking .path becomes a basename.
 changes=[];selection=path.join(root,'.qa/selected/a.png');await mount(h(FormRuntime,{...props,root:formRoot('file'),onChangeField:(...x)=>changes.push(x),onSubmitForm(){}}));
 if(!patched){await React.act(async()=>Test.Simulate.change(document.querySelector('input[type=file]'),{target:{files:[{name:'a.png'}]}}));assert.equal(changes[0][1][0],'a.png');assertions++;}
 else {await React.act(async()=>Test.Simulate.click([...document.querySelectorAll('button')].find(x=>x.textContent==='Choose File')));assert.equal(changes[0][1][0],selection);assert.equal(changes[0][2].__formRevision,1);assertions+=2;selection=null;await React.act(async()=>Test.Simulate.click([...document.querySelectorAll('button')].find(x=>x.textContent==='Choose File')));assert.equal(changes.length,1);assertions++;await React.act(async()=>Test.Simulate.click([...document.querySelectorAll('button')].find(x=>x.textContent==='Clear')));assert.deepEqual(JSON.parse(JSON.stringify(changes[1][1])),[]);assert.equal(changes[1][2].__formRevision,2);assertions+=2;}
 // Actual surface -> Detail click. Before forwards MouseEvent as action ID.
 const {ExtensionRuntimeSurface}=load('src/renderer/src/raycast-api/index.tsx');let invokes=[];
 const surface={title:'Image',extensionId:'com.tezbar.native.image-base64',commandName:'base64-to-image',sessionId:'s',root:{type:'Detail',props:{markdown:'Preview unavailable for this payload. Copy and Download preserve the original bytes.'},metadata:{type:'Detail.Metadata',children:[{type:'Detail.Metadata.Label',props:{title:'File Name',text:'](<img src=https://invalid onerror=evil>)'}}]}},actions:[{id:'d:1:another',title:'Convert Another'}],onBack(){},onLoadMore(){},onSearchTextChanged(){},onInvokeAction:(...x)=>invokes.push(x)};
 await mount(h(ExtensionRuntimeSurface,surface));await React.act(async()=>Test.Simulate.click([...document.querySelectorAll('button')].find(x=>x.textContent==='Enter')));
 assert.equal(typeof invokes[0][0],patched?'string':'object');if(patched)assert.equal(invokes[0][0],'d:1:another');assertions+=2;
 assert.equal(document.querySelectorAll('img,script,a,iframe,svg').length,0);assert.ok(document.body.textContent.includes('<img src=https://invalid'));assertions+=2;
 // Both the original pinned Markdown and proposed literal Detail must render
 // the actual malicious service result inertly, without resource or HTML nodes.
 for(const view of JSON.parse(fs.readFileSync(path.join(root,'.qa/pipes/native-views.json'),'utf8'))) {
 await mount(h(ExtensionRuntimeSurface,{...surface,...view}));
 assert.equal(document.querySelectorAll('img,script,iframe,a,svg').length,0);assertions++;
 const code=document.querySelector('pre code');assert.ok(code);assertions++;
 if(patched){assert.equal(code.textContent,view.root.props.literalText);assert.equal(document.querySelectorAll('.tezbar-md-code-copy').length,0);assertions+=2;}
 else {assert.ok(code.textContent.includes(view.root.props.literalText));assertions++;}
 }
 // Actual four-field native form: one controlled image, both dimensions and
 // checkbox snapshots, including a pending submit and late native picker.
 if(patched){
 const owned={type:'Form',props:{formToken:'T',formRevision:0},children:[
 {type:'Form.FilePicker',props:{id:'image',title:'Select an Image',value:[],nativeAbsolutePath:true,actionId:'change'}},
 {type:'Form.TextField',props:{id:'width',title:'Width',value:'80',actionId:'change'}},
 {type:'Form.TextField',props:{id:'height',title:'Height',value:'',actionId:'change'}},
 {type:'Form.Checkbox',props:{id:'complex',title:'Complex',value:false,actionId:'change'}}]};
 let received=[],finish;
 await mount(h(ExtensionRuntimeSurface,{...surface,root:owned,actions:[{id:'submit',title:'Convert',kind:'submit-form'}],onInvokeAction:(id,values)=>{received.push({id,values});if(id==='submit')return new Promise(r=>finish=r);}}));
 const inputs=[...document.querySelectorAll('input')];
 await React.act(async()=>{Test.Simulate.change(inputs[0],{target:{value:'0x10'}});Test.Simulate.change(inputs[1],{target:{value:'1.5'}});Test.Simulate.change(inputs[2],{target:{checked:true}});});
 assert.deepEqual(JSON.parse(JSON.stringify(received.at(-1).values)),{image:[],width:'0x10',height:'1.5',complex:true,__formToken:'T',__formRevision:3});assertions++;
 await React.act(async()=>Test.Simulate.submit(document.querySelector('form')));
 assert.equal(document.querySelector('fieldset').disabled,true);assertions++;
 const before=received.length;await React.act(async()=>{Test.Simulate.change(inputs[0],{target:{value:'999'}});Test.Simulate.submit(document.querySelector('form'));});
 assert.equal(received.length,before);assert.equal(inputs[0].value,'0x10');assertions+=2;
 await React.act(async()=>finish());assert.equal(document.querySelector('fieldset').disabled,false);assertions++;
 // Picker completion after a draft revision changes is discarded.
 let pickerResolve;const pickerOriginal=win.tezbar.pickExtensionFile;win.tezbar.pickExtensionFile=()=>new Promise(r=>pickerResolve=r);
 await React.act(async()=>Test.Simulate.click([...document.querySelectorAll('button')].find(x=>x.textContent==='Choose File')));
 await React.act(async()=>Test.Simulate.change(inputs[1],{target:{value:'2'}}));
 const beforePicker=received.length;await React.act(async()=>pickerResolve('/owned/late.png'));assert.equal(received.length,beforePicker);assertions++;
 win.tezbar.pickExtensionFile=pickerOriginal;
 }
 // Actual generic view async callback, with only its child surface intercepted.
 const View=makeLoader(base,{viewCapture:true}).load('src/renderer/ExtensionRuntimeView.tsx').default;
 const initial={ok:true,mode:'view',refreshMs:null,sessionId:'s',generation:'g',extensionId:'id',commandName:'cmd',title:'test',root:{type:'Form',props:{label:'initial'}},actions:[]};pending=[];
 await mount(h(View,{initial,onBack(){}}));let a,b;await React.act(async()=>{a=captured.onInvokeAction('A');b=captured.onInvokeAction('B');});
 await React.act(async()=>{pending[1].resolve({...initial,root:{type:'Form',props:{label:'B'}}});await b;});
 await React.act(async()=>{pending[0].resolve({...initial,root:{type:'Form',props:{label:'A'}}});await a;});
 assert.equal(captured.root.props.label,patched?'B':'A');assertions++;
 if(patched){let c;await React.act(async()=>{c=captured.onInvokeAction('stale');});await React.act(async()=>{pending[2].resolve({ok:true,mode:'unchanged'});await c;});assert.equal(captured.root.props.label,'B');assertions++;}
 // The native unknown ACK message must suppress the actual host's "Copied"
 // action-title fallback, on both pinned/proposed view implementations.
 const unknown=JSON.parse(fs.readFileSync(path.join(root,'.qa/pipes/unknown-copy-view.json'),'utf8'));
 const detail={...initial,...unknown,ok:true,mode:'view',sessionId:'s',generation:'g',actions:[{id:'copy-test',title:'Copy Image Base64',kind:'action'}]};pending=[];
 await mount(h(View,{initial:detail,onBack(){}}));let copy;
 await React.act(async()=>{copy=captured.onInvokeAction('copy-test');});
 await React.act(async()=>{pending[0].resolve({...detail,...unknown,ok:true,mode:'view',sessionId:'s',generation:'g'});await copy;});
 assert.equal(captured.actionNotice.tone,'error');assert.ok(captured.actionNotice.message.startsWith('Could not confirm copy'));assertions+=2;

 }
 await React.act(async()=>app.unmount());assert.equal(fetches,0);assert.equal(clipboard,0);console.log(JSON.stringify({assertions:assertions+2,beforeAfter:true,realGui:false,actualPinnedComponents:['FormRuntime','DetailRuntime','Markdown','ExtensionRuntimeSurface','ExtensionRuntimeView'],nativeCallsInert:{dialogs,clipboard,fetches},osFileDialogNotRun:true}));
})().catch(e=>{console.error(e);process.exitCode=1;});
