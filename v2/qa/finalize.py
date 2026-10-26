#!/usr/bin/env python3
"""Audit only this clone, optional exact host Git objects; never stage or publish."""
from pathlib import Path
import subprocess,json,hashlib,datetime,re,sys
r=Path(__file__).resolve().parents[1];root=r.parent;q=r/'.qa'
HEAD='f018cd7e93988a0c7e33d4afba54ea9d0ab6f49e';HOST='ea100cee3f69711de12485ba061950c5fc65258f';SDK='e955911f0bf5b558f9d3eb5183df0adc95c22af2';UP='6734d85992290066fd31a7f5bee3cca3d3545a1a';REF='51e302cacdaeea11038704ef2825893cb714e1f3'
sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=root)
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
assert git('rev-parse','--show-toplevel').decode().strip()==str(root)
assert git('rev-parse','HEAD').decode().strip()==HEAD
assert git('branch','--show-current').decode().strip()=='feat/native-v2-rust'
assert git('diff','--cached','--name-only')==b''
assert git('diff','--name-only').decode().splitlines()==['README.md']
baseline={}
for name in git('ls-tree','-r','--name-only',HEAD).decode().splitlines():
 old=git('show',HEAD+':'+name);baseline[name]=sha(old)
 if name=='README.md':assert(root/name).read_bytes()==old+b'\nNative Rust development port: [v2/README.md](v2/README.md).\n'
 else:assert(root/name).read_bytes()==old,name
 (q/'v1'/name).exists()and None
 if(q/'v1'/name).exists():assert(q/'v1'/name).read_bytes()==old,name
# During initial --prepare V1 README is exact base, never the added discovery link.
dump(r/'qa/v1-baseline.json',{'head':HEAD,'files':baseline})
icon=(r/'resources/extension-icon.png').read_bytes();assert icon==(root/'assets/extension-icon.png').read_bytes();assert sha(icon)=='883d6bb2858722b70c6d72dc47b81dc52790cb7bab256138347e0f1c7e54660e'
fixture_hashes={str(p.relative_to(r)):sha(p.read_bytes())for p in sorted((r/'qa/fixtures').rglob('*'))if p.is_file()}
host_repo=root.parents[1]/'Tezbar'
if host_repo.exists():
 for p in sorted((r/'qa/fixtures/host').rglob('*')):
  if p.is_file():assert p.read_bytes()==subprocess.check_output(['git','show',HOST+':'+str(p.relative_to(r/'qa/fixtures/host'))],cwd=host_repo),p
reuse={}
for p in sorted((q/'reference/v2').rglob('*')):
 if p.is_file():reuse[str(p.relative_to(q/'reference'))]={'sha256':sha(p.read_bytes()),'url':f'https://raw.githubusercontent.com/almatkai/image-base64/{REF}/'+str(p.relative_to(q/'reference'))}
pins={'own':{'repository':'https://github.com/almatkai/image-to-ascii','sourceSha':HEAD},'upstream':{'repository':'https://github.com/raycast/extensions','commit':UP,'path':'extensions/image-to-ascii'},'icon':{'url':f'https://raw.githubusercontent.com/raycast/extensions/{UP}/extensions/image-to-ascii/assets/extension-icon.png','sha256':sha(icon),'bytes':len(icon),'matchesOwnOriginal':True},'host':{'commit':HOST,'fixtures':'qa/fixtures/host','verifiedAgainstExactGitShow':host_repo.exists()},'sdk':{'crate':'tezbar-extension-sdk','version':'0.1.1','repository':'https://github.com/almatkai/tezbar-extension-sdk-rust.git','rev':SDK,'documents':['SDK_API.md','PROTOCOL.md','HOST_INTEGRATION.md']},'reusedReference':{'repository':'https://github.com/almatkai/image-base64','commit':REF,'license':'qa/fixtures/IMAGE-BASE64-LICENSE','downloadedFiles':reuse},'fixtureSha256':fixture_hashes}
dump(r/'qa/pins.json',pins)
if '--pins-only' in sys.argv:
 print(json.dumps({'fixtureFiles':len(fixture_hashes),'hostExactGitShow':host_repo.exists(),'v1Files':len(baseline)}));sys.exit(0)
verification=json.loads((q/'verification.json').read_text());assert all(t['exit']==0 or(t['name']in['windows-gnu-check','windows-clang-check']and t['exit']==101)for t in verification['tests'])
for name in ['tests-stable','tests-msrv']:
 log=(q/(name+'.log')).read_text();assert '9 passed; 0 failed; 1 ignored'in log and '2 passed; 0 failed'in log and '3 passed; 0 failed'in log
