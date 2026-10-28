"""Qualified complete 64-case coordinator, only the resource-repaired roots differ."""
from pathlib import Path
import hashlib

path=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-legacy-integration-074/unit_native.py')
raw=path.read_bytes()
if hashlib.sha256(raw).hexdigest()!='0b940dcfc7e5ab71a19775e0ece51bf3e7abd990fa0200a1c5c7ecdbf8d4f2b8':
    raise ValueError('exact original image/catalog/row owner')
text=raw.decode().replace("BASE = ROOT / 'target/dbus-legacy-integration-20261028/tests'",
    "BASE = ROOT / 'target/dbus-native-20261028/r'",1).replace(
    '/home/artur/.cache/hydracache-074-dbus-legacy-integration/20261028-a7',
    '/home/artur/.cache/hydracache-074-dbus-native/20261028-a3',1)
exec(compile(text,str(path),'exec'),globals())
