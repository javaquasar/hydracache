"""Exact read-only fixed channel/kernel compile slice; no host channel is opened."""
from pathlib import Path
import json,re,shutil,sys,types,tomllib
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/windows-workspace-preflight-074'))
import prepare as p
prior=p.load(ROOT/'tools/dbus-legacy-calls-074/run.py','fixed_legacy_parent')
syntax=prior.parent.parent
sha,files,write=prior.sha,prior.files,prior.write
BASE=ROOT/'target/dbus-fixed-channel-20261028/api-fixed'
NATIVE='/home/artur/.cache/hydracache-074-dbus-fixed-channel/20261028-a2'
SOURCE=ROOT/'tools/long-run-supervisor-074/src'
NAMES=('diagnostic_manager_channel.rs','diagnostic_manager_channel_kernel.rs','diagnostic_manager_channel_tests.rs','diagnostic_manager_channel_kernel_tests.rs','diagnostic_manager_alarm_tests.rs','diagnostic_manager_sandbox_query.rs')
PINS={'diagnostic_manager_channel.rs': '1110a742d1806279a69a568a70399f9a0dfce9868ccd9da70550005c4d81c27d', 'diagnostic_manager_channel_kernel.rs': '2eefec1c52c06d42e88e4b7ddfdaa70593ecbedc83a6c8b95fc1037a8726164c', 'diagnostic_manager_channel_tests.rs': '3d6567947ff6073ea76f95bbf8916a91f3d7d6560a22ad51686208d1259cb3f6', 'diagnostic_manager_channel_kernel_tests.rs': '6ed19516345e555b556817a2727b4c5d30ce10dc73c59ab8b78f91375001cd66', 'diagnostic_manager_alarm_tests.rs': 'a60ac17ff7b5404d21d3e6283ec11dcb323a179676cf465f3b7acd425ed43ccc', 'diagnostic_manager_sandbox_query.rs': 'f8f52fa47e09d9b7ee977c3595cf2269ed80228a0df24bb566d300a50a2612ba'}

def project(name,raw):
 p.require(sha(raw)==PINS[name],'whole original source: '+name)
 # Reuse the tested, call-scoped builder spelling converter under an exact pin.
 # Its per-file authority is private to this isolated imported module.
 syntax.PINS[name]=PINS[name]
 adapted,count=syntax.project(name,raw)
 text=adapted.decode();excluded=''
 if name=='diagnostic_manager_channel.rs':
  for child in ('original_join','held_delivery'):
   filename={'original_join':'diagnostic_original_manager_join.rs','held_delivery':'diagnostic_manager_held_delivery.rs'}[child]
   route='#[path = "'+filename+'"]\nmod '+child+';\n'
   p.require(text.count(route)==1,'explicit omitted original-frame child');text=text.replace(route,'',1)
  p.require(text.count('Builder::unix_stream(stream)')==1,'one Tokio ownership transfer')
  text=text.replace('zbus::AuthMechanism::External','zbus::connection::AuthMechanism::External')
  text=text.replace('Builder::unix_stream(stream)','Builder::unix_stream(io(stream.into_std())?)',1)
  p.require(text.count('.max_queued(16)')==1,'original queue bound')
  text=text.replace('.max_queued(16)','.max_queued(std::num::NonZeroUsize::new(16).unwrap())',1)
  text+='\n#[cfg(test)]\n#[path = '+json.dumps(NATIVE+'/assets/tools/dbus-fixed-channel-074/adaptation_tests.rs')+']\nmod adaptation_tests;\n'
 elif name=='diagnostic_manager_channel_kernel.rs':
  route='#[path = "diagnostic_manager_receiver_pin.rs"]\npub(super) mod receiver;\n'
  p.require(text.count(route)==1,'explicit omitted receiver join');text=text.replace(route,'',1)
 elif name=='diagnostic_manager_channel_tests.rs':
  marker='#[test]\n#[ignore = "manual local nonroot fixed manager namespace-access refusal; no mutation"]'
  p.require(text.count(marker)==1,'two manual host tests remain outside no-bus slice')
  text,excluded=text.split(marker,1);excluded=marker+excluded
 elif name=='diagnostic_manager_sandbox_query.rs':
  text=text.replace('use zbus::wire::{OwnedObjectPath, OwnedValue};','use zbus::{OwnedObjectPath, OwnedVariant as OwnedValue};')
 return text.encode(),count,excluded.encode()

