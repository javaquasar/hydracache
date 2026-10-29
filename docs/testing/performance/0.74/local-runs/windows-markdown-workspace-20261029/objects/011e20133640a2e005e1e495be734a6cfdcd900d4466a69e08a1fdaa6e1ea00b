"""Separate test-only API adaptation after a passing full original graph check."""
from pathlib import Path
import importlib.util,json,shutil,sys
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('delivery_test_parent',ROOT/'tools/dbus-original-delivery-074/prepare.py');r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
p,prior,sha,files,write,fixed=r.p,r.prior,r.sha,r.files,r.write,r.fixed
BASE=r.BASE/'unit-build';NATIVE=r.NATIVE.replace('-a1','-a2')
if __name__=='__main__':
 p.require(sys.argv==[sys.argv[0],'prepare'],'fixed exclusive preparation')
 out=BASE/'prepare';out.mkdir(parents=True,exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 review=json.loads((r.BASE/'freeze/result.json').read_bytes());gate=json.loads((r.BASE/'session/native-result.json').read_bytes())
 p.require(gate['passed'] and len(gate['phases'])==4 and all(v['exit']==0 for v in gate['phases']),'first complete original graph library gate')
 p.require(files(r.BASE/'input')==review['files'],'immutable prior input');dest=BASE/'input';shutil.copytree(r.BASE/'input',dest)
 pins=json.loads((Path(__file__).parent/'test-source-pins.json').read_bytes());r.PINS=dict(r.PINS,**pins)
 for name,digest in pins.items():
  raw=(fixed.SOURCE/name).read_bytes();p.require(sha(raw)==digest,'whole original inherited test source');body=r.project(name,raw)
  if b'zbus::' not in raw:body=body.replace(b'use zbus6 as zbus;\n',b'',1)
  (dest/'fd'/r.PKG/'src'/name).write_bytes(body)
 for variant in ('baseline','fd'):
  path=dest/variant/'.cargo/config.toml';raw=path.read_bytes();p.require(raw.count(review['native_root'].encode())==1,'only vendor relocation');path.write_bytes(raw.replace(review['native_root'].encode(),NATIVE.encode(),1))
 receipt=dict(review,files=files(dest),native_root=NATIVE,unit_build_only=True,unit_tests_executed=False,test_only_extra_sources=pins)
 (BASE/'freeze').mkdir();write(BASE/'freeze','result.json',receipt);write(out,'result.json',receipt)
 print('prepared exact inherited held-wire unit test adapter; no route exclusions')
