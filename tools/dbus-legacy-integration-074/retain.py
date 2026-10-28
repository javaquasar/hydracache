"""Retain exact original legacy integration contexts, including every first refusal."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(r'C:\Workspace\prj\jq\cashe\hydracache-074')
BASE = ROOT / 'target/dbus-legacy-integration-20261028'
OUT = ROOT / 'docs/testing/performance/0.74/local-runs/dbus-legacy-integration-20261028'
CONTEXTS = ('initial', 'owner-bound', 'api-bound', 'used-api', 'used-api-fixed', 'fixed', 'tests')
PKG = 'tools/long-run-supervisor-074'
SOURCE = ('src/diagnostic_manager.rs', 'src/diagnostic_loaded.rs', 'src/systemd_unit.rs',
          'src/diagnostic_blocking_v6.rs', 'src/lib.rs', 'Cargo.toml')


def need(ok, why):
    if not ok:
        raise ValueError(why)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    checked = json.loads((BASE / 'tests/session/native-result.json').read_bytes())
    units = json.loads((BASE / 'tests/units/native-result.json').read_bytes())
    need(checked['passed'] and checked['source_unchanged'] and checked['boot_unchanged'],
         'actual paired final library gate')
    need(units['passed'] and units['source_unchanged'] and units['boot_unchanged'] and
         units['passes_per_compiler'] == 64 and units['catalog_count'] == 1593,
         'actual paired final unit gate')
    raw = {}
    for label in CONTEXTS:
        folder = BASE if label == 'initial' else BASE / label
        need(folder.is_dir(), 'retained context: ' + label)
        for name in ('prepare', 'freeze', 'session', 'units'):
            scope = folder / name
            if scope.is_dir():
                for path in scope.rglob('*'):
                    if path.is_file():
                        raw[label + '/' + name + '/' + path.relative_to(scope).as_posix()] = path.read_bytes()
                if name in ('session', 'units') and (scope / 'intent.json').is_file():
                    intent = json.loads((scope / 'intent.json').read_bytes())
                    head = intent['head']
                    need(re.fullmatch('[0-9a-f]{40}', head), 'actual dispatch head')
                    for path in ('prepare.py', 'native.py', 'run.py', 'unit_native.py', 'unit_run.py'):
                        member = 'tools/dbus-legacy-integration-074/' + path
                        result = subprocess.run(['git', 'show', head + ':' + member], cwd=ROOT,
                                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
                        if result.returncode == 0:
                            raw[label + '/' + name + '/committed-producer/' + path] = result.stdout
            if label == 'initial' and name in ('session', 'units'):
                need(not scope.exists(), 'initial preparation was never dispatched')
        for name in SOURCE:
            path = folder / 'input/fd' / PKG / name
            if path.is_file():
                raw[label + '/input/fd/' + PKG + '/' + name] = path.read_bytes()
        for name in ('Cargo.toml', 'Cargo.lock', '.cargo/config.toml'):
            path = folder / 'input/fd' / name
            if path.is_file():
                raw[label + '/input/fd/' + name] = path.read_bytes()
        path = folder / 'units-inputs.json'
        if path.is_file():
            raw[label + '/units-inputs.json'] = path.read_bytes()
    for path in (ROOT / 'tools/dbus-legacy-integration-074').iterdir():
        if path.is_file():
            raw['producer/' + path.name] = path.read_bytes()
    raw['producer/retain.py'] = Path(__file__).read_bytes()
    parent = ROOT / 'docs/testing/performance/0.74/local-runs/dbus-original-delivery-20261028/index.json'
    raw['parent/original-delivery-index.json'] = parent.read_bytes()
    raw['source/plan.md'] = (ROOT / 'docs/testing/performance/0.74/dbus-legacy-integration-plan-20261028.md').read_bytes()
    for phase in ('session', 'units'):
        for compiler in ('194', '188'):
            for action in (('check', 'clippy') if phase == 'session' else ('build', 'list', 'test', 'clippy')):
                path = BASE / 'tests' / phase / f'fd-{compiler}-{action}/execution.json'
                execution = json.loads(path.read_bytes())
                need(execution['retired'] and not execution['errors'], 'each final phase retired')
    OUT.mkdir(exist_ok=False)
    objects = OUT / 'objects'
    objects.mkdir()
    members = {}
    for name, value in sorted(raw.items()):
        digest = sha(value)
        path = objects / digest
        if not path.exists():
            path.write_bytes(value)
        members[name] = dict(sha256=digest, bytes=len(value))
    packet = dict(schema_version=1, files=members, server=False, root_changed=False,
        admission=False, performance_improvement_proven=False,
        compilers=['GNU Rust 1.94', 'GNU Rust 1.88'], passes_per_compiler=64,
        catalog_count=1593, filtered_per_compiler=1529,
        original_windows_seconds=[300, 60, 10], first_refusals_retained=True,
        binaries_excluded=True, old_zbus_test_backend_retained=True)
    (OUT / 'index.json').write_text(json.dumps(packet, indent=2, sort_keys=True)+'\n',
                                   encoding='utf8', newline='\n')
    (OUT / '.gitattributes').write_text('objects/* -text -diff\nindex.json -text\n',
                                       encoding='ascii', newline='\n')
    print(json.dumps(dict(members=len(members), objects=len(list(objects.iterdir())),
                         index_sha256=sha((OUT / 'index.json').read_bytes()))))


if __name__ == '__main__':
    main()
