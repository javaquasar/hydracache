"""Fixed local caller continuation; no real bus/manager or product adoption."""
from pathlib import Path
import json,os,re,shutil,subprocess,sys
ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'target/dbus-read-flows-20261028'
ORIGINAL_BASE=BASE
ENTRY_LABEL=None
sys.path.insert(0,str(ROOT/'tools/windows-workspace-preflight-074'))
import prepare as p
parent=p.load(ROOT/'tools/dbus-runtime-bridge-074/run.py','runtime_parent')
predecessor=p.load(ROOT/'tools/dbus-owner-properties-074/run.py','owner_parent')
sha,files,write,snapshot,owned=parent.sha,parent.files,parent.write,parent.snapshot,parent.owned
EXTRAS=['caller_checks::read_flows::'+n for n in ('protocol_present_reads_twice_then_owner','protocol_owner_change_refuses_after_reads',
 'protocol_unit_change_stops_before_owner_recheck','protocol_absence_and_foreign_error_keep_scope','loaded_settings_decode_and_use_unit_description',
 'loaded_missing_or_invalid_values_stop_first_pair','legacy_reader_uses_lazy_getall_snapshots','legacy_optional_absence_and_namespace_refusal','legacy_lazy_getall_failure_is_not_a_snapshot')]
PINS={'diagnostic_manager.rs':parent.OWNER_SHA,'diagnostic_loaded.rs':'dedbee622bc143a7177c7edb660d115338c14cb9e845a7c891bc086e0ae974eb',
 'systemd_unit.rs':'4b8f35e341b489f5928206b2df3f09525abb081b937e35cefd5c1ca1bad01cac'}

def source_parts():
 bodies={}
 for name,digest in PINS.items():
  raw=(ROOT/'tools/long-run-supervisor-074/src'/name).read_bytes();p.require(sha(raw)==digest,'independent source pin: '+name);bodies[name]=raw.decode()
 return bodies

