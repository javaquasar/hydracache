"""Freeze a separate compile candidate from the same source-audited first lock."""
from pathlib import Path
import importlib.util
import json
import shutil
import sys

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('native_transport_first_preparation', Path(__file__).with_name('prepare.py'))
parent = importlib.util.module_from_spec(spec)
spec.loader.exec_module(parent)
p, prior, fixed = parent.p, parent.prior, parent.fixed
sha, files, write = parent.sha, parent.files, parent.write
BASE = parent.BASE / 'c'
NATIVE = '/home/artur/.cache/hydracache-074-dbus-native/20261028-a2'


def prepare():
    out = BASE/'prepare'
    out.mkdir(parents=True,exist_ok=False)
    (out/'producer.py').write_bytes(Path(__file__).read_bytes())
    review = json.loads((parent.BASE/'freeze/result.json').read_bytes())
    parent.need(files(parent.BASE/'input')==review['files'],'original first projection preserved')
    audit = json.loads((parent.BASE/'feature-audit/result.json').read_bytes())
    lock = (parent.BASE/'session/Cargo.lock').read_bytes()
    parent.need(audit['passed'] and audit['old_zbus_absent'] and audit['no_new_packages']
                and audit['resolved_lock_sha256']==sha(lock) and audit['new_cargo_run'] is False,
                'source-audited exact first resolver lock, no repeat')
    parent.need(all(sha((ROOT/n).read_bytes())==v for n,v in review['root_inputs'].items()),
                'protected product inputs unchanged')
    dest=BASE/'input'
    shutil.copytree(parent.BASE/'input',dest)
    (dest/'fd/Cargo.lock').write_bytes(lock)
    for variant in ('baseline','fd'):
        path=dest/variant/'.cargo/config.toml'
        text=path.read_text()
        path.write_text(parent.change(text,review['native_root'],NATIVE),encoding='utf8',newline='\n')
    helpers=dict(review['helpers'])
    for name in ('feature_review.py','review_saved.py','test_feature_review.py','compile_prepare.py',
                 'compile_native.py','compile_run.py','unit_native.py','unit_run.py','test_compile.py'):
        key='tools/dbus-native-transport-074/'+name
        helpers[key]=sha((ROOT/key).read_bytes())
    value=dict(review,files=files(dest),helpers=helpers,native_root=NATIVE,
        old_template_lock_requires_resolution=False,old_zbus_absent_from_candidate_lock=True,
        source_audited_feature_drop=True,root_changed=False,release_admitted=False)
    (BASE/'freeze').mkdir()
    write(BASE/'freeze','result.json',value)
    write(out,'result.json',value)
    write(out,'audit-input.json',audit)
    print(json.dumps(dict(prepared=True,packages=135,root_changed=False)))


if __name__=='__main__':
    parent.need(sys.argv[1:]==['prepare'],'fixed exclusive compiler preparation')
    prepare()
