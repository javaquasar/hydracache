"""Retain missing module refusal; supply the unchanged external test harness source."""
from pathlib import Path
import importlib.util,json,shutil,sys
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('delivery_complete_parent',ROOT/'tools/dbus-original-delivery-074/prepare.py');r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
p,prior,sha,files,write,fixed=r.p,r.prior,r.sha,r.files,r.write,r.fixed
BASE=r.BASE/'unit-complete';NATIVE=r.NATIVE.replace('-a1','-a3')
if __name__=='__main__':
 p.require(sys.argv==[sys.argv[0],'prepare'],'fixed exclusive preparation')
 before=r.BASE/'unit-build';gate=json.loads((before/'session/native-result.json').read_bytes())
 p.require(not gate['passed'] and gate['source_unchanged'] and gate['phases']==[dict(label='fd-194-build',exit=101)],'first full-unit build refusal retained')
 messages=[json.loads(line) for line in (before/'session/fd-194-build/stdout.log').read_bytes().splitlines()]
 errors=[m['message'] for m in messages if m.get('reason')=='compiler-message' and m['message']['level']=='error']
 p.require(len(errors)==1 and "../../owned-case-074/library.rs" in errors[0]['message'] and 'No such file' in errors[0]['message'],'exact missing original module refusal')
 review=json.loads((before/'freeze/result.json').read_bytes());p.require(files(before/'input')==review['files'],'unchanged first unit fixture')
 out=BASE/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes());dest=BASE/'input';shutil.copytree(before/'input',dest)
 name='tools/owned-case-074/library.rs';raw=(ROOT/name).read_bytes()
 # This is source already present in the repository and pinned by the first
 # source snapshot; do not synthesize or simplify the owned case implementation.
 intent=json.loads((before/'session/intent.json').read_bytes());p.require(intent['source'][name]==sha(raw),'same original external test harness identity')
 for variant in ('baseline','fd'):
  path=dest/variant/name;p.require(not path.exists(),'missing module only');path.parent.mkdir(parents=True);path.write_bytes(raw)
  config=dest/variant/'.cargo/config.toml';old=config.read_bytes();p.require(old.count(review['native_root'].encode())==1,'one vendor relocation');config.write_bytes(old.replace(review['native_root'].encode(),NATIVE.encode(),1))
 receipt=dict(review,files=files(dest),native_root=NATIVE,external_test_module={name:sha(raw)},helpers=dict(review['helpers'],**{name:sha(raw)}))
 (BASE/'freeze').mkdir();write(BASE/'freeze','result.json',receipt);write(out,'result.json',receipt)
 print('copied exact missing test harness source, first missing-module refusal retained')
