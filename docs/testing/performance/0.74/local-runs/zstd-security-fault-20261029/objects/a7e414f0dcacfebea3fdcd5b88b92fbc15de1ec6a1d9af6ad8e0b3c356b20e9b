"""Preserved outer CI guard, no unchanged replay of the consumed floor attempt."""
from pathlib import Path
import hashlib

path=Path(__file__).resolve().parents[2]/'tools/dbus-original-owner-074/run.py'
raw=path.read_bytes()
if hashlib.sha256(raw).hexdigest()!='f6fc9829998d3f2e1a38f35c277fa51d790c8f6e5eafdaeaf981a4c6add7fce3':
    raise ValueError('same original source/ref/Job owner')
text=raw.decode().replace('tools/dbus-original-owner-074/prepare.py',
    'tools/dbus-native-transport-074/capacity_prepare.py',1).replace(
    '/tools/dbus-original-owner-074/native.py','/tools/dbus-native-transport-074/capacity_native.py',1)
exec(compile(text,str(path),'exec'),globals())
