#!/usr/bin/env python3
"""Build only copied final product sources with a new confined CargoHome."""
from pathlib import Path
import os,shutil,subprocess,json,hashlib
r=Path(__file__).resolve().parents[1]
qa=r/'.qa';run=qa/'fresh';run.mkdir(exist_ok=True)
# No existing data are deleted; a fresh directory each invocation.
import tempfile
work=Path(tempfile.mkdtemp(prefix='release-',dir=run));source=work/'source';source.mkdir()
for name in ['Cargo.toml','Cargo.lock','extension.json','src']:
 p=r/name
 if p.is_dir():shutil.copytree(p,source/name)
 else:shutil.copyfile(p,source/name)
for name in ['cargo','home','temp','target']:(work/name).mkdir()
env=dict(os.environ,CARGO_HOME=str(work/'cargo'),CARGO_TARGET_DIR=str(work/'target'),HOME=str(work/'home'),TMPDIR=str(work/'temp'),TEMP=str(work/'temp'),TMP=str(work/'temp'),RUSTUP_HOME=os.environ.get('RUSTUP_HOME',str(Path.home()/'.rustup')))
result=subprocess.run(['cargo','+stable','build','--release','--locked','--manifest-path',str(source/'Cargo.toml')],env=env,cwd=source,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
(work/'build.log').write_text(result.stdout)
print(result.stdout[-1800:]);assert result.returncode==0
binary=work/'target/release/image-to-ascii'
report={'work':str(work.relative_to(r)),'sourceCargoLockSha256':hashlib.sha256((r/'Cargo.lock').read_bytes()).hexdigest(),'binary':str(binary),'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'bytes':binary.stat().st_size,'independentCargoHome':True,'sdkSource':'git+https://github.com/almatkai/tezbar-extension-sdk-rust.git?rev=e955911f0bf5b558f9d3eb5183df0adc95c22af2'}
(qa/'fresh-build.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
