#!/usr/bin/env python3
"""Closed development ZIP. No target release pins, URL, installation or catalog."""
from pathlib import Path
import json,zipfile,hashlib,os,stat,subprocess,tempfile
r=Path(__file__).resolve().parents[1];qa=r/'.qa'
build=json.loads((qa/'fresh-build.json').read_text());binary=Path(build['binary']);assert binary.is_relative_to(qa)
assert hashlib.sha256(binary.read_bytes()).hexdigest()==build['sha256']
assert (Path(r/build['work'])/'source/Cargo.lock').read_bytes()==(r/'Cargo.lock').read_bytes()
for source in (r/'src').glob('*.rs'):assert source.read_bytes()==(r/build['work']/'source/src'/source.name).read_bytes()
m=json.loads((r/'extension.json').read_text());assert m['artifacts']=={}
files={'bin/image-to-ascii':binary.read_bytes(),'LICENSE':(r/'LICENSE').read_bytes(),'README.md':(r/'README.md').read_bytes(),'NOTICES.md':(r/'NOTICES.md').read_bytes(),'qa/dependency-notices.json':(r/'qa/dependency-notices.json').read_bytes()}
files.update({str(p.relative_to(r)):p.read_bytes()for p in (r/'resources').rglob('*')if p.is_file()})
m['files']={n:hashlib.sha256(b).hexdigest()for n,b in sorted(files.items())};files['extension.json']=(json.dumps(m,indent=2)+'\n').encode()
zip_path=r/'image-to-ascii-aarch64-apple-darwin-dev.zip'
with zipfile.ZipFile(zip_path,'w',compression=zipfile.ZIP_DEFLATED)as z:
 for name,data in sorted(files.items()):
  entry=zipfile.ZipInfo(name,date_time=(2026,10,26,0,0,0));entry.create_system=3;entry.external_attr=((stat.S_IFREG|(0o755 if name.startswith('bin/')else 0o644))<<16);entry.compress_type=zipfile.ZIP_DEFLATED;z.writestr(entry,data)
extracted=Path(tempfile.mkdtemp(prefix='extracted-',dir=qa))
with zipfile.ZipFile(zip_path)as z:
 assert sorted(z.namelist())==sorted(files)
 for item in z.infolist():
  p=Path(item.filename);assert not p.is_absolute()and '..'not in p.parts and stat.S_ISREG(item.external_attr>>16)
  out=extracted/p;out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(z.read(item));out.chmod((item.external_attr>>16)&0o777)
actual={str(p.relative_to(extracted)):hashlib.sha256(p.read_bytes()).hexdigest()for p in extracted.rglob('*')if p.is_file()and p.name!='extension.json'}
assert actual==m['files'];assert (extracted/'bin/image-to-ascii').read_bytes()==binary.read_bytes()
result=subprocess.run(['python3',str(r/'qa/process.py'),str(extracted/'bin/image-to-ascii')],cwd=r,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);print(result.stdout);assert result.returncode==0
report={'zip':zip_path.name,'sha256':hashlib.sha256(zip_path.read_bytes()).hexdigest(),'bytes':zip_path.stat().st_size,'members':len(files),'closedInventory':True,'releasePins':{},'extracted':str(extracted.relative_to(r)),'executableSha256':build['sha256'],'processSmoke':json.loads(result.stdout.strip().splitlines()[-1]),'hostInstallNotRun':True}
(qa/'package.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items()if k!='processSmoke'}))
