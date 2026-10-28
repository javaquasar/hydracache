"""Fixed local caller continuation; no real bus/manager or product adoption."""
from pathlib import Path
import json,os,re,shutil,subprocess,sys
ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'target/dbus-owner-properties-20261028'
sys.path.insert(0,str(ROOT/'tools/windows-workspace-preflight-074'))
import prepare as p
parent=p.load(ROOT/'tools/dbus-runtime-bridge-074/run.py','runtime_parent')
sha,files,write,snapshot,owned=parent.sha,parent.files,parent.write,parent.snapshot,parent.owned
EXTRAS=['caller_checks::owner_properties::'+n for n in ('manager_owner_valid_reads_in_original_order',
 'manager_identity_refuses_after_all_three_reads','manager_wrong_sender_stops_at_original_read',
 'manager_signature_and_error_refuse_without_later_reads','property_string_u32_and_no_cache_roundtrip',
 'property_wrong_type_body_and_error_refuse','property_received_but_unanswered_preserves_timeout')]

def prepare():
 out=BASE/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 packet=ROOT/'docs/testing/performance/0.74/local-runs/dbus-runtime-bridge-20261028'
 raw=(packet/'index.json').read_bytes();p.require(sha(raw)=='59b7f69b3ee958f2a467af97d1cd80ed0d7f35194e33d9fbb58ed001602e1cda','held predecessor packet')
 index=json.loads(raw);row=index['members']['capture/prepare-S/result.json']
 receipt=(packet/'objects'/row['sha256']).read_bytes();p.require(sha(receipt)==row['sha256'],'held original source receipt')
 review=json.loads(receipt);source=parent.BASE/'fixtures/S'
 p.require(files(source)==review['files'] and parent.old.root_inputs()==review['root_inputs'],'exact passing prior S and protected root')
 p.require(all(sha((ROOT/n).read_bytes())==v for n,v in review['overlays'].items()),'exact held parent overlays')
 dest=BASE/'fixture';shutil.copytree(source,dest)
 original=(ROOT/parent.OWNER).read_bytes();p.require(sha(original)==parent.OWNER_SHA,'independent original manager pin')
 body=original.decode();function=body[body.index('fn manager_owner('):body.index('#[derive(Debug, PartialEq, Eq)]\nenum ReplyError')]
 p.require(function.count('checked_call(')==3 and function.count('uid != 0 || pid != 1')==1,'complete original owner body')
 path=dest/'probe/src/caller_checks.rs';before=path.read_text(encoding='utf-8')
 insert='const DESTINATION: &str = "org.freedesktop.systemd1";\n'+function+'\n'
 p.require(before.count('\n#[cfg(test)]')==1,'one exact old test boundary')
 after=before.replace('\n#[cfg(test)]','\n'+insert+'#[cfg(test)]',1)
 overlay=ROOT/'tools/dbus-owner-properties-074/tests.rs'
 after+='\n#[cfg(test)]\n#[path = '+json.dumps(overlay.as_posix())+']\nmod owner_properties;\n'
 path.write_text(after,encoding='utf-8',newline='\n')
 (out/'original-owner-function.rs').write_text(function,encoding='utf-8',newline='\n')
 write(out,'result.json',dict(files=files(dest),root_inputs=review['root_inputs'],overlays=dict(review['overlays'],
  **{overlay.relative_to(ROOT).as_posix():sha(overlay.read_bytes())}),source_pin=parent.OWNER_SHA,
  original_owner_function_sha256=sha(function.encode()),root_caller_changed=False,release_admitted=False))
 print('prepared one local owner/property continuation; graph unchanged')

def inventory(raw,baseline):
 lines=raw.decode().splitlines();names=[n[:-6] for n in lines if n.endswith(': test')]
 p.require(len(baseline)==14 and len(set(names))==len(names)==21 and lines[-1]=='21 tests, 0 benchmarks','complete 21-case catalog')
 p.require(sorted(names)==sorted(baseline+EXTRAS),'only seven declared additions')
 return sorted(names)

