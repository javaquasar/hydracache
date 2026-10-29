"""Full real supervisor type closure in an unpublished, two-major compile fixture."""
from pathlib import Path
import json,re,shutil,subprocess,sys,tomllib
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/windows-workspace-preflight-074'))
import prepare as p
fixed=p.load(ROOT/'tools/dbus-fixed-channel-074/run.py','owner_fixed_parent')
prior=fixed.prior.parent.parent.fragment.prior
sha,files,write=fixed.sha,fixed.files,fixed.write
BASE=ROOT/'target/dbus-original-owner-20261028'
NATIVE='/home/artur/.cache/hydracache-074-dbus-original-owner/20261028-a1'
PKG='tools/long-run-supervisor-074'
JOIN_SHA='617bf72f9473324b47854a428bb6cdb7a462451120f4e4ed480f651bba4a0a47'
FRAME_SHA='e670d7a59ba74d23cbb663f13924edb6c749eb7287c2d90d6cbbae76f4b3e876'

def value(v):
 if isinstance(v,str):return json.dumps(v,ensure_ascii=False)
 if isinstance(v,bool):return str(v).lower()
 if isinstance(v,list):return '['+', '.join(value(x) for x in v)+']'
 if isinstance(v,dict):return '{ '+', '.join(k+' = '+value(x) for k,x in v.items())+' }'
 return str(v)

def workspace():
 root=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf8'))
 spec=tomllib.loads((ROOT/PKG/'Cargo.toml').read_text(encoding='utf8'))
 wanted=set()
 def visit(v):
  if isinstance(v,dict):
   for k,item in v.items():
    if isinstance(item,dict) and item.get('workspace') is True and k not in ('edition','license'):wanted.add(k)
    else:visit(item)
 visit(spec)
 wanted.discard('lints')
 p.require(wanted<=root['workspace']['dependencies'].keys(),'only original inherited dependencies')
 text='[workspace]\nmembers = ["'+PKG+'"]\nexclude = ["source/zbus", "source/zbus_macros", "source/zbus_utils"]\nresolver = "2"\n\n[workspace.package]\n'
 text+=''.join(k+' = '+value(v)+'\n' for k,v in root['workspace']['package'].items())
 text+='\n[workspace.dependencies]\n'+''.join(k+' = '+value(root['workspace']['dependencies'][k])+'\n' for k in sorted(wanted))
 text+='\n[workspace.lints.rust]\n'+''.join(k+' = '+value(v)+'\n' for k,v in root['workspace']['lints']['rust'].items())
 return text

def join_projection(raw):
 p.require(sha(raw)==JOIN_SHA,'whole original join source')
 text=raw.decode()
 p.require(text.count('bind_fixed_manager(')==2,'constructor and its test signature')
 text=text.replace('bind_fixed_manager(','bind_fixed_manager_v6_compile_gate(')
 # A second copy of test APIs would add duplicate inherent methods to the same
 # actual frame. The original full test sources remain unchanged in baseline.
 start=text.index('#[cfg(test)]\n#[test]')
 text=text[:start]
 text+='''
// Unnamed constants retain private type/function reachability without runtime IO.
const _: fn(OriginalEnrollmentFrame<'static,'static,'static,'static,'static,'static,'static,'static,'static,'static>) = |frame| { let _ = frame.bind_fixed_manager_v6_compile_gate(); };
const _: fn(&mut ManagerJoinFailure<'static,'static,'static,'static,'static,'static,'static,'static,'static,'static>) = |failure| { let _ = &failure.held; };
const _: fn(&mut OriginalManagerJoin<'static,'static,'static,'static,'static,'static,'static,'static,'static,'static>) = |join| { let _ = join.revalidate(); };
const _: fn(&FixedManagerChannel,&str,&str)->std::result::Result<(),JoinError> = <FixedManagerChannel as ChannelChecks>::require_names;
'''
 return text.encode()

