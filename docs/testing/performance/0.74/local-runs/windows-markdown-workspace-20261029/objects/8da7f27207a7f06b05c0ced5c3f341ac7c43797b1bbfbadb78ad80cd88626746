"""Exact full default-image producer; only archive consumer paths relocate."""
from pathlib import Path
import hashlib

path = Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-original-delivery-074/test_native.py')
raw = path.read_bytes()
if hashlib.sha256(raw).hexdigest() != 'dfa4e230900dc47c0a773b4a5288007348aa55c3e4bf64c37806d9c9354eb8dc':
    raise ValueError('same exact image owner')
text = raw.decode().replace('dbus-original-delivery-20261028/unit-build',
    'dbus-native-source-2-20261028/consumer2', 1).replace(
    '/home/artur/.cache/hydracache-074-dbus-original-delivery/20261028-a2',
    '/home/artur/.cache/hydracache-074-dbus-native/20261028-a7', 1)
exec(compile(text, str(path), 'exec'), globals())
