"""Whole legacy D-Bus helper bodies under a private connection-provider seam."""
from pathlib import Path
import json,re,shutil,sys,types
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/windows-workspace-preflight-074'))
import prepare as p
parent=p.load(ROOT/'tools/dbus-source-package-074/run.py','legacy_package_parent')
sha,files,write=parent.sha,parent.files,parent.write
BASE=ROOT/'target/dbus-legacy-calls-20261028/fixed-sender'
NATIVE='/home/artur/.cache/hydracache-074-dbus-legacy-calls/20261028-a4'
SOURCE_SHA='4b8f35e341b489f5928206b2df3f09525abb081b937e35cefd5c1ca1bad01cac'
NAMES=('start_transient_unit','inspect_unit','inspect_unit_optional','stop_unit_and_wait','reset_failed_unit_and_wait','inspect_loaded_unit','dbus_properties','validate_unit_name')

def projection(raw,native):
 p.require(sha(raw)==SOURCE_SHA,'whole original legacy caller')
 text=raw.decode();chunks={}
 chunks['types']=text[text.index('#[derive(Debug, Clone, PartialEq, Eq)]\npub struct ExecCommand'):text.index('pub fn build_transient_unit_spec(')]
 chunks['dispatch_and_wait']=text[text.index('pub fn start_transient_unit('):text.index('pub fn verify_unit_terminal(')]
 chunks['inspect_and_properties']=text[text.index('fn inspect_loaded_unit('):text.index('fn role_runtime_seconds(')]
 chunks['namespace']=text[text.index('fn validate_unit_name('):text.index('\n#[cfg(test)]\nmod tests {')]
 constants=''.join(line+'\n' for line in text.splitlines() if line.startswith(('const SYSTEMD_','const UNIT_PREFIX:')))
 header='''//! Exact legacy helpers, not authenticated original-start or host authority.
use crate::bridge::Proxy;
use serde::{Serialize,Deserialize};
use thiserror::Error;
use std::time::{Duration,Instant};
use zbus::{OwnedObjectPath,Variant as Value};
'''
 route='\n#[path='+json.dumps(native+'/assets/tools/dbus-legacy-calls-074/fixture.rs')+']\nmod fixture;\nuse fixture::Connection;\n'
 body=header+constants+'\n'.join(chunks.values())+route
 for name in NAMES:p.require(len(re.findall(r'fn '+name+r'\(',body))==1,'one whole helper '+name)
 p.require(body.count('Connection::system()?')==4,'all four original connection acquisition sites')
 return body.encode(),{name:body.encode() for name,body in chunks.items()}

def prepare():
 first=json.loads((ROOT/'target/dbus-legacy-calls-20261028/session/fd-194-test/result.json').read_bytes())
 p.require(first['exit']==101 and first['accounting']['passed']==68 and first['accounting']['failed']==10,'first full 78-case refusal retained')
 observed=ROOT/'target/dbus-legacy-calls-20261028/observed/session/fd-194-test'
 p.require(json.loads((observed/'result.json').read_bytes())['accounting']['failed']==10 and b'Invalid unique name' in (observed/'stdout.log').read_bytes(),'diagnostic first sender refusal retained')
 partial=ROOT/'target/dbus-legacy-calls-20261028/fixed/session/fd-194-test'
 p.require(json.loads((partial/'result.json').read_bytes())['accounting']['failed']==10,'partial sender fixture repair retained')
 out=BASE/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 get=parent.held('dbus-source-package-20261028','84008a428497fcc0445527c86c7576fd94bb12c742aa667f03e81f4acdebd227')
 review=json.loads(get('capture/prepare/result.json'));source=ROOT/'target/dbus-source-package-20261028/input'
 p.require(files(source)==review['files'] and parent.parent.fragment.prior.parent.old.root_inputs()==review['root_inputs'],'exact previous package consumer and product')
 raw=(ROOT/'tools/long-run-supervisor-074/src/systemd_unit.rs').read_bytes()
 projected,chunks=projection(raw,NATIVE);(out/'systemd_unit.rs').write_bytes(raw)
 for name,body in chunks.items():(out/(name+'.rs')).write_bytes(body)
 dest=BASE/'input';shutil.copytree(source,dest)
 for path in dest.rglob('*'):
  if path.is_file() and 'archives' not in path.relative_to(dest).parts:path.write_bytes(path.read_bytes().replace(review['native_root'].encode(),NATIVE.encode()))
 for variant in ('baseline','fd'):
  folder=dest/variant;(folder/'probe/src/legacy_calls.rs').write_bytes(projected)
  with (folder/'probe/src/lib.rs').open('ab') as f:f.write(b'\n#[cfg(test)]\npub mod legacy_calls;\n')
  p.require((folder/'Cargo.lock').read_bytes()==(source/variant/'Cargo.lock').read_bytes(),'unchanged whole lock')
 fixture=ROOT/'tools/dbus-legacy-calls-074/fixture.rs';target=dest/'assets/tools/dbus-legacy-calls-074/fixture.rs';target.parent.mkdir(parents=True);target.write_bytes(fixture.read_bytes())
 extras=['legacy_calls::fixture::'+name for name in re.findall(r'#\[test\]\s*fn (\w+)',fixture.read_text(encoding='utf8'))]
 p.require(len(extras)==11 and len(set(extras))==11,'fixed eleven helper cases')
 write(out,'result.json',dict(review,files=files(dest),native_root=NATIVE,extras=review['extras']+extras,attempt=5,reuse_passing_baseline=True,legacy_source_sha256=SOURCE_SHA,chunk_sha256={name:sha(body) for name,body in chunks.items()},root_changed=False,release_admitted=False))
 print(json.dumps(dict(prepared=True,total_cases=78,original_connection_sites=4)))

def native_command(attempt):
 p.require(attempt==4,'single bounded context')
 return ['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-legacy-calls-074/native_fixed_sender.py','session']

def launch(worker):
 parent.reviewed_packages()
 p.require(sha((ROOT/'tools/dbus-fragments-074/run.py').read_bytes())==parent.FRAGMENT_SHA,'original CI owner')
 scope=dict(parent.parent.fragment.launch.__globals__,context=lambda attempt:(BASE,NATIVE),native_command=native_command,__file__=str(Path(__file__)))
 types.FunctionType(parent.parent.fragment.launch.__code__,scope)(4,worker)

if __name__=='__main__':
 p.require(len(sys.argv) in (2,3) and (len(sys.argv)==2 or sys.argv[2]=='--worker'),'fixed dispatch')
 if sys.argv[1]=='prepare':p.require(len(sys.argv)==2,'inert preparation');prepare()
 elif sys.argv[1]=='session-io-safe':launch(len(sys.argv)==3)
 else:raise ValueError('fixed command')
