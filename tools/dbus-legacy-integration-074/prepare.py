"""Join three previously separate legacy callers to the reviewed zbus 6 graph."""
from pathlib import Path
import hashlib
import importlib.util
import json
import shutil
import sys

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'target/dbus-legacy-integration-20261028/used-api-fixed'
PREVIOUS = ROOT / 'target/dbus-original-delivery-20261028/unit-api-owned'
NATIVE = '/home/artur/.cache/hydracache-074-dbus-legacy-integration/20261028-a5'
PKG = 'tools/long-run-supervisor-074'
NAMES = ('diagnostic_manager.rs', 'diagnostic_loaded.rs', 'systemd_unit.rs')
PROJECTOR_SHA = '29fe1f3ad3dcd16175c4d2c52ede4ef44e02617a3198e6deadcc2c16eb325d3b'
BRIDGE_SHA = '56650da011824f801a7d8d43c983328c8db9c46f157ddb4fe9cfd8124a305a54'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def need(value, why):
    if not value:
        raise ValueError(why)


def files(folder):
    return {p.relative_to(folder).as_posix(): sha(p.read_bytes())
            for p in sorted(folder.rglob('*')) if p.is_file()}


def put(path, value):
    with path.open('x', encoding='utf8', newline='\n') as stream:
        json.dump(value, stream, sort_keys=True, indent=2)
        stream.write('\n')


def original_projector():
    path = ROOT / 'tools/dbus-original-delivery-074/prepare.py'
    need(sha(path.read_bytes()) == PROJECTOR_SHA, 'original whole-body API projector')
    spec = importlib.util.spec_from_file_location('hc074_legacy_exact_projector', path)
    parent = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(parent)
    return parent


# Preserve the original outer owner's source/snapshot/capacity mechanisms.
_parent = original_projector()
p, prior, fixed = _parent.p, _parent.prior, _parent.fixed
write = _parent.write


def projection(name, raw):
    parent = original_projector()
    need(name in NAMES, 'closed three-caller integration')
    parent.LEGACY = set()  # Only this pure projection instance, not old files.
    text = parent.project(name, raw, v2=True).decode()
    if name == 'diagnostic_manager.rs':
        old = 'use zbus::blocking::{connection::Builder, Connection, Proxy};'
        need(text.count(old) == 1, 'original manager blocking import')
        text = text.replace(old, 'use crate::diagnostic_blocking_v6::{ConnectionBuilder as Builder, Connection, Proxy};')
        need(text.count('zbus::blocking::proxy::Builder::new(connection)') == 1,
             'original manager proxy builder')
        text = text.replace('zbus::blocking::proxy::Builder::new(connection)',
                            'crate::diagnostic_blocking_v6::ProxyBuilder::new(connection)')
    if name == 'systemd_unit.rs':
        old = 'use zbus::blocking::{Connection, Proxy};'
        need(text.count(old) == 1, 'original unit blocking import')
        text = text.replace(old, 'use crate::diagnostic_blocking_v6::{Connection, Proxy};')
        old = 'use zbus::wire::{OwnedObjectPath, Value};'
        need(text.count(old) == 1, 'compiler-identified unit wire alias')
        text = text.replace(old, 'use zbus::wire::{OwnedObjectPath, Variant as Value};')
    if name == 'diagnostic_loaded.rs':
        old = 'use zbus::wire::OwnedValue;'
        need(text.count(old) == 1, 'compiler-identified loaded wire alias')
        text = text.replace(old, 'use zbus::wire::OwnedVariant as OwnedValue;')
    need('zbus::blocking' not in text and 'zbus::zvariant' not in text,
         'no removed blocking/wire module in migrated caller')
    return text.encode()


