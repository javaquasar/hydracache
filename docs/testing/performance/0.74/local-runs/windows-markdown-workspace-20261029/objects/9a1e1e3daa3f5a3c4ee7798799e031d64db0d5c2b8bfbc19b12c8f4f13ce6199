"""Separate resource-repaired context; old floor refusal is never overwritten."""
from pathlib import Path
import importlib.util
import json
import shutil
import sys

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('native_transport_prior_compile',Path(__file__).with_name('compile_prepare.py'))
parent=importlib.util.module_from_spec(spec)
spec.loader.exec_module(parent)
p,prior,fixed=parent.p,parent.prior,parent.fixed
sha,files,write=parent.sha,parent.files,parent.write
BASE=ROOT/'target/dbus-native-20261028/r'
NATIVE='/home/artur/.cache/hydracache-074-dbus-native/20261028-a3'


def prepare():
    out=BASE/'prepare'
    out.mkdir(parents=True,exist_ok=False)
    (out/'producer.py').write_bytes(Path(__file__).read_bytes())
    review=json.loads((parent.BASE/'freeze/result.json').read_bytes())
    parent.parent.need(files(parent.BASE/'input')==review['files'],'first compile source remains frozen')
    failure=json.loads((parent.BASE/'session/execution.json').read_bytes())
    parent.parent.need(failure['classification']=='resource-floor' and failure['exit']==125
        and failure['retired'] and failure['errors']==['CapacityError: original 4 GiB floor'],
        'exact first capacity refusal/retirement retained')
    parent.parent.need(not (parent.BASE/'session/native-result.json').exists(),
                       'do not promote or join an absent native result')
    parent.parent.need(shutil.disk_usage(ROOT).free>=16*1024**3,'repaired physical headroom, original floor retained')
    dest=BASE/'input'
    shutil.copytree(parent.BASE/'input',dest)
    for variant in ('baseline','fd'):
        path=dest/variant/'.cargo/config.toml'
        raw=path.read_bytes()
        parent.parent.need(raw.count(review['native_root'].encode())==1,'one exact vendor path relocation')
        path.write_bytes(raw.replace(review['native_root'].encode(),NATIVE.encode(),1))
    helpers=dict(review['helpers'])
    for name in ('capacity_prepare.py','capacity_native.py','capacity_run.py',
                 'capacity_unit_native.py','capacity_unit_run.py','test_capacity.py'):
        key='tools/dbus-native-transport-074/'+name
        helpers[key]=sha((ROOT/key).read_bytes())
    value=dict(review,files=files(dest),helpers=helpers,native_root=NATIVE,
               first_capacity_refusal_preserved=True,root_changed=False,release_admitted=False)
    (BASE/'freeze').mkdir()
    write(BASE/'freeze','result.json',value)
    write(out,'result.json',value)
    write(out,'capacity.json',dict(observed_windows_free=shutil.disk_usage(ROOT).free,
        initial_repaired_prerequisite=16*1024**3,running_floor=4*1024**3,
        poll=5,retirement=5,bound=1200,session_bound=1100,thresholds_changed=False,
        old_native_verdict_absent=True))
    print(json.dumps(dict(prepared=True,resource_repaired=True,source_code_changed=False)))


if __name__=='__main__':
    parent.parent.need(sys.argv[1:]==['prepare'],'fixed exclusive resource context')
    prepare()
