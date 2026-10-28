"""Reuse the exact bounded CI library gate with only exclusive roots changed."""
from pathlib import Path
import hashlib
ROOT=Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
path=ROOT/'tools/dbus-original-owner-074/native.py'
raw=path.read_bytes()
if hashlib.sha256(raw).hexdigest()!='30aa12ddc33da8b94a0c8fd07d4161b2d4d42e27baa08c1c2d39070d5b045b9a':raise ValueError('pinned parent native owner')
text=raw.decode().replace("BASE=ROOT/'target/dbus-original-owner-20261028'","BASE=ROOT/'target/dbus-original-delivery-20261028'",1).replace('/home/artur/.cache/hydracache-074-dbus-original-owner/20261028-a1','/home/artur/.cache/hydracache-074-dbus-original-delivery/20261028-a1',1)
exec(compile(text,str(path),'exec'),globals())
