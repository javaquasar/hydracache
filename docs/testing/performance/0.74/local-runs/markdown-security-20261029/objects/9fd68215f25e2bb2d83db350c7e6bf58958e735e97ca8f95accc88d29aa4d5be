"""Execute only exact reviewed wire cases from the genuine full-library image."""
from pathlib import Path
import hashlib,importlib.util,json,os,re,subprocess,sys,time
ROOT=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
BASE=ROOT/'target/dbus-original-delivery-20261028/unit-api-owned'
NATIVE=Path('/home/artur/.cache/hydracache-074-dbus-original-delivery/20261028-a7')
OUT=BASE/'runtime'
OWNER_SHA='09ab4fb404b93af4ec5581b189d3e7fe54562fff14626bfabcbc07c11c9f51b1'
CASE_FILES=('diagnostic_held_delivery_wire_tests.rs','diagnostic_original_manager_message_units.rs','diagnostic_original_pipe_transport_tests.rs','diagnostic_dispatch_exchange_tests.rs','diagnostic_dispatch_reply_flags_tests.rs','diagnostic_dispatch_reply_state_tests.rs','diagnostic_original_reply_binding_tests.rs')

def configured():
 path=ROOT/'tools/dbus-linux-074/native.py'
 if hashlib.sha256(path.read_bytes()).hexdigest()!=OWNER_SHA:raise ValueError('same original process owner')
 spec=importlib.util.spec_from_file_location('delivery_wire_runtime_owner',path);owner=importlib.util.module_from_spec(spec);spec.loader.exec_module(owner)
 owner.BASE=BASE;owner.NATIVE=NATIVE;owner.OUT=OUT;owner.prepared=lambda:BASE/'freeze/result.json'
 original=owner.command
 def command(label,review):
  parts=label.split('-')
  owner.need(len(parts)==3 and parts[0]=='fd' and parts[1] in ('194','188') and parts[2] in ('list','test'),'exact image-only phase')
  argv,env,artifact,target,folder,action=original(label,review)
  if action=='test':
   listing=json.loads((OUT/f'fd-{parts[1]}-list/result.json').read_bytes())
   owner.need(len(listing['selected'])==41 and len(set(listing['selected']))==41,'exact reviewed forty plus adapter')
   argv+=['--exact',*listing['selected']]
  return argv,env,artifact,target,folder,action
 owner.command=command
 return owner

def select(catalog,owner):
 names=[]
 for name in CASE_FILES:
  raw=(NATIVE/'fd/tools/long-run-supervisor-074/src'/name).read_text(encoding='utf8')
  names+=re.findall(r'#\[test\]\s*fn (\w+)',raw)
 owner.need(len(names)==40 and len(set(names))==40,'original reviewed source names')
 names+=['negotiated_credential_adapter_keeps_optional_fields_and_process_fd_alias']
 selected=[]
 for leaf in names:
  found=[n for n in catalog if n.endswith('::'+leaf)]
  owner.need(len(found)==1,'unique actual catalog identity: '+leaf);selected+=found
 return sorted(selected)

if len(sys.argv)==4 and sys.argv[1]=='worker':
 owner=configured();owner.worker(sys.argv[2],float(sys.argv[3]));raise SystemExit('worker exits')
if sys.argv[1:]!=['session']:raise ValueError('fixed runtime session')
owner=configured();review=json.loads(owner.prepared().read_bytes());deadline=time.monotonic()+1100;os.umask(0o077)
result=dict(passed=False,tests_executed=True,selected_only=True,server=False,services=False,clocks_changed=False,release_admitted=False,phases=[],boot_before=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),clocks_before=owner.clocks())
try:
 owner.need(os.getuid()==os.getgid()==1000,'same nonroot Linux');owner.sources(review);owner.capacity()
 built=json.loads((BASE/'session/native-result.json').read_bytes());owner.need(built['passed'] and built['source_unchanged'] and built['boot_unchanged'],'complete actual unit build on both compilers')
 catalogs=[]
 for compiler in ('194','188'):
  build=BASE/f'session/fd-{compiler}-build/result.json';receipt=json.loads(build.read_bytes());owner.need(receipt['exit']==0 and receipt['artifact'],'actual GNU test image')
  folder=OUT/f'fd-{compiler}-build';folder.mkdir();owner.write(folder,'result.json',receipt)
  for action in ('list','test'):
   owner.need(time.monotonic()<deadline,'same absolute session origin');owner.sources(review);owner.capacity()
   label=f'fd-{compiler}-{action}';out=OUT/label;out.mkdir();argv,env,artifact,target,folder,_=owner.command(label,review)
   owner.write(out,'intent.json',dict(argv=argv,environment=env,artifact=artifact,absolute_deadline=deadline,bound=1200,session_bound=1100,floor=4*1024**3,poll=5,retirement=5,retry=False,selected_only=True))
   with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
    child=subprocess.Popen(['/usr/bin/python3','-B',str(Path(__file__)),'worker',label,str(deadline)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,start_new_session=True)
    code=child.wait(timeout=max(0,deadline-time.monotonic())+10)
   e=json.loads((out/'execution.json').read_bytes());owner.need(e['retired'] and not e['errors'],'same owned retirement');owner.sources(review)
   value=dict(exit=code,artifact=artifact,source_unchanged=True,selected_only=True,release_admitted=False);raw=(out/'stdout.log').read_text(encoding='utf8')
   if action=='list' and code==0:
    catalog=sorted(line[:-6] for line in raw.splitlines() if line.endswith(': test'));owner.need(len(catalog)==len(set(catalog)) and len(catalog)>41,'actual complete library catalog')
    value.update(inventory=catalog,selected=select(catalog,owner));catalogs.append(catalog)
   if action=='test':
    listing=json.loads((OUT/f'fd-{compiler}-list/result.json').read_bytes());rows=re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)$',raw,re.M)
    summary=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',raw,re.M)
    value.update(rows=rows,summary=summary,selected=listing['selected'])
    owner.write(out,'accounting.json',value)
    owner.need(len(rows)==41 and sorted(n for n,state in rows)==listing['selected'] and all(state=='ok' for n,state in rows),'all exact reviewed cases passed')
    owner.need(summary==[('ok','41','0','0','0',str(len(listing['inventory'])-41))] and code==0,'selected/full count and exit agree')
   owner.write(out,'result.json',value);result['phases'].append(dict(label=label,exit=code));print(json.dumps(dict(phase=label,exit=code)),flush=True);owner.need(code==0,'first runtime refusal: '+label)
 owner.need(len(catalogs)==2 and catalogs[0]==catalogs[1],'complete compiler catalogs agree');result.update(passed=True,catalog_count=len(catalogs[0]),passes_per_compiler=41,original_cases=40,adapter_cases=1)
except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error))
finally:
 try:owner.sources(review);result['source_unchanged']=True
 except BaseException as error:result['source_unchanged']=False;result['source_refusal']=str(error);result['passed']=False
 result.update(boot_after=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),clocks_after=owner.clocks());result['boot_unchanged']=result['boot_before']==result['boot_after'];owner.write(OUT,'native-result.json',result)
raise SystemExit(0 if result['passed'] and result['boot_unchanged'] else 1)
