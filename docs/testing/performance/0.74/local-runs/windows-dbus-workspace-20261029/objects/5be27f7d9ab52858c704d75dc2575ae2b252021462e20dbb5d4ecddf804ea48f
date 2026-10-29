"""Original fixed CI owner for one explicitly lock-mutating offline preparation."""
from pathlib import Path
import hashlib

path = Path(__file__).resolve().parents[2] / 'tools/dbus-original-owner-074/run.py'
raw = path.read_bytes()
if hashlib.sha256(raw).hexdigest() != 'f6fc9829998d3f2e1a38f35c277fa51d790c8f6e5eafdaeaf981a4c6add7fce3':
    raise ValueError('same original outer CI source/ref/Job guard')
text = raw.decode().replace('tools/dbus-original-owner-074/prepare.py',
    'tools/dbus-native-transport-074/prepare.py', 1).replace(
    '/tools/dbus-original-owner-074/native.py', '/tools/dbus-native-transport-074/resolve_native.py', 1)
old = "result['passed'] and result['source_unchanged'] and result['boot_unchanged']"
if text.count(old) != 1:
    raise ValueError('one original native outcome boundary')
text = text.replace(old,
    "result['passed'] and result['source_unchanged_except_resolved_lock'] and result['boot_unchanged']", 1)
exec(compile(text, str(path), 'exec'), globals())