def blocking_bridge(raw):
    need(sha(raw) == BRIDGE_SHA, 'qualified v2 blocking source before narrowing')
    text = raw.decode()
    # Only three unused, private wrapper entry points; no dummy call or lint waiver.
    for old in (
        '    pub fn from_async(connection: zbus::Connection) -> Self {\n        Self(connection)\n    }\n\n',
        '    pub fn close(self) -> Result<()> {\n        zbus::block_on(self.0.close())\n    }\n',
        '    pub fn connection(&self) -> &Connection {\n        &self.connection\n    }\n\n',
    ):
        need(text.count(old) == 1, 'one compiler-identified unused wrapper')
        text = text.replace(old, '', 1)
    for old, new in (
        ('    connection: Connection,\n    inner: Option<zbus::Proxy',
         '    _connection: Connection,\n    inner: Option<zbus::Proxy'),
        ('        Ok(Proxy {\n            connection: self.connection,',
         '        Ok(Proxy {\n            _connection: self.connection,'),
        ('        Ok(Self {\n            connection: connection.clone(),\n            inner: Some(proxy),',
         '        Ok(Self {\n            _connection: connection.clone(),\n            inner: Some(proxy),'),
    ):
        need(text.count(old) == 1, 'retained original connection lifetime slot')
        text = text.replace(old, new, 1)
    lines = text.splitlines(True)
    lines.insert(1, 'use zbus6 as zbus;\n')
    return ''.join(lines).encode()


def prepare():
    out = BASE / 'prepare'
    out.mkdir(parents=True, exist_ok=False)
    (out / 'producer.py').write_bytes(Path(__file__).read_bytes())
    review = json.loads((PREVIOUS / 'freeze/result.json').read_bytes())
    need(files(PREVIOUS / 'input') == review['files'], 'immutable full original graph')
    need(all(sha((ROOT / n).read_bytes()) == v for n, v in review['root_inputs'].items()),
         'unchanged protected root inputs')
    dest = BASE / 'input'
    shutil.copytree(PREVIOUS / 'input', dest)
    source = dest / 'fd' / PKG / 'src'
    changes = {}
    for name in NAMES:
        path = source / name
        before = path.read_bytes()
        after = projection(name, before)
        need(before == (ROOT / PKG / 'src' / name).read_bytes(), 'whole original legacy caller')
        path.write_bytes(after)
        changes[name] = dict(before=sha(before), after=sha(after))
    bridge = (ROOT / 'tools/dbus-runtime-bridge-074/bridge_v2.rs').read_bytes()
    need(sha(bridge) == BRIDGE_SHA,
         'previously qualified blocking bridge, not first prototype')
    (source / 'diagnostic_blocking_v6.rs').write_bytes(blocking_bridge(bridge))
    lib = source / 'lib.rs'
    need('diagnostic_blocking_v6' not in lib.read_text(), 'one new private bridge module')
    with lib.open('ab') as stream:
        stream.write(b'\n#[cfg(target_os = "linux")]\nmod diagnostic_blocking_v6;\n')
    manifest = dest / 'fd' / PKG / 'Cargo.toml'
    raw = manifest.read_text()
    old = 'zbus = { version = "5.19.0", default-features = false, features = ["blocking-api", "tokio"] }\n'
    need(raw.count(old) == 1 and "[target.'cfg(target_os = \"linux\")'.dev-dependencies]" not in raw,
         'one old production dependency')
    raw = raw.replace(old, '')
    raw += '\n[target.\'cfg(target_os = "linux")\'.dev-dependencies]\n' + old
    manifest.write_text(raw, encoding='utf8', newline='\n')
    for variant in ('baseline', 'fd'):
        path = dest / variant / '.cargo/config.toml'
        raw = path.read_bytes()
        need(raw.count(review['native_root'].encode()) == 1, 'one exact vendor relocation')
        path.write_bytes(raw.replace(review['native_root'].encode(), NATIVE.encode(), 1))
        need((dest / variant / 'Cargo.lock').read_bytes() ==
             (PREVIOUS / 'input' / variant / 'Cargo.lock').read_bytes(), 'unchanged complete lock')
    helpers = dict(review['helpers'])
    for name in ('prepare.py', 'run.py', 'native.py', 'test_controls.py'):
        key = 'tools/dbus-legacy-integration-074/' + name
        helpers[key] = sha((ROOT / key).read_bytes())
    put(out / 'projection.json', changes)
    (BASE / 'freeze').mkdir()
    put(BASE / 'freeze/result.json', dict(review, files=files(dest), helpers=helpers,
        native_root=NATIVE, three_legacy_callers_joined=True, production_old_zbus_removed=True,
        old_zbus_test_backend_retained=True, root_changed=False, release_admitted=False))
    print(json.dumps(dict(prepared=True, callers=3, old_zbus_test_only=True, root_changed=False)))


if __name__ == '__main__':
    need(sys.argv[1:] == ['prepare'], 'fixed exclusive preparation')
    prepare()
