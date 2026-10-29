"""Private native-zbus transport candidate, before offline lock resolution."""
from pathlib import Path
import hashlib
import importlib.util
import json
import shutil
import sys

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'target/dbus-native-20261028'
PREVIOUS = ROOT / 'target/dbus-legacy-integration-20261028/tests'
NATIVE = '/home/artur/.cache/hydracache-074-dbus-native/20261028-a1'
PKG = 'tools/long-run-supervisor-074'
PINS = {
    'src/diagnostic_dispatch_negotiated_tests.rs': '05f024b711a01ce61ab0c76070ba7c18e229a570a7fe216bab7f41fa55d2c29e',
    'src/diagnostic_negotiated_credentials_adapter.rs': '27087765fa607e7da78c7b6091ee4a735308f3712ad347e01170dc4e95bf23e2',
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def need(ok, why):
    if not ok:
        raise ValueError(why)


def files(folder):
    return {p.relative_to(folder).as_posix(): sha(p.read_bytes())
            for p in sorted(folder.rglob('*')) if p.is_file()}


def write(folder, name, value):
    with (folder / name).open('x', encoding='utf8', newline='\n') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')


# Reuse the established outer source/ref guard and CI mechanism unchanged.
spec = importlib.util.spec_from_file_location('native_transport_original_parent',
    ROOT / 'tools/dbus-legacy-integration-074/prepare.py')
parent = importlib.util.module_from_spec(spec)
spec.loader.exec_module(parent)
p, prior, fixed = parent.p, parent.prior, parent.fixed


def change(text, old, new):
    need(text.count(old) == 1, 'whole unique transport projection: ' + old[:60])
    return text.replace(old, new, 1)


def transport(raw):
    need(sha(raw) == PINS['src/diagnostic_dispatch_negotiated_tests.rs'], 'whole original transport')
    text = raw.decode()
    replacements = (
        ('use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};\n', ''),
        ('struct Reader(OwnedReadHalf, Arc<Mutex<Wire>>, bool, bool);',
         'struct Reader(Box<dyn ReadHalf>, Arc<Mutex<Wire>>, bool, bool, tokio::net::UnixStream);'),
        ('            let socket = self.0.as_ref();', '            let socket = &self.4;'),
        ('::zbus::connection::socket::ReadHalf::recvmsg(&mut self.0, bytes).await?',
         'self.0.recvmsg(bytes).await?'),
        ('credential_adapter::transfer(::zbus::connection::socket::ReadHalf::peer_credentials(&mut self.0).await?)',
         'self.0.peer_credentials().await'),
        ('struct Writer(OwnedWriteHalf, Arc<Mutex<Wire>>, bool);',
         'struct Writer(Box<dyn WriteHalf>, Arc<Mutex<Wire>>, bool);'),
        ('::zbus::connection::socket::WriteHalf::sendmsg(&mut self.0, &bytes[..bytes.len().min(max)], fds).await?',
         'self.0.sendmsg(&bytes[..bytes.len().min(max)], fds).await?'),
        ('::zbus::connection::socket::WriteHalf::close(&mut self.0).await', 'self.0.close().await'),
        ('credential_adapter::transfer(::zbus::connection::socket::WriteHalf::peer_credentials(&mut self.0).await?)',
         'self.0.peer_credentials().await'),
        ('        let (read, write) = socket.into_split();',
         '        let socket = socket.into_std().unwrap();\n'
         '        let atomic_socket = tokio::net::UnixStream::from_std(socket.try_clone().unwrap()).unwrap();\n'
         '        let (read, write) = zbus::connection::socket::hc074_unix_fixture(socket).unwrap().take();'),
        ('Box::new(Reader(read, wire.clone(), capable, atomic)),',
         'Box::new(Reader(read, wire.clone(), capable, atomic, atomic_socket)),'),
    )
    for old, new in replacements:
        text = change(text, old, new)
    need('::zbus::' not in text and 'OwnedReadHalf' not in text, 'no old runtime backend')
    return text.encode()


def credentials(raw):
    need(sha(raw) == PINS['src/diagnostic_negotiated_credentials_adapter.rs'], 'whole original DTO control')
    text = raw.decode().replace('::zbus::fdo::', '::zbus6::fdo::').replace(
        '::zbus::zvariant::', '::zbus6::wire::').replace('::zbus6::wire::Value::',
        '::zbus6::wire::Variant::').replace('Context::new_dbus(', 'Context::new(')
    need('::zbus::' not in text and 'new_dbus' not in text, 'one current DTO type only')
    return text.encode()


def prepare():
    out = BASE / 'prepare'
    out.mkdir(parents=True, exist_ok=False)
    (out / 'producer.py').write_bytes(Path(__file__).read_bytes())
    review = json.loads((PREVIOUS / 'freeze/result.json').read_bytes())
    need(files(PREVIOUS / 'input') == review['files'], 'complete qualified predecessor')
    gate = json.loads((PREVIOUS / 'units/native-result.json').read_bytes())
    need(gate['passed'] and gate['passes_per_compiler'] == 64 and gate['source_unchanged'],
         'actual joined-caller paired unit gate first')
    need(all(sha((ROOT / n).read_bytes()) == v for n, v in review['root_inputs'].items()),
         'same protected product inputs')
    dest = BASE / 'input'
    shutil.copytree(PREVIOUS / 'input', dest)
    source = dest / 'fd' / PKG
    for name, project in (('src/diagnostic_dispatch_negotiated_tests.rs', transport),
                          ('src/diagnostic_negotiated_credentials_adapter.rs', credentials)):
        path = source / name
        path.write_bytes(project(path.read_bytes()))
    manifest = source / 'Cargo.toml'
    text = manifest.read_text()
    old = '[target.\'cfg(target_os = "linux")\'.dev-dependencies]\n' + \
        'zbus = { version = "5.19.0", default-features = false, features = ["blocking-api", "tokio"] }\n'
    text = change(text, old,
        '[target.\'cfg(target_os = "linux")\'.dev-dependencies.zbus6]\n'
        'package = "zbus"\npath = "../../source/zbus"\ndefault-features = false\n'
        'features = ["hc074-unix-fixture"]\n')
    manifest.write_text(text, encoding='utf8', newline='\n')
    zbus = dest / 'fd/source/zbus'
    original_manifest = (zbus / 'Cargo.toml').read_bytes()
    need(sha(original_manifest) == 'd2836489dcf6772e84e0fdbe124078ee6fd968e99a53d312f21f10c0bfe222af',
         'exact qualified canonical compiler manifest')
    for name in ('Cargo.toml', 'Cargo.toml.orig'):
        path = zbus / name
        need(path.read_bytes() == original_manifest, 'same qualified authoring manifest')
        path.write_text(change(path.read_text(), '[features]\n',
            '[features]\nhc074-unix-fixture = []\n'), encoding='utf8', newline='\n')
    path = zbus / 'src/connection/socket/mod.rs'
    need(sha(path.read_bytes()) == '72ca2ca5d08f4758c099045b7a887a4f1b0e034c28d29a3d34460caa90649794',
         'whole original socket module')
    with path.open('ab') as stream:
        stream.write(b'\n' + (Path(__file__).parent / 'unix_fixture.rs').read_bytes())
    provenance = zbus / 'BACKPORT074.json'
    body = json.loads(provenance.read_bytes())
    body.update(compiler_manifest_sha256=sha((zbus / 'Cargo.toml').read_bytes()),
                source_revision='hc074-zbus-native-fixture-source-2',
                nondefault_fixture_feature='hc074-unix-fixture',
                production_default_features_changed=False)
    body['source_changes_from_upstream'] = sorted(set(body['source_changes_from_upstream']) |
                                                {'src/connection/socket/mod.rs'})
    provenance.write_text(json.dumps(body, indent=2, sort_keys=True)+'\n', encoding='utf8', newline='\n')
    for variant in ('baseline', 'fd'):
        path = dest / variant / '.cargo/config.toml'
        path.write_bytes(change(path.read_text(), review['native_root'], NATIVE).encode())
    helpers = dict(review['helpers'])
    for name in ('prepare.py', 'resolve_native.py', 'resolve_run.py', 'lock_review.py', 'test_controls.py', 'unix_fixture.rs'):
        key = 'tools/dbus-native-transport-074/' + name
        helpers[key] = sha((ROOT / key).read_bytes())
    (BASE / 'freeze').mkdir()
    value = dict(review, files=files(dest), helpers=helpers, native_root=NATIVE,
        old_backend_removed_from_source=True, old_template_lock_requires_resolution=True,
        root_changed=False, release_admitted=False)
    write(BASE / 'freeze', 'result.json', value)
    write(out, 'result.json', value)
    print(json.dumps(dict(prepared=True, root_changed=False, lock_not_qualified=True)))


if __name__ == '__main__':
    need(sys.argv[1:] == ['prepare'], 'fixed preparation only')
    prepare()
