"""One authorized local original-manager baseline. No worker release or server."""
from pathlib import Path
import hashlib,importlib.util,json,os,shutil,subprocess,sys,time
ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'target/original-manager-live-20261028'
PARENT=ROOT/'target/dbus-original-delivery-20261028/unit-api-owned'
NATIVE=Path('/home/artur/.cache/hydracache-074-dbus-original-delivery/20261028-a7')
REVIEW=json.loads((PARENT/'freeze/result.json').read_bytes())
TEST='privileged_original_manager_service_once'
sha=lambda b:hashlib.sha256(b).hexdigest()
def need(v,why):
 if not v:raise ValueError(why)
def load(path,name):
 spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
p=load(ROOT/'tools/dbus-original-delivery-074/prepare.py','original_live_preparation');owned=p.prior.owned
def write(folder,name,data):
 with (folder/name).open('x',encoding='utf8',newline='\n') as f:json.dump(data,f,indent=2,sort_keys=True);f.write('\n');f.flush();os.fsync(f.fileno())
def guard():
 need(p.files(PARENT/'input')==REVIEW['files'],'exact previously qualified source copy')
 need(p.fixed.prior.parent.parent.fragment.prior.parent.old.root_inputs()==REVIEW['root_inputs'],'unchanged protected inputs')
def capacity():
 if shutil.disk_usage(ROOT).free<4*1024**3:raise owned.CapacityError('original physical 4 GiB floor')
def linux(path):
 path=path.resolve();need(path.is_relative_to(ROOT),'exact workspace path');return '/mnt/c/'+path.as_posix()[3:] if os.name=='nt' else str(path)