def prepare():
 out=BASE/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 packet=ROOT/'docs/testing/performance/0.74/local-runs/dbus-owner-properties-20261028'
 raw=(packet/'index.json').read_bytes();p.require(sha(raw)=='6dcc7caeafe086771c60a717c27bfe3ab2091177c1866520fcc1edc25b846dfb','held predecessor packet')
 index=json.loads(raw);row=index['members']['capture/prepare/result.json']
 receipt=(packet/'objects'/row['sha256']).read_bytes();p.require(sha(receipt)==row['sha256'],'held original source receipt')
 review=json.loads(receipt);source=predecessor.BASE/'fixture'
 p.require(files(source)==review['files'] and parent.old.root_inputs()==review['root_inputs'],'exact passing prior S and protected root')
 p.require(all(sha((ROOT/n).read_bytes())==v for n,v in review['overlays'].items()),'exact held parent overlays')
 dest=BASE/'fixture';shutil.copytree(source,dest)
 bodies=source_parts();m,l,u=(bodies[n] for n in ('diagnostic_manager.rs','diagnostic_loaded.rs','systemd_unit.rs'))
 chunks={}
 chunks['observed']=m[m.index('#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct ObservedUnit'):m.index('/// An observation of a pinned manager owner')]
 chunks['unit_decoders']=m[m.index('fn read_unit('):m.index('\n#[cfg(test)]\nmod tests {')]
 chunks['loaded_types']=l[l.index('#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]'):l.index('/// New, separate wire document')]
 chunks['loaded_constants']=l[l.index('const TEXT:'):l.index('fn expected_settings(')]
 chunks['loaded_decode']=l[l.index('impl LoadedSettings {'):l.index('\n#[cfg(test)]\npub(crate) use tests::fixture')]
 chunks['protocol']=m[m.index('    let bus = proxy(',m.index('fn inspect_in_worker(')):m.index('    boot_matches(scope)?;',m.index('    let bus = proxy(',m.index('fn inspect_in_worker(')))]
 chunks['unit_types']=u[u.index('#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]'):u.index('#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum AttachableUnitState')]
 chunks['unit_error']=u[u.index('#[derive(Debug, Error)]\npub enum UnitError'):u.index('pub fn build_transient_unit_spec(')]
 chunks['legacy']=u[u.index('fn inspect_loaded_unit('):u.index('fn dbus_properties(')]
 chunks['optional']=u[u.index('pub fn inspect_unit_optional('):u.index('pub fn stop_unit_and_wait(')]
 chunks['name_validation']=u[u.index('fn validate_unit_name('):u.index('\n#[cfg(test)]\nmod tests {')]
 for name,text in chunks.items():(out/(name+'.rs')).write_text(text,encoding='utf-8',newline='\n')
 constants=''.join(line+'\n' for text in (m,u) for line in text.splitlines() if line.startswith(('const MANAGER_PATH:','const MANAGER_INTERFACE:','const UNIT_INTERFACE:','const SERVICE_INTERFACE:','const SYSTEMD_','const UNIT_PREFIX:')))
 protocol=chunks['protocol'];p.require(protocol.count('scope.unit_name()')==1,'one asserted-name test seam')
 protocol=protocol.replace('scope.unit_name()','name_input.to_owned()',1)
 protocol='fn inspect_protocol(connection: Connection, name_input: &str, loaded: bool) -> Result<(String, Option<ObservedUnit>, Option<LoadedSettings>), String> {\n'+protocol+'    Ok((owner, unit, settings))\n}\n'
 legacy=chunks['legacy'].replace('fn inspect_loaded_unit(unit_name:', 'fn inspect_loaded_unit(connection_input: Connection, unit_name:',1).replace('Connection::system()?','connection_input',1)
 optional=chunks['optional'].replace('pub fn inspect_unit_optional(unit_name:','pub fn inspect_unit_optional(connection_input: Connection, unit_name:',1).replace('inspect_loaded_unit(unit_name)','inspect_loaded_unit(connection_input, unit_name)',1)
 assembled='\nuse serde::Deserialize;\nuse std::collections::{BTreeMap,HashMap};\nuse zbus::{OwnedObjectPath,OwnedVariant as OwnedValue};\nuse thiserror::Error;\n'+constants
 for name in ('observed','unit_decoders','loaded_types','loaded_constants','loaded_decode'):
  assembled+=chunks[name].replace('use crate::diagnostic_manager::take;','use self::take;')
 assembled+=protocol+chunks['unit_types']+chunks['unit_error']+legacy+optional+chunks['name_validation']
 overlay=ROOT/'tools/dbus-read-flows-074/tests.rs'
 assembled+='\n#[cfg(test)]\n#[path = '+json.dumps(overlay.as_posix())+']\nmod read_flows;\n'
 path=dest/'probe/src/caller_checks.rs';path.write_bytes(path.read_bytes()+assembled.encode())
 lib=dest/'probe/src/lib.rs';lib.write_bytes(lib.read_bytes()+b'\n#[cfg(test)]\npub use caller_checks::{UnitSnapshot,UnitMismatch,UnitError};\n')
 manifest=dest/'probe/Cargo.toml';manifest.write_bytes(manifest.read_bytes()+b'\nserde_json = "=1.0.150"\nthiserror = "=2.0.18"\n')
 listing=index['members']['capture/list-194/result.json'];listed=(packet/'objects'/listing['sha256']).read_bytes();p.require(sha(listed)==listing['sha256'],'held parent catalog')
 write(out,'result.json',dict(files=files(dest),root_inputs=review['root_inputs'],overlays=dict(review['overlays'],
  **{overlay.relative_to(ROOT).as_posix():sha(overlay.read_bytes())}),source_pin=parent.OWNER_SHA,
  baseline_inventory=json.loads(listed)['inventory'],source_pins=PINS,chunk_sha256={n:sha(t.encode()) for n,t in chunks.items()},
  root_caller_changed=False,release_admitted=False))
 print('prepared complete selected protocol/loaded/legacy read-body shadows')

def inventory(raw,baseline):
 lines=raw.decode().splitlines();names=[n[:-6] for n in lines if n.endswith(': test')]
 p.require(len(baseline)==21 and len(set(names))==len(names)==30 and lines[-1]=='30 tests, 0 benchmarks','complete 30-case catalog')
 p.require(sorted(names)==sorted(baseline+EXTRAS),'only nine declared additions')
 return sorted(names)


PEER_CONTROL='caller_checks::read_flows::peer_mismatch_closes_connection_and_refuses'
def inventory_v3(raw,baseline):
 lines=raw.decode().splitlines();names=[n[:-6] for n in lines if n.endswith(': test')]
 p.require(len(baseline)==len(set(baseline))==21,'held unique original catalog')
 p.require(len(names)==len(set(names))==31 and lines[-1]=='31 tests, 0 benchmarks','complete 31-case catalog')
 p.require(sorted(names)==sorted(baseline+EXTRAS+[PEER_CONTROL]),'nine flows and one fail-closed peer control')
 return sorted(names)

