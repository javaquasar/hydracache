"""Local outer owner for a complete production-library type-check fixture."""
from pathlib import Path
import importlib.util,json,shutil,subprocess,sys
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('owner_launch_preparation',ROOT/'tools/dbus-original-owner-074/prepare.py');r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
corrected=len(sys.argv)==2 and sys.argv[1] in ('lint-fixed','--worker-lint-fixed')
base=r.BASE/'lint-fixed' if corrected else r.BASE
receipt=base/('prepare/result.json' if corrected else 'freeze/result.json')
out=base/'session';review=json.loads(receipt.read_bytes())
argv=['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-original-owner-074/native.py','session-lint-fixed' if corrected else 'session']
def guard():
 r.p.require(r.files(base/'input')==review['files'] and r.fixed.prior.parent.parent.fragment.prior.parent.old.root_inputs()==review['root_inputs'],'source/root custody')
if len(sys.argv)==2 and sys.argv[1].startswith('--worker'):
 r.p.require(sys.argv[1] in ('--worker','--worker-lint-fixed'),'fixed worker');guard()
 def capacity():
  if shutil.disk_usage(ROOT).free<r.p.FLOOR:raise r.prior.owned.CapacityError('original 4 GiB floor')
 result=r.prior.owned._owned(r.prior.owned.load_job().WindowsJob(),argv,str(ROOT),capacity,bound=1200);r.write(out,'execution.json',result);raise SystemExit(result['exit'])
r.p.require(len(sys.argv)==1 or (corrected and sys.argv[1]=='lint-fixed'),'fixed session');out.mkdir(exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
 try:
  guard();before=r.prior.snapshot();head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip()
  r.write(out,'intent.json',dict(argv=argv,source=before,head=head,freeze_sha256=r.sha(receipt.read_bytes()),bound=1200,session_bound=1100,floor=r.p.FLOOR,poll=5,retirement=5,retry=False,library_only=True))
  child=subprocess.Popen([sys.executable,'-B',str(Path(__file__)),'--worker-lint-fixed' if corrected else '--worker'],cwd=ROOT,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
  try:code=child.wait(timeout=1210)
  except subprocess.TimeoutExpired:child.kill();child.wait(timeout=5);raise ValueError('owned session worker lost')
  guard();r.p.require(before==r.prior.snapshot() and head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip(),'source/head postflight')
  e=json.loads((out/'execution.json').read_bytes());r.p.require(e['retired'] and not e['errors'],'outer retirement')
  result=json.loads((out/'native-result.json').read_bytes());r.p.require(code==0 and result['passed'] and result['source_unchanged'] and result['boot_unchanged'],'complete library type-check gate')
  r.write(out,'result.json',dict(exit=0,source_unchanged=True,library_only=True,tests_executed=False,root_changed=False,release_admitted=False))
 except BaseException as error:r.write(out,'refusal.json',dict(type=type(error).__name__,reason=str(error)));raise
print(json.dumps(dict(exit=code,library_only=True)))