def run(label,worker):
 match=re.fullmatch(r'(build|list|test|check|clippy|policy)-(194|188)',label);p.require(match is not None,'fixed command')
 action,compiler=match.groups();review=json.loads((BASE/'prepare/result.json').read_bytes());folder=BASE/'fixture'
 def guard():
  p.require(parent.old.root_inputs()==review['root_inputs'] and sha((ROOT/parent.OWNER).read_bytes())==parent.OWNER_SHA,'held root/caller')
  p.require(files(folder)==review['files'],'whole frozen source/lock')
  p.require(all(sha((ROOT/n).read_bytes())==v for n,v in review['overlays'].items()),'held overlay bodies')
 identity=sha(json.dumps(dict(files=review['files'],overlays=review['overlays'],compiler=compiler),sort_keys=True).encode())
 target=BASE/'targets'/compiler/identity;cargo=['cargo','+1.94.0' if compiler=='194' else '+1.88.0']
 common=['--offline','--locked','--manifest-path',str(folder/'Cargo.toml'),'--target-dir',str(target),'-p','dbus-probe-074']
 artifact=None
 if action=='build':argv=cargo+['test',*common,'--lib','--no-run','--message-format=json']
 elif action in ('check','clippy'):argv=cargo+[action,*common,'--all-targets','--all-features']+(['--','-D','warnings'] if action=='clippy' else [])
 elif action=='policy':argv=cargo+['deny','--manifest-path',str(folder/'Cargo.toml'),'--offline','--locked','check','--config',str(ROOT/'deny.toml')]
 else:
  built=json.loads((BASE/f'build-{compiler}/result.json').read_bytes());p.require(built['exit']==0 and built['unchanged'],'actual successful build')
  artifact=built['artifact'];image=Path(artifact['path'])
  p.require(image.resolve().is_relative_to(target) and not image.is_symlink() and not image.is_junction() and sha(image.read_bytes())==artifact['sha256'],'held actual executable')
  if action=='list':argv=[str(image),'--list']
  else:
   listing=json.loads((BASE/f'list-{compiler}/result.json').read_bytes());p.require(listing['exit']==0 and listing['unchanged'] and listing['artifact']==artifact,'held complete catalog')
   argv=[str(image),'--test-threads=1','--show-output']
 out=BASE/label
 if worker:
  guard()
  def capacity():
   if shutil.disk_usage(ROOT).free<p.FLOOR:raise owned.CapacityError('original 4 GiB floor')
  execution=owned._owned(owned.load_job().WindowsJob(),argv,str(ROOT),capacity,bound=1200)
  write(out,'execution.json',execution);raise SystemExit(execution['exit'])
 out.mkdir(exist_ok=False);(out/'launcher.py').write_bytes(Path(__file__).read_bytes())
 with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
  try:
   guard();before=snapshot();head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip()
   write(out,'intent.json',dict(argv=argv,source=before,head=head,artifact=artifact,compiler=compiler,
    bound=1200,floor=p.FLOOR,capacity_poll=5,retirement=5,retry=False,system_bus=False,server=False,services=False,clocks=False))
   child=subprocess.Popen([sys.executable,'-B',str(Path(__file__)),label,'--worker'],cwd=ROOT,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,
    env=dict(os.environ,CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='2'))
   try:code=child.wait(timeout=1210)
   except subprocess.TimeoutExpired:
    child.kill();child.wait(timeout=5);raise ValueError('worker lost; no PID fallback')
   guard();execution=json.loads((out/'execution.json').read_bytes())
   unchanged=before==snapshot() and head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip()
   p.require(unchanged and execution['retired'] and not execution['errors'],'source/head/owned retirement')
   result=dict(exit=code,unchanged=unchanged,artifact=artifact,release_admitted=False);raw=(out/'stdout.log').read_bytes()
   if action=='build' and code==0:
    messages=[json.loads(n) for n in raw.splitlines()]
    found=[m for m in messages if m.get('reason')=='compiler-artifact' and m['target']['name']=='dbus_probe_074' and m['target']['kind']==['lib'] and m['profile']['test'] and m.get('executable')]
    p.require(len(found)==1 and [m['success'] for m in messages if m.get('reason')=='build-finished']==[True],'actual Cargo test image')
    image=Path(found[0]['executable']);p.require(image.resolve().is_relative_to(target) and Path(found[0]['target']['src_path']).resolve()==(folder/'probe/src/lib.rs').resolve(),'actual image source')
    result['artifact']=dict(path=str(image),sha256=sha(image.read_bytes()),bytes=image.stat().st_size)
   if artifact:p.require(sha(Path(artifact['path']).read_bytes())==artifact['sha256'],'image postflight')
   if action=='list' and code==0:result['inventory']=inventory(raw,json.loads((parent.BASE/'list-S-194/result.json').read_bytes())['inventory'])
   if action=='test':
    a=p.load(ROOT/'tools/chitchat-mtu-review-074/accounting.py','owner_account')
    result['accounting']=a.account(raw,listing['inventory'],'',False);p.require((code==0)==result['accounting']['test_pass'],'native exit/count agreement')
   write(out,'result.json',result)
  except BaseException as error:
   write(out,'refusal.json',dict(type=type(error).__name__,reason=str(error),release_admitted=False));raise
 print(json.dumps(dict(label=label,exit=code,unchanged=unchanged)));raise SystemExit(code)

if __name__=='__main__':
 p.require(len(sys.argv) in (2,3) and (len(sys.argv)==2 or sys.argv[2]=='--worker'),'fixed invocation')
 if sys.argv[1]=='prepare':p.require(len(sys.argv)==2,'inert prepare');prepare()
 else:run(sys.argv[1],len(sys.argv)==3)
