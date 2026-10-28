"""Native library-only compile coordinator; uses the unchanged owned process owner."""
from pathlib import Path
import hashlib,importlib.util,json,os,shutil,subprocess,sys,tarfile,time
ROOT=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
BASE=ROOT/'target/dbus-original-owner-20261028'
NATIVE=Path('/home/artur/.cache/hydracache-074-dbus-original-owner/20261028-a1')
OUT=BASE/'session';OWNER_SHA='09ab4fb404b93af4ec5581b189d3e7fe54562fff14626bfabcbc07c11c9f51b1'
CORRECTED=len(sys.argv)>1 and sys.argv[1] in ('session-lint-fixed','worker-lint-fixed')
if CORRECTED:
 BASE=BASE/'lint-fixed';NATIVE=Path(str(NATIVE).replace('-a1','-a2'));OUT=BASE/'session'
PREP=BASE/('prepare/result.json' if CORRECTED else 'freeze/result.json')
def load(path,name):
 spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
def configured():
 path=ROOT/'tools/dbus-linux-074/native.py'
 if hashlib.sha256(path.read_bytes()).hexdigest()!=OWNER_SHA:raise ValueError('original CI owner drift')
 owner=load(path,'hc074_owner_full_types');owner.BASE=BASE;owner.NATIVE=NATIVE;owner.OUT=OUT
 original=owner.command
 def command(label,review):
  parts=label.split('-')
  if len(parts)!=3 or parts[0] not in ('baseline','fd') or parts[1] not in ('194','188') or parts[2] not in ('check','clippy'):raise ValueError('fixed library-only phase before any parent receipt reads')
  argv,env,artifact,target,folder,action=original(label,review)
  if action not in ('check','clippy'):raise ValueError('library-only no run')
  argv[argv.index('dbus-probe-074')]='hydracache-long-run-supervisor-074';argv[argv.index('--all-targets')]='--lib'
  return argv,env,artifact,target,folder,action
 owner.command=command
 owner.prepared=lambda:PREP
 return owner
if len(sys.argv)==4 and sys.argv[1] in ('worker','worker-lint-fixed'):
 owner=configured();owner.worker(sys.argv[2],float(sys.argv[3]));raise SystemExit('worker exit required')
if len(sys.argv)!=2 or sys.argv[1] not in ('session','session-lint-fixed'):raise ValueError('fixed session/worker')
owner=configured();os.umask(0o077);review=json.loads(owner.prepared().read_bytes());deadline=time.monotonic()+1100
result=dict(passed=False,library_only=True,tests_executed=False,root_changed=False,server=False,services=False,clocks_changed=False,phases=[],boot_before=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),clocks_before=owner.clocks())
try:
 owner.need(sys.platform=='linux' and os.getuid()==os.getgid()==1000,'original nonroot local Linux')
 parent=NATIVE.parent
 if not parent.exists():parent.mkdir(mode=0o700)
 owner.need(parent.resolve()==parent and parent.stat().st_uid==os.getuid() and not parent.stat().st_mode&0o077,'exclusive native parent')
 NATIVE.mkdir(mode=0o700,exist_ok=False);owner.capacity()
 owner.need(owner.tree(BASE/'input')==review['files'],'exact frozen copy')
 for name in review['files']:
  source=BASE/'input'/name;owner.need(not source.is_symlink(),'regular input');dest=NATIVE/name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(source.read_bytes())
 vendor={}
 for row in review['registry']:
  archive=NATIVE/'archives'/(row['name']+'.crate');owner.need(owner.sha(archive.read_bytes())==row['sha256'],'held registry archive')
  directory=NATIVE/'vendor'/row['name'];directory.mkdir(parents=True);members={}
  with tarfile.open(archive,'r:gz') as tar:
   for entry in tar:
    owner.need(entry.name.startswith(row['name']+'/') and '..' not in Path(entry.name).parts,'archive scope')
    if entry.isdir():continue
    owner.need(entry.isfile(),'regular archive member');rel=entry.name[len(row['name'])+1:];dest=directory/rel;dest.parent.mkdir(parents=True,exist_ok=True);raw=tar.extractfile(entry).read();dest.write_bytes(raw);members[rel]=owner.sha(raw)
  owner.write(directory,'.cargo-checksum.json',dict(package=row['sha256'],files=members));vendor[row['name']]=members
 owner.write(OUT,'vendor-receipt.json',vendor)
 for variant in (('fd',) if review.get('reuse_passing_baseline') else ('baseline','fd')):
  for compiler in ('194','188'):
   for action in ('check','clippy'):
    owner.need(time.monotonic()<deadline,'original session origin exhausted');owner.sources(review);owner.capacity()
    label=f'{variant}-{compiler}-{action}';out=OUT/label;out.mkdir(exist_ok=False);argv,env,_,_,folder,_=owner.command(label,review)
    owner.write(out,'intent.json',dict(argv=argv,environment=env,absolute_deadline=deadline,bound=1200,session_bound=1100,floor=4*1024**3,poll=5,retirement=5,retry=False,library_only=True))
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
     child=subprocess.Popen(['/usr/bin/python3','-B',str(Path(__file__)), 'worker-lint-fixed' if CORRECTED else 'worker',label,str(deadline)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,start_new_session=True)
     code=child.wait(timeout=max(0,deadline-time.monotonic())+10)
    e=json.loads((out/'execution.json').read_bytes());owner.need(e['retired'] and not e['errors'],'owned phase retirement');owner.sources(review)
    owner.write(out,'result.json',dict(exit=code,source_unchanged=True,library_only=True,tests_executed=False,release_admitted=False));result['phases'].append(dict(label=label,exit=code));print(json.dumps(dict(phase=label,exit=code)),flush=True)
    owner.need(code==0,'first actual library refusal: '+label)
 for name,expected in vendor.items():
  actual=owner.tree(NATIVE/'vendor'/name);actual.pop('.cargo-checksum.json');owner.need(actual==expected,'vendor source postflight')
 result['passed']=True
except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error))
finally:
 try:owner.sources(review);result['source_unchanged']=True
 except BaseException as error:result['source_unchanged']=False;result['source_refusal']=str(error);result['passed']=False
 result.update(boot_after=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),clocks_after=owner.clocks());result['boot_unchanged']=result['boot_after']==result['boot_before'];owner.write(OUT,'native-result.json',result)
raise SystemExit(0 if result['passed'] and result['boot_unchanged'] else 1)