def prepare():
 out=BASE/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 original={}
 listed=subprocess.check_output(['git','ls-files','-z','--',PKG],cwd=ROOT).split(b'\0')
 for name in listed:
  if not name:continue
  text=name.decode();path=ROOT/text;p.require(path.is_file() and not path.is_symlink(),'tracked regular source');original[text]=path.read_bytes()
 p.require(sum(n.startswith(PKG+'/src/') and n.endswith('.rs') for n in original)==545,'complete original physical Rust inventory')
 baseline=BASE/'input/baseline';baseline.mkdir(parents=True)
 for name,raw in original.items():path=baseline/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
 (baseline/'Cargo.toml').write_text(workspace(),encoding='utf8',newline='\n')
 (baseline/'Cargo.lock').write_bytes((ROOT/'Cargo.lock').read_bytes())
 # Copy every literal include file without changing any include spelling. Inline
 # cfg(test) declarations remain data for this production-library compile gate.
 includes={}
 for name,raw in original.items():
  if not name.endswith('.rs'):continue
  for literal in re.findall(r'include_(?:str|bytes)!\(\s*"([^"\n]+)"',raw.decode()):
   path=(ROOT/name).parent/literal;path=path.resolve()
   p.require(path.is_relative_to(ROOT) and path.is_file(),'original literal include file')
   rel=path.relative_to(ROOT).as_posix();includes[rel]=path.read_bytes()
 for name,raw in includes.items():path=baseline/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
 write(out,'result.json',dict(baseline_files=files(baseline),original_sources={n:sha(v) for n,v in original.items()},include_sources={n:sha(v) for n,v in includes.items()},root_inputs=fixed.prior.parent.parent.fragment.prior.parent.old.root_inputs(),library_only=True,tests_executed=False,services=False,server=False,root_changed=False,release_admitted=False))
 print(json.dumps(dict(prepared=True,original_rust_files=545,include_files=len(includes))))

