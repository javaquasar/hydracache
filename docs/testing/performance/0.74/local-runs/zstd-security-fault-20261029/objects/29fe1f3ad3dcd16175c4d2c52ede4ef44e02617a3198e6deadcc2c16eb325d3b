"""One original message graph on zbus 6; isolated unpublished supervisor copy."""
from pathlib import Path
import importlib.util,json,re,shutil,sys
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('delivery_original_parent',ROOT/'tools/dbus-original-owner-074/prepare.py');parent=importlib.util.module_from_spec(spec);spec.loader.exec_module(parent)
p,prior,sha,files,write=parent.p,parent.prior,parent.sha,parent.files,parent.write
fixed=parent.fixed
BASE=ROOT/'target/dbus-original-delivery-20261028'
NATIVE='/home/artur/.cache/hydracache-074-dbus-original-delivery/20261028-a1'
PKG=parent.PKG
PINS=json.loads((Path(__file__).parent/'source-pins.json').read_bytes())
LEGACY={'diagnostic_manager.rs','diagnostic_loaded.rs','systemd_unit.rs'}

def project(name,raw,*,v2=False):
 p.require(sha(raw)==PINS[name],'whole original caller pin: '+name)
 text=raw.decode()
 if name in LEGACY:return raw
 # Same call-scoped spelling changes as the independently tested ReplyState
 # projection, without excluding any original module or test route.
 text=text.replace('zbus::zvariant::','zbus::wire::').replace('zvariant::{','wire::{')
 text=text.replace('use zbus::wire::Value;','use zbus::wire::Variant as Value;')
 text=text.replace('{Fd, Value}','{Fd, Variant as Value}').replace('{Fd, OwnedObjectPath, Value}','{Fd, OwnedObjectPath, Variant as Value}')
 text=text.replace('{OwnedObjectPath, OwnedValue}','{OwnedObjectPath, OwnedVariant as OwnedValue}')
 for a,b in {'NoReplyExpected':'NO_REPLY_EXPECTED','NoAutoStart':'NO_AUTO_START','AllowInteractiveAuth':'ALLOW_INTERACTIVE_AUTH'}.items():text=text.replace('Flags::'+a,'Flags::'+b)
 for flag in ('NO_REPLY_EXPECTED','NO_AUTO_START','ALLOW_INTERACTIVE_AUTH'):text=text.replace('Flags::'+flag+' as u8','Flags::'+flag+'.bits()')
 # Remove only the result operator on a builder setter/constructor. Fallible
 # build, body decoding and every surrounding validation remain in place.
 pattern=r'"(?:\\.|[^"\\])*"|[A-Za-z_][A-Za-z_0-9]*|[^\s]'
 if v2:pattern=r'//[^\n]*|/\*.*?\*/|(?:br|r)(?P<hash>#{0,8})".*?"(?P=hash)|b?"(?:\\.|[^"\\])*"|b?\'(?:\\.|[^\'\\])\'|[A-Za-z_][A-Za-z_0-9]*|[^\s]'
 tokens=[m for m in re.finditer(pattern,text,re.S if v2 else 0) if not v2 or not m.group().startswith(('//','/*'))]
 setters={'method_call','method_return','error','destination','sender','interface','path','member','with_flags'};edits=[]
 for i,token in enumerate(tokens):
  unwrap=token.group()=='unwrap' and i>=2 and [v.group() for v in tokens[i-1:i+3]]==['.','unwrap','(',')']
  question=token.group()=='?' and i>=1
  close=i-2 if unwrap else i-1
  if not (unwrap or question) or tokens[close].group()!=')':continue
  depth=1;j=close-1
  while j>=0 and depth:
   depth+=(tokens[j].group()==')')-(tokens[j].group()=='(');j-=1
  if j>=0 and tokens[j].group() in setters and (not v2 or j+2<close):edits.append((tokens[i-1].start(),tokens[i+2].end()) if unwrap else (token.start(),token.end()))
 for a,b in reversed(edits):text=text[:a]+text[b:]
 if name=='diagnostic_held_delivery_wire.rs':
  old='''Message::method_call(PATH, "StartTransientUnit")
        .and_then(|b| b.destination(manager))
        .and_then(|b| b.interface(INTERFACE))
        .and_then(|b| b.serial(serial).build(&body))'''
  new='''Message::method_call(PATH, "StartTransientUnit")
        .destination(manager)
        .interface(INTERFACE)
        .serial(serial).build(&body)'''
  p.require(text.count(old)==1,'one held builder chain');text=text.replace(old,new,1)
 if name=='diagnostic_manager_channel.rs':
  p.require(text.count('Builder::unix_stream(stream)')==1 and text.count('.max_queued(16)')==1,'fixed channel adapter sites')
  text=text.replace('Builder::unix_stream(stream)','Builder::unix_stream(io(stream.into_std())?)',1).replace('.max_queued(16)','.max_queued(std::num::NonZeroUsize::new(16).unwrap())',1).replace('zbus::AuthMechanism::External','zbus::connection::AuthMechanism::External')
 # A lexical alias selects one Message/FD/wire implementation for this entire
 # original tree. Legacy helpers remain separate; there is no message bridge.
 lines=text.splitlines(True);at=0
 while at<len(lines) and lines[at].startswith('//!'):at+=1
 lines.insert(at,'use zbus6 as zbus;\n')
 return ''.join(lines).encode()

