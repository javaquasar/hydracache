"""Original exact Job guard with additional resource-context unit-driver pins."""
from pathlib import Path
import hashlib
import json

root=Path(__file__).resolve().parents[2]
base=root/'target/dbus-native-20261028/r'
pins=json.loads((base/'units-inputs.json').read_bytes())
for name,digest in pins['files'].items():
    if hashlib.sha256((root/name).read_bytes()).hexdigest()!=digest:
        raise ValueError('exact resource-context unit-driver custody')
path=root/'tools/dbus-original-owner-074/run.py'
raw=path.read_bytes()
if hashlib.sha256(raw).hexdigest()!='f6fc9829998d3f2e1a38f35c277fa51d790c8f6e5eafdaeaf981a4c6add7fce3':
    raise ValueError('same original outer owner')
text=raw.decode().replace('tools/dbus-original-owner-074/prepare.py',
    'tools/dbus-native-transport-074/capacity_prepare.py',1).replace(
    '/tools/dbus-original-owner-074/native.py','/tools/dbus-native-transport-074/capacity_unit_native.py',1).replace(
    "out=base/'session'","out=base/'units'",1)
exec(compile(text,str(path),'exec'),globals())