def candidate():
 out=BASE/'candidate';out.mkdir(exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 resolution=json.loads((BASE/'resolve-baseline/result.json').read_bytes());p.require(resolution['exit']==0 and resolution['root_unchanged'],'full original metadata baseline')
 source=BASE/'input/baseline';p.require(files(source)==resolution['files_after'],'resolved exact baseline')
 dest=BASE/'input/fd';shutil.copytree(source,dest)
 # Canonical zbus 6 source package bodies, unchanged from their consumer gate.
 _,_,libraries=fixed.prior.parent.reviewed_packages()
 for crate,members in libraries.items():
  for name,raw in members.items():path=dest/'source'/crate/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
 with (dest/'Cargo.toml').open('a',encoding='utf8',newline='\n') as f:f.write('\n[patch.crates-io]\nzbus_macros = { path = "source/zbus_macros" }\nzbus_utils = { path = "source/zbus_utils" }\n')
 manifest=dest/PKG/'Cargo.toml'
 with manifest.open('a',encoding='utf8',newline='\n') as f:f.write('\n[target.\'cfg(target_os = "linux")\'.dependencies.zbus6]\npackage = "zbus"\npath = "../../source/zbus"\ndefault-features = false\nfeatures = ["tokio-multithread", "proxy", "p2p"]\n')
 directory=dest/PKG/'src'
 frame=directory/'diagnostic_original_enrollment_frame.rs';raw=frame.read_bytes();p.require(sha(raw)==FRAME_SHA,'whole actual original frame')
 frame.write_bytes(raw+b'\n#[path = "diagnostic_manager_channel_v6.rs"]\nmod manager_channel_v6;\n')
 original=(fixed.SOURCE/'diagnostic_manager_channel.rs').read_bytes()
 core=fixed.project('diagnostic_manager_channel.rs',original)[0].decode()
 route='\n#[cfg(test)]\n#[path = '+json.dumps(fixed.NATIVE+'/assets/tools/dbus-fixed-channel-074/adaptation_tests.rs')+']\nmod adaptation_tests;\n'
 p.require(core.count(route)==1,'only prior adapter test route');core=core.replace(route,'',1)
 core=core.replace('zbus::','zbus6::').replace('"diagnostic_manager_channel_kernel.rs"','"diagnostic_manager_channel_kernel_v6.rs"').replace('"diagnostic_manager_sandbox_query.rs"','"diagnostic_manager_sandbox_query_v6.rs"')
 core+='\n#[path = "diagnostic_original_manager_join_v6.rs"]\nmod original_join;\nconst _: fn(&mut FixedManagerChannel,&str,&str)->std::result::Result<(u32,[u8;16],String),()> = FixedManagerChannel::observe_sandbox;\n'
 (directory/'diagnostic_manager_channel_v6.rs').write_text(core,encoding='utf8',newline='\n')
 (directory/'diagnostic_manager_channel_kernel_v6.rs').write_bytes((fixed.SOURCE/'diagnostic_manager_channel_kernel.rs').read_bytes())
 sandbox=fixed.project('diagnostic_manager_sandbox_query.rs',(fixed.SOURCE/'diagnostic_manager_sandbox_query.rs').read_bytes())[0].replace(b'zbus::',b'zbus6::')
 (directory/'diagnostic_manager_sandbox_query_v6.rs').write_bytes(sandbox)
 (directory/'diagnostic_original_manager_join_v6.rs').write_bytes(join_projection((fixed.SOURCE/'diagnostic_original_manager_join.rs').read_bytes()))
 write(out,'result.json',dict(files=files(dest),two_majors_fixture_only=True,actual_original_frame=True,actual_kernel_receiver_binding=True,held_delivery_included=False,library_only=True,tests_executed=False,root_changed=False,release_admitted=False))
 print('prepared full actual frame/receiver type-closure candidate; no context stubs')

def lint_fixed():
 first=json.loads((BASE/'session/native-result.json').read_bytes())
 p.require(not first['passed'] and first['source_unchanged'] and first['boot_unchanged'],'first source-stable refusal retained')
 p.require(first['phases']==[dict(label=v,exit=0) for v in ('baseline-194-check','baseline-194-clippy','baseline-188-check','baseline-188-clippy','fd-194-check')]+[dict(label='fd-194-clippy',exit=101)],'exact first compile/clippy sequence')
 errors=(BASE/'session/fd-194-clippy/stderr.log').read_text(encoding='utf8')
 p.require(errors.count('\nerror: ')==3 and 'clippy::duplicate_mod' in errors and 'clippy::type_complexity' in errors,'exact two fixture lint failures, no exemption')
 previous=json.loads((BASE/'freeze/result.json').read_bytes())
 p.require(files(BASE/'input')==previous['files'],'unchanged first frozen fixture')
 destbase=BASE/'lint-fixed';out=destbase/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 dest=destbase/'input';shutil.copytree(BASE/'input',dest)
 native=NATIVE.replace('-a1','-a2')
 for variant in ('baseline','fd'):
  path=dest/variant/'.cargo/config.toml';raw=path.read_bytes();p.require(raw.count(NATIVE.encode())==1,'only vendor root relocation');path.write_bytes(raw.replace(NATIVE.encode(),native.encode(),1))
 directory=dest/'fd'/PKG/'src'
 kernel=directory/'diagnostic_manager_channel_kernel_v6.rs';raw=kernel.read_bytes()
 old=b'#[path = "diagnostic_manager_receiver_pin.rs"]';new=b'#[path = "diagnostic_manager_receiver_pin_v6.rs"]'
 p.require(raw.count(old)==1,'duplicate module route only');kernel.write_bytes(raw.replace(old,new,1))
 receiver=(fixed.SOURCE/'diagnostic_manager_receiver_pin.rs').read_bytes();(directory/'diagnostic_manager_receiver_pin_v6.rs').write_bytes(receiver)
 core=directory/'diagnostic_manager_channel_v6.rs';text=core.read_text(encoding='utf8')
 old='const _: fn(&mut FixedManagerChannel,&str,&str)->std::result::Result<(u32,[u8;16],String),()> = FixedManagerChannel::observe_sandbox;'
 new='const _: fn(&mut FixedManagerChannel, &str, &str) = |channel, unit, cgroup| { let _ = channel.observe_sandbox(unit, cgroup); };'
 p.require(text.count(old)==1,'compile-only reachability signature');core.write_text(text.replace(old,new,1),encoding='utf8',newline='\n')
 write(out,'result.json',dict(previous,files=files(dest),native_root=native,reuse_passing_baseline=True,first_failure_retained=True,kernel_body_equal_except_receiver_path=True,receiver_body_equal=True,no_lint_exemptions=True))
 print('prepared distinct receiver module identity and simple compile-only anchor; first refusal unchanged')

if __name__=='__main__':
 p.require(len(sys.argv)==2 and sys.argv[1] in ('prepare','candidate','lint-fixed'),'fixed preparation')
 {'prepare':prepare,'candidate':candidate,'lint-fixed':lint_fixed}[sys.argv[1]]()