def prepare_peer():
 out=BASE/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 previous=ROOT/'target/dbus-read-flows-v2-20261028'
 execution=json.loads((previous/'test-194/execution.json').read_bytes())
 p.require(execution['classification']=='timeout' and execution['exit']==124 and execution['retired'] and not execution['errors'],'first native timeout and original owned cleanup')
 p.require(execution['bound_seconds']==1200 and execution['cleanup_seconds']==5,'original full envelope')
 refusal=json.loads((previous/'test-194/refusal.json').read_bytes())
 p.require(refusal['reason']=='exact executed test identities','incomplete native catalog refuses')
 raw=(previous/'test-194/stdout.log').read_bytes()
 p.require(len(re.findall(rb'^test .+? \.\.\. ok$',raw,re.M))==7 and b'test result:' not in raw,'seven completed tests are not a complete gate')
 review=json.loads((previous/'prepare/result.json').read_bytes());source=previous/'fixture'
 p.require(files(source)==review['files'],'original timed-out source held unchanged')
 dest=BASE/'fixture';shutil.copytree(source,dest)
 path=dest/'probe/src/caller_checks.rs';raw=path.read_bytes()
 old=(ROOT/'tools/dbus-read-flows-074/tests.rs').as_posix().encode();overlay=ROOT/'tools/dbus-read-flows-074/tests_v3.rs'
 p.require(raw.count(old)==1,'one test route only');path.write_bytes(raw.replace(old,overlay.as_posix().encode(),1))
 write(out,'result.json',dict(review,files=files(dest),overlays=dict(review['overlays'],**{overlay.relative_to(ROOT).as_posix():sha(overlay.read_bytes())}),
  first_native_timeout_retained=True,only_test_route_changed=True,production_predicates_or_counts_changed=False))
 print('prepared v3 peer: cache owner discovery and fail-closed mismatch control')

def prepare_corrected():
 out=BASE/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 first=json.loads((ORIGINAL_BASE/'build-194/result.json').read_bytes())
 p.require(first['exit']==101 and first['unchanged'],'preserved first compile refusal')
 messages=[json.loads(n) for n in (ORIGINAL_BASE/'build-194/stdout.log').read_bytes().splitlines()]
 errors=[m['message'] for m in messages if m.get('reason')=='compiler-message' and m['message']['level']=='error']
 p.require(len(errors)==1 and errors[0]['code']['code']=='E0505','exact held-owner move error')
 review=json.loads((ORIGINAL_BASE/'prepare/result.json').read_bytes())
 source=ORIGINAL_BASE/'fixture';p.require(files(source)==json.loads((ORIGINAL_BASE/'resolve-194/result.json').read_bytes())['fixture_after'],'original refused input unchanged')
 dest=BASE/'fixture';shutil.copytree(source,dest)
 path=dest/'probe/src/caller_checks.rs';raw=path.read_bytes();before=b'    Ok((owner, unit, settings))\n';after=b'    Ok((owner.clone(), unit, settings))\n'
 p.require(raw.count(before)==1,'one test return adapter')
 path.write_bytes(raw.replace(before,after,1))
 write(out,'result.json',dict(review,files=files(dest),only_return_adapter_changed=True,first_refusal_retained=True))
 print('prepared v2: return owner clone with original proxies still held')