build=json.loads((q/'fresh-build.json').read_text());package=json.loads((q/'package.json').read_text())
assert package['executableSha256']==build['sha256']and sha((r/package['zip']).read_bytes())==package['sha256'];assert package['processSmoke']['checks']==7 and package['processSmoke']['externalNetworkDenied']
for p in (r/'src').glob('*.rs'):assert p.read_bytes()==(r/build['work']/'source/src'/p.name).read_bytes(),p
for name in ['Cargo.toml','Cargo.lock','extension.json']:assert(r/name).read_bytes()==(r/build['work']/'source'/name).read_bytes(),name
oracle=json.loads((r/'qa/oracle-cases.json').read_text());assert oracle['sourceSHA256']==baseline['src/convert-image-to-ascii.tsx']
def last_json(name):return json.loads((q/(name+'.log')).read_text().strip().splitlines()[-1])
assert last_json('process-worker')['checks']==218
assert last_json('renderer')['nativeCallsInert']['clipboard']==0 and last_json('renderer')['nativeCallsInert']['fetches']==0
assert last_json('schema')['cases']==16
subprocess.run(['git','apply','--check','--directory=v2/qa/fixtures/host','v2/integration/host-prerequisites.patch'],cwd=root,check=True)
subprocess.run(['git','diff','--check'],cwd=root,check=True)
manifest=json.loads((r/'extension.json').read_text());assert manifest['artifacts']=={};assert manifest['capabilities']==['selection.read','clipboard.write']
xerj={'prefix':'ax-'+sha(str(root).encode())[:12],'state':'.qa/xerj','initial':{'exit':3,'indexedFiles':10,'codeFilesIndexed':4,'codeFiles':4,'records':56,'nonCodeJunkOrSkipped':3},'finalLog':'.qa/xerj-final.log','final':json.loads((q/'xerj-final.json').read_text())}
evidence={'recordedAtUtc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'sourceHead':HEAD,'branch':'feat/native-v2-rust','iconSha256':sha(icon),'verification':verification,'coverage':{'rustPerToolchain':{'passed':14,'explicitHelperIgnoredButSeparatelyRun':1},'sourceOracle':{k:oracle[k]for k in ['assertions','lockedAxios','lockedFormData','closedVM','realProducerCalls']},'sourceNumericCases':len(oracle['numeric']),'sourceResponseCases':len(oracle['responses']),'process':last_json('process-worker'),'renderer':last_json('renderer'),'schema':last_json('schema'),'v1Tests':1,'v1Typecheck':'passed'},'freshRelease':build,'devPackage':package,'xerj':xerj,'limits':{'fileBytes':8000000,'resultBytes':1000000,'dimensionMaximum':10000,'fieldBytes':256,'pathBytes':4096,'filenameBytes':512,'sessions':16,'connectSeconds':8,'requestSeconds':30},'handoff':{'uncommitted':True,'stagedDiffEmpty':True,'v1UnchangedExceptReadmeDiscoveryLink':True,'hostPatch':'UNAPPLIED; check against exact pinned fixtures passed','noStageCommitPushPrReleaseInstallLedgerUpdateOrCloneDeletion':True},'acceptance':{'macOSNativeGuiFinderClipboard':'not-run','WindowsNativeGui':'not-run','fullHost':'not-run','liveServiceUpload':'not-run; intentionally denied'},'windowsToolFailures':{'gnu':'ring C compilation: missing x86_64-w64-mingw32-gcc','existingClangAr':'ring C compilation: Windows assert.h header missing','msvc':'not installed or attempted'},'deviations':['explicit path/file/filename/resource bounds','positive finite dimensions <=10000; JavaScript Number/Number.toString otherwise retained','strict UTF-8, non-string JSON and unpaired-surrogate exclusion','literal React-escaped Detail output and safe fence; exact extension-owned Copy ACK','verified TLS; no proxies/redirects/credentials/automatic retries','in-memory action/checkpoint state; no durable/exactly-once claim']}
def visible():
 names=sorted(set(n for n in git('ls-files','--cached','--others','--exclude-standard','-z').decode().split('\0')if n));assert all(not({'.qa','target','node_modules'}&set(Path(n).parts))and not n.endswith('.zip')for n in names);return names
names=visible();evidence['sourceFilesSha256']={n:sha((root/n).read_bytes())for n in names if n not in['v2/qa/evidence.json','v2/INDEX.sha256']};dump(r/'qa/evidence.json',evidence)
index=[f'{sha((root/n).read_bytes())}  {n}'for n in visible()if n!='v2/INDEX.sha256'];(r/'INDEX.sha256').write_text('\n'.join(index)+'\n')
assert git('diff','--cached','--name-only')==b''
print(json.dumps({'sourceSha':HEAD,'branch':'feat/native-v2-rust','indexEmpty':True,'iconSha256':sha(icon),'inventoryFiles':len(index),'zipSha256':package['sha256'],'executableSha256':build['sha256']}))
