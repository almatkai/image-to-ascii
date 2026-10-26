const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const r=path.resolve(__dirname,'..'),Ajv=require(path.join(r,'.qa/renderer/node_modules/ajv/dist/2020'));
const schema=JSON.parse(fs.readFileSync(path.join(r,'qa/fixtures/host/schemas/extension-v3.schema.json'))),m=JSON.parse(fs.readFileSync(path.join(r,'extension.json')));
const ajv=new Ajv({strict:false,validateFormats:false}),validate=ajv.compile(schema);assert(validate(m),JSON.stringify(validate.errors));let cases=1;
for(const [key,value]of [['schemaVersion',2],['protocolVersion',2],['runtime','javascript'],['execution','embedded'],['id','raycast.image-to-ascii'],['replaces',[]],['capabilities',['unknown']],['platforms',['bad']],['commands',[]],['artifacts',{'bad-target':{}}],['files',{'../escape':'0'.repeat(64)}],['extra',true]]){
 const v=structuredClone(m);v[key]=value;assert.equal(validate(v),false,key);cases++;
}
for(const artifact of [{url:'http://invalid',sha256:'0'.repeat(64),executable:'bin/x',executableSha256:'0'.repeat(64)},{url:'https://invalid',sha256:'bad',executable:'bin/x',executableSha256:'0'.repeat(64)},{url:'https://invalid',sha256:'0'.repeat(64),executable:'../x',executableSha256:'0'.repeat(64)}]){const v=structuredClone(m);v.artifacts={'aarch64-apple-darwin':artifact};assert.equal(validate(v),false);cases++;}
assert.deepEqual(m.artifacts,{});assert.deepEqual(m.replaces,['com.tezbar.image-to-ascii','raycast.image-to-ascii']);console.log(JSON.stringify({cases,fullPinnedSchema:true,artifactsEmpty:true}));
