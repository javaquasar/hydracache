"""Keep the original Windows Job outer owner and all frozen input checks."""
from pathlib import Path
import hashlib

path = Path(__file__).resolve().parents[2] / 'tools/dbus-original-owner-074/run.py'
raw = path.read_bytes()
if hashlib.sha256(raw).hexdigest() != 'f6fc9829998d3f2e1a38f35c277fa51d790c8f6e5eafdaeaf981a4c6add7fce3':
    raise ValueError('unchanged original outer library compile owner')
text = raw.decode().replace('tools/dbus-original-owner-074/prepare.py',
    'tools/dbus-legacy-integration-074/prepare.py', 1).replace(
    '/tools/dbus-original-owner-074/native.py', '/tools/dbus-legacy-integration-074/native.py', 1)
exec(compile(text, str(path), 'exec'), globals())