def prepare():
 first=ROOT/'target/dbus-fixed-channel-20261028/session/fd-194-build'
 p.require(json.loads((first/'result.json').read_bytes())['exit']==101,'first compile refusal retained')
 messages=[json.loads(v) for v in (first/'stdout.log').read_bytes().splitlines()]
 errors=[v['message'] for v in messages if v.get('reason')=='compiler-message' and v['message']['level']=='error']
 p.require(len(errors)==2 and all(v['code']['code']=='E0433' for v in errors),'exact two missing API/features')
 out=BASE/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 get=prior.parent.held('dbus-legacy-calls-20261028','eb82212e5e93a10017712b4499ad4b288e8e1018801aa0d64b9184a04e3417f6')
 review=json.loads(get('capture/final/prepare/result.json'));source=ROOT/'target/dbus-legacy-calls-20261028/fixed-sender/input'
 p.require(files(source)==review['files'] and prior.parent.parent.fragment.prior.parent.old.root_inputs()==review['root_inputs'],'exact prior consumer/product')
 dest=BASE/'input';shutil.copytree(source,dest)
 for path in dest.rglob('*'):
  if path.is_file() and 'archives' not in path.relative_to(dest).parts:path.write_bytes(path.read_bytes().replace(review['native_root'].encode(),NATIVE.encode()))
 assets=dest/'assets/tools/dbus-fixed-channel-074';assets.mkdir(parents=True)
 extras=[];projection={}
 for name in NAMES:
  raw=(SOURCE/name).read_bytes();adapted,count,excluded=project(name,raw)
  (out/name).write_bytes(raw);(assets/name).write_bytes(adapted)
  if excluded:(out/'manual-host-tests-not-executed.rs').write_bytes(excluded)
  projection[name]=dict(original_sha256=sha(raw),projected_sha256=sha(adapted),builder_unwraps_removed=count)
  route={'diagnostic_manager_channel_tests.rs':'tests','diagnostic_manager_channel_kernel_tests.rs':'kernel::tests','diagnostic_manager_alarm_tests.rs':'alarm_tests','diagnostic_manager_sandbox_query.rs':'sandbox_query::tests'}.get(name)
  if route:extras+=['fixed_channel::'+route+'::'+n for n in re.findall(r'#\[test\]\s*fn (\w+)',adapted.decode())]
 overlay=ROOT/'tools/dbus-fixed-channel-074/adaptation_tests.rs';body=overlay.read_bytes();(assets/overlay.name).write_bytes(body)
 extras+=['fixed_channel::adaptation_tests::'+n for n in re.findall(r'#\[test\]\s*fn (\w+)',body.decode())]
 p.require(len(extras)==17 and len(set(extras))==17,'five channel/four kernel/four alarm/two sandbox/two adapter cases')
 for variant in ('baseline','fd'):
  lib=dest/variant/'probe/src/lib.rs'
  with lib.open('ab') as f:f.write(('\n#[cfg(all(test,target_os="linux"))]\n#[path='+json.dumps(NATIVE+'/assets/tools/dbus-fixed-channel-074/diagnostic_manager_channel.rs')+']\nmod fixed_channel;\n').encode())
  manifest=dest/variant/'probe/Cargo.toml';before=manifest.read_bytes()
  old=b'features = ["rt", "time"]';p.require(before.count(old)==1,'one direct Tokio feature')
  manifest.write_bytes(before.replace(old,b'features = ["rt", "time", "macros"]',1))
  lock=dest/variant/'Cargo.lock';before=lock.read_bytes();nodes=tomllib.loads(before.decode())['package']
  p.require(not any(n['name']=='tokio-macros' for n in nodes),'new fixture-only macro node')
  a=before.index(b'name = "tokio"');b=before.index(b'\n[[package]]',a)
  chunk=before[a:b];p.require(chunk.count(b' "windows-sys",')==1,'exact Tokio edge anchor')
  chunk=chunk.replace(b' "windows-sys",',b' "tokio-macros",\n "windows-sys",',1)
  node=b'\n[[package]]\nname = "tokio-macros"\nversion = "2.7.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "385a6cb71ab9ab790c5fe8d67f1645e6c450a7ce006a33de03daa956cf70a496"\ndependencies = [\n "proc-macro2",\n "quote",\n "syn 2.0.119",\n]\n'
  after=before[:a]+chunk+node+before[b:];updated=tomllib.loads(after.decode())['package']
  p.require(len(updated)==len(nodes)+1 and [n for n in updated if n['name'] not in ('tokio','tokio-macros')]==[n for n in nodes if n['name']!='tokio'],'only Tokio macro edge/node')
  lock.write_bytes(after)
 macro_dir=ROOT/'target/dbus-fixed-macros-20261028';archive=(macro_dir/'tokio-macros-2.7.0.crate').read_bytes()
 sparse=json.loads((macro_dir/'sparse-row.json').read_bytes());p.require(sha(archive)==sparse['cksum']=='385a6cb71ab9ab790c5fe8d67f1645e6c450a7ce006a33de03daa956cf70a496' and sparse['vers']=='2.7.0','held primary checksum')
 product=tomllib.loads((ROOT/'Cargo.lock').read_text(encoding='utf8'))['package'];p.require([n['checksum'] for n in product if n['name']=='tokio-macros' and n['version']=='2.7.0']==[sha(archive)],'exact product macro identity')
 (dest/'archives/tokio-macros-2.7.0.crate').write_bytes(archive)
 (out/'tokio-macros-sparse-row.json').write_bytes((macro_dir/'sparse-row.json').read_bytes())
 write(out,'projection.json',projection)
 write(out,'result.json',dict(review,files=files(dest),native_root=NATIVE,registry=review['registry']+[dict(name='tokio-macros-2.7.0',sha256=sha(archive),bytes=len(archive))],extras=review['extras']+extras,attempt=5,reuse_passing_baseline=True,fixed_channel_source_pins=PINS,host_tests_executed=False,original_frame_join_included=False,root_changed=False,release_admitted=False))
 print(json.dumps(dict(prepared=True,total_tests=95,additional_original_tests=15,additional_adapter_tests=2)))

def native_command(attempt):
 p.require(attempt==4,'single fixed context')
 return ['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-fixed-channel-074/native_api_fixed.py','session']

def launch(worker):
 prior.parent.reviewed_packages()
 p.require(sha((ROOT/'tools/dbus-fragments-074/run.py').read_bytes())==prior.parent.FRAGMENT_SHA,'same parent owner')
 scope=dict(prior.parent.parent.fragment.launch.__globals__,context=lambda attempt:(BASE,NATIVE),native_command=native_command,__file__=str(Path(__file__)))
 types.FunctionType(prior.parent.parent.fragment.launch.__code__,scope)(4,worker)

if __name__=='__main__':
 p.require(len(sys.argv) in (2,3) and (len(sys.argv)==2 or sys.argv[2]=='--worker'),'fixed dispatch')
 if sys.argv[1]=='prepare':p.require(len(sys.argv)==2,'inert preparation');prepare()
 elif sys.argv[1]=='session-io-safe':launch(len(sys.argv)==3)
 else:raise ValueError('fixed command')
