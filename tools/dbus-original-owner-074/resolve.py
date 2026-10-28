"""Owned offline resolver; freezes output changes without altering product inputs."""
from pathlib import Path
import json,os,shutil,subprocess,sys
ROOT=Path(__file__).resolve().parents[2]
import importlib.util
spec=importlib.util.spec_from_file_location('owner_fixture_preparation',ROOT/'tools/dbus-original-owner-074/prepare.py')
r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
label=sys.argv[1];r.p.require(label in ('resolve-baseline','resolve-fd'),'fixed metadata label')
variant=label.split('-',1)[1];folder=r.BASE/'input'/variant;out=r.BASE/label
argv=[str(Path(os.environ['USERPROFILE'])/'.rustup/toolchains/1.94.0-x86_64-pc-windows-msvc/bin/cargo.exe'),'metadata','--offline','--format-version','1','--manifest-path',str(folder/'Cargo.toml')]
def guard():
 review=json.loads((r.BASE/'prepare/result.json').read_bytes());r.p.require(r.fixed.prior.parent.parent.fragment.prior.parent.old.root_inputs()==review['root_inputs'],'protected root inputs')
 expected=review['baseline_files'] if variant=='baseline' else json.loads((r.BASE/'candidate/result.json').read_bytes())['files']
 actual=r.files(folder);expected=dict(expected);actual.pop('Cargo.lock',None);expected.pop('Cargo.lock',None)
 r.p.require(actual==expected,'whole fixture except intentional first resolver lock write')
 return review
if len(sys.argv)==3:
 r.p.require(sys.argv[2]=='--worker','fixed worker');guard()
 def capacity():
  if shutil.disk_usage(ROOT).free<r.p.FLOOR:raise r.prior.owned.CapacityError('original 4 GiB floor')
 for key in ('RUSTFLAGS','RUSTDOCFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTC_BOOTSTRAP'):
  r.p.require(not os.environ.get(key),'no injected compiler configuration '+key)
 os.environ['CARGO_INCREMENTAL']='0';os.environ['CARGO_BUILD_JOBS']='2';os.environ['CARGO_TARGET_DIR']=str(r.BASE/'metadata-target');os.environ['CARGO_HOME']=str(Path(os.environ['USERPROFILE'])/'.cargo')
 result=r.prior.owned._owned(r.prior.owned.load_job().WindowsJob(),argv,str(folder),capacity,bound=1200)
 r.write(out,'execution.json',result);raise SystemExit(result['exit'])
out.mkdir(exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
 try:
  guard();before=r.prior.snapshot();head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip()
  r.write(out,'intent.json',dict(argv=argv,source=before,head=head,files_before=r.files(folder),cargo_sha256=r.sha(Path(argv[0]).read_bytes()),bound=1200,floor=r.p.FLOOR,poll=5,retirement=5,offline=True,retry=False,root_changed=False))
  child=subprocess.Popen([sys.executable,'-B',str(Path(__file__)),label,'--worker'],cwd=ROOT,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
  try:code=child.wait(timeout=1210)
  except subprocess.TimeoutExpired:child.kill();child.wait(timeout=5);raise ValueError('owned resolver worker lost')
  guard();e=json.loads((out/'execution.json').read_bytes());r.p.require(e['retired'] and not e['errors'],'owned resolver retirement')
  r.p.require(before==r.prior.snapshot() and head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip(),'source/head resolver postflight')
  r.write(out,'result.json',dict(exit=code,root_unchanged=True,files_after=r.files(folder),release_admitted=False))
 except BaseException as error:r.write(out,'refusal.json',dict(type=type(error).__name__,reason=str(error)));raise
print(json.dumps(dict(label=label,exit=code)));raise SystemExit(code)