def build_native():
 need(sha((ROOT/'tools/dbus-linux-074/native.py').read_bytes())=='09ab4fb404b93af4ec5581b189d3e7fe54562fff14626bfabcbc07c11c9f51b1','same pinned CI owner')
 owner=load(ROOT/'tools/dbus-linux-074/native.py','original_live_build_owner')
 owner.NATIVE=NATIVE;owner.sources(REVIEW);owner.capacity();need(os.getuid()==os.getgid()==1000,'nonroot compiler')
 folder=BASE/'build';deadline=time.monotonic()+1100;group=load(ROOT/'scripts/ci/watchdog_linux_group.py','original_live_group');receipts={}
 toolchain=Path('/home/artur/.rustup/toolchains/1.94.0-x86_64-unknown-linux-gnu/bin');target=NATIVE/'builds/fd/194-original-release'
 env=dict(HOME='/home/artur',USER='artur',LOGNAME='artur',PATH=str(toolchain)+':/usr/local/bin:/usr/bin:/bin',LANG='C.UTF-8',CARGO_HOME=str(NATIVE/'cargo-home'),CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='2')
 for label,command in [('libtest',['test','--lib','--no-run']),('bootstrap',['build','--bin','diagnostic_held_bootstrap'])]:
  out=folder/label;out.mkdir();argv=[str(toolchain/'cargo'),*command,'--release','--offline','--locked','--manifest-path',str(NATIVE/'fd/Cargo.toml'),'--target-dir',str(target),'-p','hydracache-long-run-supervisor-074','--message-format=json']
  write(out,'intent.json',dict(argv=argv,environment=env,deadline=deadline,bound=1200,session_bound=1100,floor=4*1024**3,poll=5,retirement=5,retry=False,tests_executed=False))
  # Disposable process owner keeps the original absolute origin across commands.
  with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
   child=subprocess.Popen(['/usr/bin/python3','-B',str(Path(__file__)),'build-phase',label,str(deadline)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,start_new_session=True);code=child.wait(timeout=max(0,deadline-time.monotonic())+10)
  execution=json.loads((out/'execution.json').read_bytes());need(execution['retired'] and not execution['errors'],'owned build retirement');owner.sources(REVIEW)
  messages=[json.loads(line) for line in (out/'stdout.log').read_bytes().splitlines()]
  found=[m for m in messages if m.get('reason')=='compiler-artifact' and m.get('executable') and m['target']['name']==('hydracache_long_run_supervisor_074' if label=='libtest' else 'diagnostic_held_bootstrap') and bool(m['profile']['test'])==(label=='libtest')]
  result=dict(exit=code,source_unchanged=True,tests_executed=False)
  if code==0:
   need(len(found)==1 and [m['success'] for m in messages if m.get('reason')=='build-finished']==[True],'actual exact compiler artifact');image=Path(found[0]['executable']);need(image.resolve().is_relative_to(target),'owned artifact target');result['artifact']=dict(path=str(image),sha256=sha(image.read_bytes()),bytes=image.stat().st_size);receipts[label]=result['artifact']
  write(out,'result.json',result);need(code==0,'first build refusal: '+label)
 write(folder,'native-result.json',dict(passed=True,artifacts=receipts,source_unchanged=True,tests_executed=False,release_admitted=False))

def phase(label,deadline):
 need(label in ('libtest','bootstrap'),'fixed compiler phase');intent=json.loads((BASE/'build'/label/'intent.json').read_bytes());owner=load(ROOT/'tools/dbus-linux-074/native.py','original_live_phase');owner.NATIVE=NATIVE;owner.sources(REVIEW);owner.capacity();group=load(ROOT/'scripts/ci/watchdog_linux_group.py','original_live_phase_group')
 os.environ.clear();os.environ.update(intent['environment']);result=owned._owned(group.LinuxGroup(),intent['argv'],str(NATIVE/'fd'),owner.capacity,bound=1200,deadline=deadline);write(BASE/'build'/label,'execution.json',result);raise SystemExit(result['exit'])

def outcome(code,live):
 send='original_send=true manager_ack=true' in live;ready='readiness=true release=false completion=false' in live
 valid=code==0 and send and ready and live.count('running 1 test')==1 and 'eof=true fence_retained=true custodians_terminal=true' in live
 return dict(exit=(0 if valid else (code or 74)),original_send=send,readiness=ready,cleanup_is_not_autonomous_retirement=True,original_release=False,completion=False,reconnect_tested=False,server=False,retry=False,release_admitted=False)

def dispatch(mode,worker=False):
 need(mode in ('build','baseline-once'),'fixed dispatch');out=BASE/mode
 if mode=='build':argv=['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B',linux(Path(__file__)),'build-native']
 else:
  proof=json.loads((BASE/'build/native-result.json').read_bytes());need(proof['passed'],'successful exact fresh release build')
  bundle=ROOT/'target/diagnostic-start-material-bundle-20261017';need((bundle/'timing-controls-074').is_file(),'original nonexecuted input bundle')
  argv=['wsl.exe','-d','HydraCache074Native','-u','root','--cd','/','--exec','/bin/sh',linux(ROOT/'tools/original-manager-live-074/baseline_fixture.sh'),linux(out),linux(bundle),proof['artifacts']['bootstrap']['sha256'],proof['artifacts']['libtest']['sha256']]
 if worker:
  guard();result=owned._owned(owned.load_job().WindowsJob(),argv,str(ROOT),capacity,bound=1200);write(out,'outer-execution.json',result);raise SystemExit(result['exit'])
 out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 with (out/'session.stdout.log').open('xb') as stdout,(out/'session.stderr.log').open('xb') as stderr:
  try:
   guard();capacity();before=p.prior.snapshot();head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip()
   if mode=='baseline-once':
    need(not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT).strip(),'committed clean caller before actual admission')
    for label,name in [('bootstrap','bootstrap'),('libtest','libtest')]:
     row=proof['artifacts'][label];source=Path(r'\\wsl.localhost\Ubuntu')/row['path'].lstrip('/');need(sha(source.read_bytes())==row['sha256'],'held build artifact before transfer');shutil.copyfile(source,out/name);need(sha((out/name).read_bytes())==row['sha256'],'exact transferred artifact')
   write(out,'intent.json',dict(argv=argv,head=head,source=before,original_windows=[300,60,10],bound=1200,session_bound=1100,live_bound=120,retirement=5,floor=4*1024**3,retry=False,server=False,release_install=False,existing_services_changed=False))
   child=subprocess.Popen([sys.executable,'-B',str(Path(__file__)),mode,'--worker'],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,cwd=ROOT);code=child.wait(timeout=1210)
   guard();need(before==p.prior.snapshot() and head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip(),'source/head postflight');e=json.loads((out/'outer-execution.json').read_bytes());need(e['retired'] and not e['errors'],'outer retirement')
   result=dict(exit=code,source_unchanged=True,release_admitted=False)
   if mode=='baseline-once':
    live=(out/'live.stdout.log').read_text(encoding='utf8',errors='replace') if (out/'live.stdout.log').exists() else '';result=outcome(code,live)
    need((out/'postflight-result.txt').read_text().strip().endswith('cleanup_rc=0'),'exact owned cleanup');need((out/'preflight.txt').read_bytes()==(out/'postflight.txt').read_bytes(),'existing service/clock mask context preserved')
   write(out,'result.json',result);print(json.dumps(result));raise SystemExit(result['exit'])
  except Exception as error:write(out,'refusal.json',dict(type=type(error).__name__,reason=str(error)));raise

if __name__=='__main__':
 if sys.argv[1:]==['build-native']:build_native()
 elif len(sys.argv)==4 and sys.argv[1]=='build-phase':phase(sys.argv[2],float(sys.argv[3]))
 else:
  need(len(sys.argv) in (2,3) and (len(sys.argv)==2 or sys.argv[2]=='--worker'),'fixed CLI');dispatch(sys.argv[1],len(sys.argv)==3)
