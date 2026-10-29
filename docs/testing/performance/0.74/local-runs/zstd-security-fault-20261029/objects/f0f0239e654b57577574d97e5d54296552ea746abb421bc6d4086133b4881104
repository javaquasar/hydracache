"""Archive-only source consumer with separate, explicit cfg-test formatting."""
from pathlib import Path
import importlib.util
import json
import shutil
import sys

ROOT = Path(__file__).resolve().parents[2]


def load(name, label):
    spec = importlib.util.spec_from_file_location(label, Path(__file__).with_name(name))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


parent = load('fault_prepare.py', 'native_consumer_prior')
package = load('source_package.py', 'native_consumer_package')
p, prior, fixed = parent.p, parent.prior, parent.fixed
sha, files, write = parent.sha, parent.files, parent.write
BASE = ROOT / 'target/dbus-native-source-2-20261028/consumer2'
NATIVE = '/home/artur/.cache/hydracache-074-dbus-native/20261028-a7'
PKG = parent.PKG
TESTS = ('src/diagnostic_dispatch_negotiated_tests.rs', 'src/diagnostic_negotiated_credentials_adapter.rs')
need = parent.need


def formatting(name, raw, expected):
    need(sha(raw) == expected, 'whole prior cfg-test formatting preimage')
    text = raw.decode()
    change = parent.replace
    if name == TESTS[0]:
        text = change(text, '\nuse zbus6 as zbus;\n', '\n')
        text = change(text, 'use zbus::{Connection, MatchRule, MessageStream};\n',
                      'use zbus::{Connection, MatchRule, MessageStream};\nuse zbus6 as zbus;\n')
        text = change(text, 'struct Reader(Box<dyn ReadHalf>, Arc<Mutex<Wire>>, bool, bool, tokio::net::UnixStream);',
            'struct Reader(\n    Box<dyn ReadHalf>,\n    Arc<Mutex<Wire>>,\n    bool,\n    bool,\n    tokio::net::UnixStream,\n);')
        text = change(text,
            '        let (read, write) = zbus::connection::socket::hc074_unix_fixture(socket).unwrap().take();',
            '        let (read, write) = zbus::connection::socket::hc074_unix_fixture(socket)\n            .unwrap()\n            .take();')
        need(text.count('\n        \n') == 3, 'three prior empty builder lines')
        text = text.replace('\n        \n', '\n')
        text = change(text,
            '    let (client, wire) = bus.connect(case != "no-capability", case == "atomic-adapter").await;',
            '    let (client, wire) = bus\n        .connect(case != "no-capability", case == "atomic-adapter")\n        .await;')
        text = change(text,
            '            assert_eq!(unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) }, original_flags);',
            '            assert_eq!(\n                unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) },\n                original_flags\n            );')
    elif name == TESTS[1]:
        text = change(text, 'wire::{Variant, serialized::Context}', 'wire::{serialized::Context, Variant}')
    else:
        raise ValueError('only two named cfg-test formatting files')
    return text.encode()


def prepare():
    out = BASE / 'prepare'
    out.mkdir(parents=True, exist_ok=False)
    (out / 'producer.py').write_bytes(Path(__file__).read_bytes())
    held = package.member('dbus-native-transport-20261028', package.NATIVE, 'f2/freeze/result.json')
    review = json.loads(held)
    need((parent.BASE / 'freeze/result.json').read_bytes() == held
         and files(parent.BASE / 'input') == review['files'], 'actual qualified predecessor')
    result = json.loads((parent.BASE / 'units/native-result.json').read_bytes())
    need(result['passed'] and result['source_unchanged'] and result['boot_unchanged']
         and result['passes_per_compiler'] == 65, 'actual first complete native fault cohort')
    receipt = package.export(BASE / 'export')
    recipe, packages, _ = package.reviewed()
    oldfd = parent.BASE / 'input/fd'

    def ignore(folder, names):
        return ['source'] if Path(folder) == oldfd else []

    dest = BASE / 'input'
    shutil.copytree(parent.BASE / 'input', dest, ignore=ignore)
    for crate, raw in packages.items():
        filename = crate + '-' + recipe['bases'][crate]['version'] + '-hc074-source-2.tar'
        exported = (BASE / 'export' / filename).read_bytes()
        need(exported == raw and sha(exported) == receipt['source_archive_sha256'][crate],
             'actual exported archive, not loose predecessor members')
        members = package.codec.unpack(exported, crate + '-' + recipe['bases'][crate]['version'], True)
        for name, body in members.items():
            path = dest / 'fd/source' / crate / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)
    for name in TESTS:
        path = dest / 'fd' / PKG / name
        path.write_bytes(formatting(name, path.read_bytes(), review['files']['fd/' + PKG + '/' + name]))
    for variant in ('baseline', 'fd'):
        path = dest / variant / '.cargo/config.toml'
        path.write_bytes(path.read_bytes().replace(review['native_root'].encode(), NATIVE.encode(), 1))
    changed = {n for n, value in files(dest).items() if review['files'].get(n) != value}
    need(changed == {'fd/' + PKG + '/' + n for n in TESTS} |
         {'baseline/.cargo/config.toml', 'fd/.cargo/config.toml'},
         'archive source exact; only explicit cfg-test formatting and vendor paths change')
    helpers = dict(review['helpers'])
    for name in ('source_package.py', 'consumer_prepare.py', 'consumer_build_native.py',
                 'consumer_build_run.py', 'consumer_unit_native.py', 'consumer_unit_run.py',
                 'test_consumer.py', 'test_source_package.py'):
        member = 'tools/dbus-native-transport-074/' + name
        helpers[member] = sha((ROOT / member).read_bytes())
    value = dict(review, files=files(dest), helpers=helpers, native_root=NATIVE,
        archive_source_exact=True, only_cfg_test_formatting_changed=True,
        source_package_recipe_sha256=package.RECIPE_SHA,
        source_archive_sha256=receipt['source_archive_sha256'], production_source_equal=True,
        root_changed=False, release_admitted=False)
    (BASE / 'freeze').mkdir()
    write(BASE / 'freeze', 'result.json', value)
    write(out, 'result.json', value)
    print(json.dumps(dict(prepared=True, archive_source_exact=True, production_source_changed=False)))


if __name__ == '__main__':
    need(sys.argv[1:] == ['prepare'], 'exclusive archive consumer preparation')
    prepare()