def prepare():
 out=BASE/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 previous=parent.BASE/'lint-fixed';review=json.loads((previous/'prepare/result.json').read_bytes())
 final=json.loads((previous/'session/result.json').read_bytes());p.require(final['exit']==0 and final['source_unchanged'],'passing real-owner predecessor')
 p.require(files(previous/'input')==review['files'],'whole predecessor frozen inputs')
 p.require(fixed.prior.parent.parent.fragment.prior.parent.old.root_inputs()==review['root_inputs'],'unchanged protected roots')
 dest=BASE/'input';shutil.copytree(previous/'input',dest)
 # Restore the whole genuine supervisor tree, removing only the five generated
 # predecessor modules from this NEW disposable copy, never the prior evidence.
 src=dest/'fd'/PKG/'src';baseline=dest/'baseline'/PKG/'src'
 generated=set(files(src))-set(files(baseline));p.require(generated=={'diagnostic_manager_channel_v6.rs','diagnostic_manager_channel_kernel_v6.rs','diagnostic_manager_sandbox_query_v6.rs','diagnostic_original_manager_join_v6.rs','diagnostic_manager_receiver_pin_v6.rs'},'exact prior generated module inventory')
 for name in generated:(src/name).unlink()
 for name in files(baseline):(src/name).write_bytes((baseline/name).read_bytes())
 current={path.name for path in fixed.SOURCE.glob('*.rs') if 'zbus::' in path.read_text(encoding='utf8')};p.require(current==set(PINS),'closed original direct-caller census')
 changes={}
 for name in sorted(PINS):
  raw=(fixed.SOURCE/name).read_bytes();adapted=project(name,raw);p.require((src/name).read_bytes()==raw,'genuine original caller');(src/name).write_bytes(adapted)
  changes[name]=dict(original=sha(raw),candidate=sha(adapted),legacy=name in LEGACY)
 for variant in ('baseline','fd'):
  path=dest/variant/'.cargo/config.toml';raw=path.read_bytes();p.require(raw.count(review['native_root'].encode())==1,'one vendor relocation');path.write_bytes(raw.replace(review['native_root'].encode(),NATIVE.encode(),1))
 p.require((dest/'baseline/Cargo.lock').read_bytes()==(previous/'input/baseline/Cargo.lock').read_bytes() and (dest/'fd/Cargo.lock').read_bytes()==(previous/'input/fd/Cargo.lock').read_bytes(),'unchanged dependency locks')
 write(out,'projection.json',changes)
 receipt=dict(review,files=files(dest),native_root=NATIVE,reuse_passing_baseline=True,held_delivery_included=True,message_owner_included=True,one_original_message_graph=True,root_changed=False,release_admitted=False)
 freeze=BASE/'freeze';freeze.mkdir();write(freeze,'result.json',receipt);write(out,'result.json',receipt)
 print(json.dumps(dict(prepared=True,census=len(PINS),migrated=len(PINS)-len(LEGACY),legacy_helpers=len(LEGACY))))

if __name__=='__main__':
 p.require(sys.argv==[sys.argv[0],'prepare'],'fixed exclusive preparation');prepare()