def run(label,worker):
 match=re.fullmatch(r'(resolve|build|list|test|check|clippy|policy)-(194|188)',label);p.require(match is not None,'fixed command')
 action,compiler=match.groups();review=json.loads((BASE/'prepare/result.json').read_bytes());folder=BASE/'fixture'
 expected=review['files'] if action=='resolve' or BASE!=ORIGINAL_BASE else json.loads((BASE/'resolve-194/result.json').read_bytes())['fixture_after']
 def guard():
  p.require(parent.old.root_inputs()==review['root_inputs'] and sha((ROOT/parent.OWNER).read_bytes())==parent.OWNER_SHA,'held root/caller')
  source_parts();p.require(files(folder)==expected,'whole frozen source/lock')
  p.require(all(sha((ROOT/n).read_bytes())==v for n,v in review['overlays'].items()),'held overlay bodies')
 identity=sha(json.dumps(dict(files=expected,overlays=review['overlays'],compiler=compiler),sort_keys=True).encode())
 target=BASE/'targets'/compiler/identity;cargo=['cargo','+1.94.0' if compiler=='194' else '+1.88.0']
 common=['--offline','--locked','--manifest-path',str(folder/'Cargo.toml'),'--target-dir',str(target),'-p','dbus-probe-074']
 artifact=None
 if action=='resolve':
  p.require(compiler=='194','one fixed offline resolution');argv=cargo+['metadata','--offline','--manifest-path',str(folder/'Cargo.toml'),'--format-version','1']
 elif action=='build':argv=cargo+['test',*common,'--lib','--no-run','--message-format=json']
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
   child=subprocess.Popen([sys.executable,'-B',str(Path(__file__)),ENTRY_LABEL,'--worker'],cwd=ROOT,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,
    env=dict(os.environ,CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='2'))
   try:code=child.wait(timeout=1210)
   except subprocess.TimeoutExpired:
    child.kill();child.wait(timeout=5);raise ValueError('worker lost; no PID fallback')
   after=files(folder)
   if action=='resolve' and code==0:p.require(set(after)==set(expected) and all(after[n]==v for n,v in expected.items() if n!='Cargo.lock'),'only fixture lock resolution')
   else:guard()
   execution=json.loads((out/'execution.json').read_bytes())
   unchanged=before==snapshot() and head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip()
   p.require(unchanged and execution['retired'] and not execution['errors'],'source/head/owned retirement')
   result=dict(exit=code,unchanged=unchanged,artifact=artifact,fixture_after=after,release_admitted=False);raw=(out/'stdout.log').read_bytes()
   if action=='build' and code==0:
    messages=[json.loads(n) for n in raw.splitlines()]
    found=[m for m in messages if m.get('reason')=='compiler-artifact' and m['target']['name']=='dbus_probe_074' and m['target']['kind']==['lib'] and m['profile']['test'] and m.get('executable')]
    p.require(len(found)==1 and [m['success'] for m in messages if m.get('reason')=='build-finished']==[True],'actual Cargo test image')
    image=Path(found[0]['executable']);p.require(image.resolve().is_relative_to(target) and Path(found[0]['target']['src_path']).resolve()==(folder/'probe/src/lib.rs').resolve(),'actual image source')
    result['artifact']=dict(path=str(image),sha256=sha(image.read_bytes()),bytes=image.stat().st_size)
   if artifact:p.require(sha(Path(artifact['path']).read_bytes())==artifact['sha256'],'image postflight')
   if action=='list' and code==0:result['inventory']=(inventory_v3 if BASE==ROOT/'target/dbus-read-flows-v3-20261028' else inventory)(raw,review['baseline_inventory'])
   if action=='test':
    a=p.load(ROOT/'tools/chitchat-mtu-review-074/accounting.py','owner_account')
    result['accounting']=a.account(raw,listing['inventory'],'',False);p.require((code==0)==result['accounting']['test_pass'],'native exit/count agreement')
   write(out,'result.json',result)
  except BaseException as error:
   write(out,'refusal.json',dict(type=type(error).__name__,reason=str(error),release_admitted=False));raise
 print(json.dumps(dict(label=label,exit=code,unchanged=unchanged)));raise SystemExit(code)

if __name__=='__main__':
 p.require(len(sys.argv) in (2,3) and (len(sys.argv)==2 or sys.argv[2]=='--worker'),'fixed invocation')
 ENTRY_LABEL=sys.argv[1];label=ENTRY_LABEL
 if label.startswith('v3-'):
  BASE=ROOT/'target/dbus-read-flows-v3-20261028';label=label[3:]
  p.require(label!='resolve-194','v3 keeps original resolved graph')
  if label=='prepare':p.require(len(sys.argv)==2,'inert prepare');prepare_peer()
  else:run(label,len(sys.argv)==3)
 elif label.startswith('v2-'):
  BASE=ROOT/'target/dbus-read-flows-v2-20261028';label=label[3:]
  p.require(label!='resolve-194','v2 keeps original resolved graph')
  if label=='prepare':p.require(len(sys.argv)==2,'inert prepare');prepare_corrected()
  else:run(label,len(sys.argv)==3)
 elif label=='prepare':p.require(len(sys.argv)==2,'inert prepare');prepare()
 else:run(label,len(sys.argv)==3)
