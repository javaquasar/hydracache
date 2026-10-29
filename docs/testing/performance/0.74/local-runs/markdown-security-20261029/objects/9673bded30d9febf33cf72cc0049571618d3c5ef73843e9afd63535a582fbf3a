"""Strict test-cfg lint over the held native source; never executes tests or bins."""
from pathlib import Path
import hashlib,importlib.util,json,os,subprocess,sys,time
ROOT=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074');BASE=ROOT/'target/dbus-original-delivery-20261028/unit-api-owned'
NATIVE=Path('/home/artur/.cache/hydracache-074-dbus-original-delivery/20261028-a7');OUT=BASE/'lint'

def configured():
 path=ROOT/'tools/dbus-linux-074/native.py'
 if hashlib.sha256(path.read_bytes()).hexdigest()!='09ab4fb404b93af4ec5581b189d3e7fe54562fff14626bfabcbc07c11c9f51b1':raise ValueError('same original owned process helper')
 spec=importlib.util.spec_from_file_location('delivery_test_cfg_lint',path);owner=importlib.util.module_from_spec(spec);spec.loader.exec_module(owner)
 owner.BASE=BASE;owner.NATIVE=NATIVE;owner.OUT=OUT;owner.prepared=lambda:BASE/'freeze/result.json';original=owner.command
 def command(label,review):
  owner.need(label in ('fd-194-clippy','fd-188-clippy'),'fixed strict test-cfg lint')
  argv,env,artifact,target,folder,action=original(label,review)
  argv[argv.index('dbus-probe-074')]='hydracache-long-run-supervisor-074';i=argv.index('--all-targets');argv[i:i+1]=['--lib','--tests']
  return argv,env,artifact,target,folder,action
 owner.command=command;return owner

if len(sys.argv)==4 and sys.argv[1]=='worker':
 owner=configured();owner.worker(sys.argv[2],float(sys.argv[3]));raise SystemExit('worker exits')
if sys.argv[1:]!=['session']:raise ValueError('fixed lint session')
owner=configured();review=json.loads(owner.prepared().read_bytes());deadline=time.monotonic()+1100;os.umask(0o077)
result=dict(passed=False,tests_executed=False,server=False,services=False,clocks_changed=False,release_admitted=False,phases=[],boot_before=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),clocks_before=owner.clocks())
try:
 owner.need(os.getuid()==os.getgid()==1000,'same nonroot Linux');owner.sources(review);owner.capacity()
 runtime=json.loads((BASE/'runtime/native-result.json').read_bytes());owner.need(runtime['passed'] and runtime['passes_per_compiler']==41,'completed selected wire runtime first')
 for compiler in ('194','188'):
  owner.need(time.monotonic()<deadline,'same absolute session origin');owner.sources(review);owner.capacity();label=f'fd-{compiler}-clippy';out=OUT/label;out.mkdir()
  argv,env,_,_,folder,_=owner.command(label,review);owner.write(out,'intent.json',dict(argv=argv,environment=env,absolute_deadline=deadline,bound=1200,session_bound=1100,floor=4*1024**3,poll=5,retirement=5,retry=False,tests_executed=False))
  with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
   child=subprocess.Popen(['/usr/bin/python3','-B',str(Path(__file__)),'worker',label,str(deadline)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,start_new_session=True);code=child.wait(timeout=max(0,deadline-time.monotonic())+10)
  e=json.loads((out/'execution.json').read_bytes());owner.need(e['retired'] and not e['errors'],'owned lint retirement');owner.sources(review);owner.write(out,'result.json',dict(exit=code,source_unchanged=True,tests_executed=False,release_admitted=False));result['phases'].append(dict(label=label,exit=code));print(json.dumps(dict(phase=label,exit=code)),flush=True);owner.need(code==0,'first strict test-cfg refusal: '+label)
 vendor=json.loads((BASE/'session/vendor-receipt.json').read_bytes())
 for name,expected in vendor.items():
  actual=owner.tree(NATIVE/'vendor'/name);actual.pop('.cargo-checksum.json');owner.need(actual==expected,'held vendor postflight')
 result['passed']=True
except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error))
finally:
 try:owner.sources(review);result['source_unchanged']=True
 except BaseException as error:result['source_unchanged']=False;result['source_refusal']=str(error);result['passed']=False
 result.update(boot_after=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),clocks_after=owner.clocks());result['boot_unchanged']=result['boot_before']==result['boot_after'];owner.write(OUT,'native-result.json',result)
raise SystemExit(0 if result['passed'] and result['boot_unchanged'] else 1)
