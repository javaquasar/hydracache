"""Retain archive-only consumer, first inert failure and exact source recipe."""
from pathlib import Path
import importlib.util
import json
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def load(name, label):
    spec = importlib.util.spec_from_file_location(label, Path(__file__).with_name(name))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


r = load('retain.py', 'archive_consumer_actual_accounting')
p = load('source_package.py', 'archive_consumer_reconstruction')
BASE = ROOT / 'target/dbus-native-source-2-20261028'
OUT = ROOT / 'docs/testing/performance/0.74/local-runs/dbus-native-source-2-20261028'


def validate():
    folder = BASE / 'consumer2'
    result = r.load(folder / 'session/native-result.json')
    r.need(result['passed'] and result['source_unchanged'] and result['boot_unchanged'],
           'actual paired archive image builds')
    r.verify_cohort(folder / 'units', 65)
    for phase in ('session', 'units'):
        execution = r.load(folder / phase / 'execution.json')
        r.need(execution['retired'] and not execution['errors'], 'owned outer postflight')
        r.need(r.load(folder / phase / 'result.json')['exit'] == 0, 'complete outer result')
    freeze = r.load(folder / 'freeze/result.json')
    recipe, packages, final = p.reviewed()
    r.need(freeze['archive_source_exact'] and freeze['production_source_equal']
           and freeze['source_package_recipe_sha256'] == p.RECIPE_SHA, 'actual archive consumer')
    for crate, members in final.items():
        for name, raw in members.items():
            r.need(r.sha(raw) == freeze['files']['fd/source/' + crate + '/' + name],
                   'consumer uses reconstructed source members')
        path = folder / 'export' / (crate + '-' + recipe['bases'][crate]['version'] + '-hc074-source-2.tar')
        r.need(path.read_bytes() == packages[crate], 'actual exported canonical archive')
    r.need((folder / 'format/exit.txt').read_text() == '0'
           and not (folder / 'format/stdout.log').read_bytes()
           and not (folder / 'format/stderr.log').read_bytes(), 'clean actual two-file rustfmt check')
    r.need(not (BASE / 'consumer/session').exists() and not (BASE / 'consumer/units').exists(),
           'first inert consumer was never executed')
    refusal = r.load(BASE / 'consumer/first-inert/refusal.json')
    r.need(not refusal['passed'] and not refusal['native_executed'], 'first control failure retained')
    return recipe


def main():
    recipe = validate()
    raw = {}
    for context in ('consumer', 'consumer2'):
        folder = BASE / context
        for name in ('prepare', 'freeze', 'export', 'format', 'session', 'units', 'first-inert'):
            scope = folder / name
            if not scope.is_dir():
                continue
            for path in scope.rglob('*'):
                if path.is_file() and path.suffix != '.tar':
                    raw[context + '/' + name + '/' + path.relative_to(scope).as_posix()] = path.read_bytes()
            if (scope / 'intent.json').is_file():
                head = r.load(scope / 'intent.json')['head']
                for name in ('source_package.py', 'consumer_prepare.py', 'consumer_build_native.py',
                             'consumer_build_run.py', 'consumer_unit_native.py', 'consumer_unit_run.py',
                             'test_consumer.py', 'test_source_package.py'):
                    member = 'tools/dbus-native-transport-074/' + name
                    raw[context + '/' + scope.name + '/committed-producer/' + name] = \
                        subprocess.check_output(['git', 'show', head + ':' + member], cwd=ROOT)
        for name in ('Cargo.toml', 'Cargo.lock', '.cargo/config.toml',
                     'tools/long-run-supervisor-074/Cargo.toml',
                     'tools/long-run-supervisor-074/src/diagnostic_dispatch_negotiated_tests.rs',
                     'tools/long-run-supervisor-074/src/diagnostic_negotiated_credentials_adapter.rs'):
            raw[context + '/input/fd/' + name] = (folder / 'input/fd' / name).read_bytes()
        path = folder / 'units-inputs.json'
        if path.is_file():
            raw[context + '/units-inputs.json'] = path.read_bytes()
    for path in p.PATCHES.rglob('*'):
        if path.is_file():
            raw['patches/' + path.relative_to(p.PATCHES).as_posix()] = path.read_bytes()
    for path in Path(__file__).parent.iterdir():
        if path.is_file():
            raw['producer/' + path.name] = path.read_bytes()
    raw['export/first-receipt.json'] = (BASE / 'export/receipt.json').read_bytes()
    raw['parent/native-index.json'] = (r.OUT / 'index.json').read_bytes()
    raw['source/plan.md'] = (ROOT / 'docs/testing/performance/0.74/dbus-native-transport-plan-20261028.md').read_bytes()
    OUT.mkdir(exist_ok=False)
    objects = OUT / 'objects'
    objects.mkdir()
    members = {}
    for name, value in sorted(raw.items()):
        digest = r.sha(value)
        path = objects / digest
        if not path.exists():
            path.write_bytes(value)
        members[name] = dict(sha256=digest, bytes=len(value))
    value = dict(schema_version=1, files=members, root_changed=False, server=False,
        release_admitted=False, root_adopted=False, registry_qualified=False, unsigned=True,
        performance_improvement_proven=False, binaries_excluded=True,
        canonical_archives_reconstructible=True, source_package_recipe_sha256=p.RECIPE_SHA,
        source_archive_sha256=recipe['source_archive_sha256'], source_members=266,
        passes_per_compiler=65, private_scenarios_per_compiler=10,
        catalog_count=1593, filtered_per_compiler=1528,
        compilers=['GNU Rust 1.94', 'GNU Rust 1.88'], original_windows_seconds=[300, 60, 10],
        first_inert_control_refusal_retained=True, formatting_qualified=True)
    (OUT / 'index.json').write_bytes(p.codec.encoded(value))
    (OUT / '.gitattributes').write_text('objects/* -text -diff\nindex.json -text\n',
                                      encoding='ascii', newline='\n')
    print(json.dumps(dict(members=len(members), objects=len(list(objects.iterdir())),
                         index_sha256=r.sha((OUT / 'index.json').read_bytes()))))


if __name__ == '__main__':
    main()
