"""The original fixed Job envelope, with additional exact unit-driver custody."""
from pathlib import Path
import hashlib
import json

root = Path(__file__).resolve().parents[2]
base = root / 'target/dbus-legacy-integration-20261028/tests'
pins = json.loads((base / 'units-inputs.json').read_bytes())
for name, digest in pins['files'].items():
    if hashlib.sha256((root / name).read_bytes()).hexdigest() != digest:
        raise ValueError('fixed reviewed unit driver')
path = root / 'tools/dbus-original-owner-074/run.py'
raw = path.read_bytes()
if hashlib.sha256(raw).hexdigest() != 'f6fc9829998d3f2e1a38f35c277fa51d790c8f6e5eafdaeaf981a4c6add7fce3':
    raise ValueError('same original outer CI owner')
text = raw.decode().replace('tools/dbus-original-owner-074/prepare.py',
    'tools/dbus-legacy-integration-074/prepare.py', 1).replace(
    '/tools/dbus-original-owner-074/native.py', '/tools/dbus-legacy-integration-074/unit_native.py', 1).replace(
    "out=base/'session'", "out=base/'units'", 1)
exec(compile(text, str(path), 'exec'), globals())
