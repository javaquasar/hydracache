"""Identical library unit-build coordinator in a separate complete-source root."""
from pathlib import Path
import hashlib
path=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-original-delivery-074/test_native.py')
raw=path.read_bytes()
if hashlib.sha256(raw).hexdigest()!='dfa4e230900dc47c0a773b4a5288007348aa55c3e4bf64c37806d9c9354eb8dc':raise ValueError('same pinned unit-build coordinator')
text=raw.decode().replace('dbus-original-delivery-20261028/unit-build','dbus-original-delivery-20261028/unit-complete',1).replace('hydracache-074-dbus-original-delivery/20261028-a2','hydracache-074-dbus-original-delivery/20261028-a3',1)
exec(compile(text,str(path),'exec'),globals())
