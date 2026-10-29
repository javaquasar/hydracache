"""Close all original literal module/include edges before the next unit build."""
from pathlib import Path
import importlib.util,json,shutil,sys
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('delivery_inventory_parent',ROOT/'tools/dbus-original-delivery-074/prepare.py');r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
p,prior,sha,files,write,fixed=r.p,r.prior,r.sha,r.files,r.write,r.fixed
BASE=r.BASE/'unit-inventory';NATIVE=r.NATIVE.replace('-a1','-a4')
if __name__=='__main__':
 p.require(sys.argv==[sys.argv[0],'prepare'],'fixed exclusive preparation')
 before=r.BASE/'unit-complete';gate=json.loads((before/'session/native-result.json').read_bytes())
 p.require(not gate['passed'] and gate['source_unchanged'] and gate['phases']==[dict(label='fd-194-build',exit=101)],'first concat-include refusal retained')
 messages=[json.loads(line) for line in (before/'session/fd-194-build/stdout.log').read_bytes().splitlines()]
 errors=[m['message'] for m in messages if m.get('reason')=='compiler-message' and m['message']['level']=='error']
 p.require(len(errors)==1 and 'material-lock-fixture-074/controls.rs' in errors[0]['message'] and 'No such file' in errors[0]['message'],'exact missing original concat include')
 review=json.loads((before/'freeze/result.json').read_bytes());p.require(files(before/'input')==review['files'],'exact previous source')
 module=p.load(Path(__file__).parent/'include_inventory.py','delivery_original_inventory');members,edges=module.inventory(ROOT,r.PKG,p.require)
 missing={n:raw for n,raw in members.items() if not (before/'input/fd'/n).exists()}
 p.require(set(missing)=={'tools/owned-case-074/controls.rs','tools/material-lock-fixture-074/controls.rs','docs/testing/performance/0.74/local-runs/status-field-scan-baseline-20261024/objects/fb77cd5dae690c838652f358e7520d7d454ba202b8526bfd1678ca0315d20b97'},'closed three missing original assets')
 out=BASE/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes());dest=BASE/'input';shutil.copytree(before/'input',dest)
 for variant in ('baseline','fd'):
  for name,raw in missing.items():path=dest/variant/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
  config=dest/variant/'.cargo/config.toml';old=config.read_bytes();p.require(old.count(review['native_root'].encode())==1,'only vendor relocation');config.write_bytes(old.replace(review['native_root'].encode(),NATIVE.encode(),1))
 receipt=dict(review,files=files(dest),native_root=NATIVE,original_literal_inventory={n:sha(raw) for n,raw in members.items()},original_literal_edges=edges,added_assets={n:sha(raw) for n,raw in missing.items()},helpers=dict(review['helpers'],**{n:sha(raw) for n,raw in missing.items()}))
 (BASE/'freeze').mkdir();write(BASE/'freeze','result.json',receipt);write(out,'result.json',receipt)
 print(json.dumps(dict(targets=len(members),edges=len(edges),exact_original_assets_added=len(missing))))
