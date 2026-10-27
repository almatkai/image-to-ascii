// Own untouched TSX + exact-lock Axios node distribution and form-data in a closed VM.
// Every external producer is replaced BEFORE the module/effects/callbacks run.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'..'),nm=path.join(root,'.qa/v1/node_modules'),ts=require(path.join(nm,'typescript'));
const source=fs.readFileSync(path.join(root,'../src/convert-image-to-ascii.tsx'),'utf8');
const outputs=[],numeric=[],selectionCases=[];let count=0;
function eq(a,b){assert.deepEqual(JSON.parse(JSON.stringify(a)),JSON.parse(JSON.stringify(b)));count++;}
function sandbox({selection=[],selectionError=false,body='ASCII\n',postError=false,streamError=false,exists=true,directory=false}={}){
 const calls={selection:0,reads:[],stat:[],multipart:[],toasts:[],push:[],clipboard:[],denied:[]};
 const deny=name=>(...args)=>{calls.denied.push(name);throw Error('DENIED '+name);};
 const cache={};let state=[],cursor=0,effects=[],tree,axios;
 const synthetic=new Proxy({existsSync:p=>{calls.stat.push(['exists',p]);return exists;},lstatSync:p=>{calls.stat.push(['lstat',p]);return {isDirectory:()=>directory};},createReadStream:p=>{calls.reads.push(p);const s=new (require('node:stream').PassThrough)();s.path=p;s.fd=17;queueMicrotask(()=>streamError?s.destroy(Error('inert stream failure')):s.end(Buffer.from([0,1,2,255])));return s;},stat:(p,cb)=>{calls.stat.push(['async',p]);queueMicrotask(()=>cb(null,{size:4}));}},{get(o,k){if(k==='__esModule')return false;return k in o?o[k]:deny('fs.'+String(k));}});
 const deniedModule=name=>new Proxy({},{get:(_,k)=>deny(name+'.'+String(k))});
 const processMock=new Proxy({[Symbol.toStringTag]:'process',nextTick:(fn,...a)=>queueMicrotask(()=>fn(...a)),env:Object.freeze({}),version:'v22.0.0',versions:Object.freeze({node:'22.0.0'}),platform:'darwin'},{get(o,k){return k in o?o[k]:deny('process.'+String(k));}});
 const context=vm.createContext({Buffer,URL,URLSearchParams,TextEncoder,TextDecoder,Promise,queueMicrotask,setImmediate:queueMicrotask,setTimeout:deny('timer'),clearTimeout(){},process:processMock,console:{error(...a){calls.error=a.map(x=>String(x));},log:deny('console.log')}},{codeGeneration:{strings:false,wasm:false}});
 function moduleLoad(file){file=path.resolve(file);assert.ok(file.startsWith(nm+path.sep));if(cache[file])return cache[file].exports;
  const mod={exports:{}};cache[file]=mod;if(file.endsWith('.json')){mod.exports=JSON.parse(fs.readFileSync(file));return mod.exports;}
  const req=name=>load(name,file);const code=fs.readFileSync(file,'utf8');const fn=new vm.Script('(function(require,module,exports,__filename,__dirname){'+code+'\n})',{filename:file}).runInContext(context,{timeout:2000});fn(req,mod,mod.exports,file,path.dirname(file));return mod.exports;
 }
 function load(name,from=path.join(nm,'entry.js')){
  if(name==='fs'||name==='node:fs')return synthetic;
  if(['http','https','http2','net','tls','os','child_process','worker_threads','dns'].includes(name))return deniedModule(name);
  if(name==='crypto')return {randomBytes:n=>Buffer.alloc(n,17),randomFillSync:b=>b.fill(17)};
  if(['stream','events','util','path','url','buffer','assert','zlib'].includes(name))return require('node:'+name);
  if(name==='follow-redirects')return {http:deniedModule('redirect-http'),https:deniedModule('redirect-https')};
  if(name==='@raycast/api')return raycast;
  if(name==='react')return {useState:init=>{const i=cursor++;if(!(i in state))state[i]=init;return [state[i],v=>state[i]=v];},useEffect:fn=>{if(!effects.length&&state.length<=2)effects.push(fn);}};
  if(name==='react/jsx-runtime')return {jsx:(type,props)=>({type,props}),jsxs:(type,props)=>({type,props})};
  if(name==='axios')return axios;
  const resolved=require.resolve(name,{paths:[path.dirname(from)]});return moduleLoad(resolved);
 }
 const raycast={ActionPanel:'ActionPanel',Form:Object.assign('Form',{FilePicker:'FilePicker',TextField:'TextField',Description:'Description',Checkbox:'Checkbox'}),Detail:'Detail',Action:Object.assign(function(){},{SubmitForm:'SubmitForm'}),Toast:{Style:{Failure:'failure',Success:'success'}},showToast:v=>{calls.toasts.push(v);return Promise.resolve();},useNavigation:()=>({push:v=>calls.push.push(v)}),Clipboard:{copy:async v=>calls.clipboard.push(v)},getSelectedFinderItems:async()=>{calls.selection++;if(selectionError)throw Error('inert selection failure');return selection;}};
 // Actual locked Axios node build. Its http/http2/redirect producers are denied.
 axios=moduleLoad(path.join(nm,'axios/dist/node/axios.cjs'));
 // Bind only Axios' transport; retain its actual Node HTTP adapter, stream,
 // BOM/UTF-8 decoding, settle and default JSON response transformation.
 axios.defaults.transport={request(options,callback){
  assert.equal(options.hostname,'ascii-image-converter.zeabur.app');assert.equal(options.path,'/image-to-ascii');assert.equal(options.method,'POST');
  const chunks=[];const {Writable,PassThrough}=require('node:stream');
  const req=new Writable({autoDestroy:false,write(chunk,_enc,cb){chunks.push(Buffer.from(chunk));cb();}});
  req.setTimeout=()=>req;
  req.on('finish',()=>{
   calls.multipart.push({headers:options.headers,base64:Buffer.concat(chunks).toString('base64')});
   if(postError){req.destroy(Error('inert post failure'));return;}
   const res=new PassThrough();res.statusCode=200;res.statusMessage='OK';res.headers={'content-type':'text/plain','content-length':String(Buffer.byteLength(body))};res.req=req;
   callback(res);res.end(Buffer.from(body));
  });return req;
 }};
 const exports={};new vm.Script('(function(require,exports){'+ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,target:ts.ScriptTarget.ES2022,esModuleInterop:true}}).outputText+'\n})').runInContext(context)(name=>load(name),exports);
 function render(){cursor=0;tree=exports.default();return tree;}
 render();const initialEffects=effects.slice();effects=[];
 return {calls,context,render,get tree(){return tree;},async launch(){for(const e of initialEffects)e();await new Promise(queueMicrotask);await new Promise(queueMicrotask);return render();},async submit(values){return tree.props.actions.props.children.props.onSubmit(values);}};
}
(async()=>{
 for(const [selection,error,expected]of [[[],false,[]],[[{path:'/owned/PIC.JpEg'}],false,['/owned/PIC.JpEg']],[[{path:'/owned/a.png'},{path:'/owned/b.png'}],false,[]],[[{path:'/owned/a.gif'}],false,[]],[[{}],false,[]],[[],true,[]]]){
  const s=sandbox({selection,selectionError:error});await s.launch();eq(s.tree.props.children[0].props.value,expected);eq(s.calls.selection,1);eq(s.calls.reads,[]);selectionCases.push({selection,error,expected});
 }
 const defaults=sandbox();eq(defaults.tree.props.children[1].props.defaultValue,'80');eq(defaults.tree.props.children[4].props.defaultValue,false);eq(defaults.tree.props.children[0].props.allowMultipleSelection,false);
 const values={image:['/owned/Fake.PNG'],width:'80',height:'',complex:false};
 for(const text of ['', ' ', '80','1.5','1e2','0x20','0o10','0b11','+2','.5','1.','1e-7','1e-6','5e-324','1e-324','-1','0','-0','NaN','Infinity','+Infinity','inf','0x','+0x10','1_000','1e4','10000.1','\ufeff80\u00a0','\u008580','1e400']){
  const s=sandbox();await s.launch();await s.submit({...values,width:text});
  const n=vm.runInContext('Number('+JSON.stringify(text)+')',s.context),trim=vm.runInContext(JSON.stringify(text)+'.trim()',s.context);
  const wire=trim!==''&&!Number.isNaN(n)&&n>0?String(n):null;
  if(s.calls.multipart.length!==(wire===null&&trim!==''?0:1))console.error(text,s.calls);eq(s.calls.multipart.length,wire===null&&trim!==''?0:1);
  const row={text,trim,number:Number.isNaN(n)?'NaN':String(n),sourceAccepted:s.calls.multipart.length===1,wire:trim===''?null:wire,nativeExpected:wire!==null&&(!Number.isFinite(n)||n>10000)?'resource-rejected':(wire===null&&trim!==''?'invalid':wire)};
  numeric.push(row);if(s.calls.multipart.length)outputs.push({...row,multipart:s.calls.multipart[0]});eq(s.calls.denied,[]);
 }
 for(const [opts,v,title]of [[{}, {...values,image:[]},'Please select an image file.'],[{exists:false},{...values,width:'-2'},'Selected file is not valid.'],[{directory:true},values,'Selected file is not valid.'],[{}, {...values,image:['/owned/a.gif'],width:'-2'},'Images must be in one of the following formats: .jpeg, .jpg, .png, .bmp, .webp'],[{}, {...values,width:'NaN',height:'-1'},'Width must be a positive number.'],[{}, {...values,height:'0'},'Height must be a positive number.']]){
  const s=sandbox(opts);await s.launch();await s.submit(v);eq(s.calls.toasts.at(-1).title,title);eq(s.calls.multipart.length,0);eq(s.calls.reads,[]);
 }
 const responses=[];
 for(const body of ['ASCII\n','"quoted\\nASCII"','{"error":"bad"}','null','true','123','1e400','[1,2]','"\\ud800"','"\\ud83d\\ude00"','{broken','', '\ufeff"BOM"', '```\n<img src="https://blocked.invalid">\n![x](https://blocked.invalid/a)\n\u202e']){
  const s=sandbox({body});await s.launch();await s.submit({...values,width:'0x10',height:'1.5',complex:true});eq(s.calls.push.length,1);
  const detail=s.calls.push[0],copy=detail.props.actions.props.children.props.onAction;
  await copy();await copy();eq(s.calls.clipboard.length,2);
  let parsed;try{parsed=JSON.parse(body.replace(/^\ufeff/,''));}catch{parsed=body.replace(/^\ufeff/,'');}
  eq(s.calls.clipboard[0],parsed);eq(s.calls.toasts.at(-1).title,'Ascii image copied to clipboard');eq(s.calls.denied,[]);
  const unpaired=typeof parsed==='string'&&/[\ud800-\udbff](?![\udc00-\udfff])|(?<![\ud800-\udbff])[\udc00-\udfff]/u.test(parsed);
  responses.push({body,result:unpaired?null:s.calls.clipboard[0],sourceMarkdown:unpaired?null:detail.props.markdown,type:typeof s.calls.clipboard[0],...(unpaired?{nativeExpected:'unicode-rejected',utf16:Array.from({length:parsed.length},(_,i)=>parsed.charCodeAt(i))}:{})});
 }
 for(const opts of [{postError:true},{streamError:true}]){
  const s=sandbox(opts);await s.launch();await s.submit(values);eq(s.calls.push,[]);eq(s.calls.toasts.at(-1).title,'Failed to convert image');eq(s.calls.denied,[]);
 }
 // Source controlled picker and navigation keep selected draft; repeated submit is deliberate.
 const s=sandbox({selection:[{path:'/owned/a.webp'}]});await s.launch();s.tree.props.children[0].props.onChange(['/owned/b.BMP']);s.render();eq(s.tree.props.children[0].props.value,['/owned/b.BMP']);
 for(const width of ['80','81','80','bad','80'])await s.submit({...values,image:['/owned/b.BMP'],width});eq(s.calls.multipart.length,4);eq(s.calls.push.length,4);eq(s.render().props.children[0].props.value,['/owned/b.BMP']);eq(s.calls.denied,[]);
 const report={assertions:count,sourceSHA256:crypto.createHash('sha256').update(source).digest('hex'),lockedAxios:require(path.join(nm,'axios/package.json')).version,lockedFormData:require(path.join(nm,'form-data/package.json')).version,numeric,selectionCases,responses,multipartSamples:outputs,closedVM:true,realProducerCalls:0};
 fs.writeFileSync(path.join(root,'qa/oracle-cases.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({assertions:count,numeric:numeric.length,responses:responses.length,selection:selectionCases.length,closedVM:true,realProducerCalls:0}));
})().catch(e=>{console.error(e);process.exitCode=1;});
