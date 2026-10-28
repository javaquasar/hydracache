"""Qualified image/accounting owner, exact 64 original cases plus one private matrix."""
from pathlib import Path
import hashlib

path=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-legacy-integration-074/unit_native.py')
raw=path.read_bytes()
if hashlib.sha256(raw).hexdigest()!='0b940dcfc7e5ab71a19775e0ece51bf3e7abd990fa0200a1c5c7ecdbf8d4f2b8':
    raise ValueError('same complete original cohort/retirement owner')
text=raw.decode().replace("BASE = ROOT / 'target/dbus-legacy-integration-20261028/tests'",
    "BASE = ROOT / 'target/dbus-native-20261028/f2'",1).replace(
    '/home/artur/.cache/hydracache-074-dbus-legacy-integration/20261028-a7',
    '/home/artur/.cache/hydracache-074-dbus-native/20261028-a5',1)
old="    need(len(selected) == len(set(selected)) == 64, 'exact 23 caller plus 41 wire/DTO set')"
new='''    need(len(selected)==len(set(selected))==64, 'all original selected cases remain')
    matrix=[name for name in catalog if name.endswith('::dispatch_negotiated_private_bus_fault_matrix')]
    need(len(matrix)==1,'one exact original nonroot private matrix')
    selected.extend(matrix)
    need(len(selected)==len(set(selected))==65,'all original 64 plus one private matrix')'''
if text.count(old)!=1:raise ValueError('exact original source selection boundary')
text=text.replace(old,new,1)
for old,new in (
    ("len(selected) == len(set(selected)) == 64,", "len(selected) == len(set(selected)) == 65,"),
    ("len(rows) == 64 and", "len(rows) == 65 and"),
    ("[('ok', '64', '0', '0', '0', '1529')]", "[('ok', '65', '0', '0', '0', '1528')]"),
    ("len(listing['selected']) == 64", "len(listing['selected']) == 65"),
    ("argv += ['--exact', *listing['selected']]", "argv += ['--include-ignored', '--exact', *listing['selected']]"),
    ("for action in ('build', 'list', 'test', 'clippy'):", "for action in ('list', 'test', 'clippy'):"),
    ("passes_per_compiler=64", "passes_per_compiler=65"),
):
    if text.count(old)!=1:raise ValueError('one exact expanded-cohort boundary: '+old)
    text=text.replace(old,new,1)
old="        for compiler in ('194', '188'):\n            for action"
new="""        for compiler in ('194', '188'):
            receipt=json.loads((BASE/f'session/fd-{compiler}-build/result.json').read_bytes())
            need(receipt['exit']==0 and receipt['artifact'],'actual already built image')
            held=OUT/f'fd-{compiler}-build'
            held.mkdir()
            owner.write(held,'result.json',receipt)
            for action"""
if text.count(old)!=1:raise ValueError('exact verified image-receipt reuse boundary')
text=text.replace(old,new,1)
exec(compile(text,str(path),'exec'),globals())
