#!/usr/bin/env python3
"""Reproduce confined checks. Never install targets/tools, use OS UI or upload live images."""
from pathlib import Path
import os,sys,subprocess,json,shutil,tempfile
r=Path(__file__).resolve().parents[1];repo=r.parent;q=r/'.qa';q.mkdir(exist_ok=True)
for d in ['home','tmp','cargo','v1','renderer']:(q/d).mkdir(exist_ok=True)
env=dict(os.environ,HOME=str(q/'home'),USERPROFILE=str(q/'home'),TMPDIR=str(q/'tmp'),TEMP=str(q/'tmp'),TMP=str(q/'tmp'),CARGO_HOME=str(q/'cargo'),CARGO_TARGET_DIR=str(q/'target'),RUSTUP_HOME=os.environ.get('RUSTUP_HOME',str(Path.home()/'.rustup')))
results=[]
def run(name,args,extra=None,allowed=(0,)):
 e=dict(env,**(extra or {}));p=subprocess.run(args,cwd=repo,env=e,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);(q/(name+'.log')).write_text(p.stdout)
 results.append({'name':name,'argv':args,'exit':p.returncode,'log':'.qa/'+name+'.log'});print(name,p.returncode,flush=True)
 if p.returncode not in allowed:print(p.stdout[-5000:]);raise RuntimeError(name)
 return p.stdout
if '--prepare'in sys.argv:
 for name in subprocess.check_output(['git','ls-files','-z'],cwd=repo).decode().split('\0'):
  if name and not name.startswith('v2/'):
   p=q/'v1'/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(subprocess.check_output(['git','show','f018cd7e93988a0c7e33d4afba54ea9d0ab6f49e:'+name],cwd=repo))
 for f in ['package.json','package-lock.json']:shutil.copyfile(r/'qa'/f,q/'renderer'/f)
 run('npm-v1',['npm','ci','--prefix',str(q/'v1'),'--cache',str(q/'npm'),'--ignore-scripts','--no-audit','--no-fund'])
 run('npm-renderer',['npm','ci','--prefix',str(q/'renderer'),'--cache',str(q/'npm'),'--ignore-scripts','--no-audit','--no-fund'])
run('source-oracle',['node',str(r/'qa/source-oracle.cjs')])
run('host-patch',['python3',str(r/'qa/host-patch.py')]);run('host-contract',['python3',str(r/'qa/host-contract.py')]);run('producers',['python3',str(r/'qa/producers.py')])
run('patch-check',['git','apply','--check','--directory=v2/qa/fixtures/host','v2/integration/host-prerequisites.patch'])
run('fmt',['cargo','fmt','--manifest-path',str(r/'Cargo.toml'),'--check'])
run('fresh-build',['python3',str(r/'qa/fresh-build.py')]);run('notices',['python3',str(r/'qa/notices.py')]);run('package',['python3',str(r/'qa/package.py')])
for version,label in [('stable','stable'),('1.85.0','msrv')]:
 for command,tail in [('test',[]),('clippy',['--','-D','warnings'])]:
  run(('tests'if command=='test'else command)+'-'+label,['cargo','+'+version,command,'--locked','--all-targets','--manifest-path',str(r/'Cargo.toml'),*tail],{'CARGO_TARGET_DIR':str(q/('target'if label=='stable'else'target-msrv'))})
worker=max([p for p in (q/'target/debug/deps').glob('image_to_ascii_native-*')if p.is_file()and os.access(p,os.X_OK)and p.suffix==''],key=lambda p:p.stat().st_mtime)
run('process-worker',['python3',str(r/'qa/process.py'),'--worker',str(worker)])
run('renderer',['node',str(r/'qa/renderer.cjs')]);run('schema',['node',str(r/'qa/schema.cjs')]);run('v1-test',['npm','--prefix',str(q/'v1'),'test']);run('v1-typecheck',['npm','--prefix',str(q/'v1'),'run','typecheck'])
run('windows-gnu-check',['cargo','check','--locked','--target','x86_64-pc-windows-gnu','--manifest-path',str(r/'Cargo.toml')],{'CARGO_TARGET_DIR':str(q/'windows-target')},allowed=(0,101))
run('windows-clang-check',['cargo','check','--locked','--target','x86_64-pc-windows-gnu','--manifest-path',str(r/'Cargo.toml')],{'CARGO_TARGET_DIR':str(q/'windows-clang-target'),'CC_x86_64_pc_windows_gnu':'/usr/bin/clang','AR_x86_64_pc_windows_gnu':'/usr/bin/ar'},allowed=(0,101))
run('diff-check',['git','diff','--check'])
(q/'verification.json').write_text(json.dumps({'tests':results},indent=2)+'\n')
