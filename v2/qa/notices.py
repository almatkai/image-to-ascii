#!/usr/bin/env python3
"""Copy authentic locked dependency notices from this clone's Cargo cache."""
from pathlib import Path
import subprocess,json,os,hashlib,shutil
r=Path(__file__).resolve().parents[1]
env=dict(os.environ,CARGO_HOME=str(r/'.qa/cargo'),TMPDIR=str(r/'.qa/tmp'))
m=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1','--manifest-path',str(r/'Cargo.toml')],env=env))
rows=[]
for package in sorted(m['packages'],key=lambda p:(p['name'],p['version'])):
 if package['name']=='image-to-ascii-native':continue
 source=Path(package['manifest_path']).parent
 assert source.is_relative_to(r/'.qa/cargo')
 files=[]
 for p in sorted(source.iterdir()):
  if p.is_file() and p.name.upper().startswith(('LICENSE','LICENCE','COPYING','NOTICE','UNLICENSE')):
   out=r/'resources/licenses'/f"{package['name']}-{package['version']}"/p.name;out.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,out)
   files.append({'path':str(out.relative_to(r)),'sha256':hashlib.sha256(out.read_bytes()).hexdigest()})
 rows.append({'name':package['name'],'version':package['version'],'license':package['license'],'source':package['source'],'notices':files})
# Preserve superseded generated notice versions in ignored QA evidence rather
# than shipping files for a dependency no longer present in the lock.
current={f"{x['name']}-{x['version']}"for x in rows}
for directory in (r/'resources/licenses').iterdir():
 if directory.is_dir() and directory.name not in current:
  import uuid
  destination=r/'.qa/superseded-notices'/(directory.name+'-'+uuid.uuid4().hex[:8]);destination.parent.mkdir(parents=True,exist_ok=True);shutil.move(str(directory),str(destination))
(r/'qa/dependency-notices.json').write_text(json.dumps(rows,indent=2)+'\n')
print(json.dumps({'lockedDependencies':len(rows),'authenticNoticeFiles':sum(len(x['notices'])for x in rows),'withoutSeparateNotice':[x['name']for x in rows if not x['notices']]}))
