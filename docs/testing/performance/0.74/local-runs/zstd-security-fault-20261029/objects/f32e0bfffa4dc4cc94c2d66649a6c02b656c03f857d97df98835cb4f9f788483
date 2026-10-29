"""Retain native transport proofs and every consumed predecessor without binaries."""
from pathlib import Path
import hashlib
import json
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'target/dbus-native-20261028'
OUT = ROOT / 'docs/testing/performance/0.74/local-runs/dbus-native-transport-20261028'
CASES = ['bare-receiver', 'healthy', 'atomic-adapter', 'fragmented', 'lost-ack',
         'foreign-sender', 'wrong-serial', 'disconnect', 'manager-error', 'no-capability']
PKG = 'tools/long-run-supervisor-074'
SOURCE = ['Cargo.toml', 'src/lib.rs', 'src/diagnostic_blocking_v6.rs',
          'src/diagnostic_manager.rs', 'src/diagnostic_loaded.rs', 'src/systemd_unit.rs',
          'src/diagnostic_dispatch_negotiated_tests.rs',
          'src/diagnostic_dispatch_negotiated_recv_tests.rs',
          'src/diagnostic_negotiated_credentials_adapter.rs']


def need(ok, why):
    if not ok:
        raise ValueError(why)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def load(path):
    return json.loads(path.read_bytes())


def verify_cohort(folder, count):
    result = load(folder / 'native-result.json')
    need(result['passed'] and result['source_unchanged'] and result['boot_unchanged']
         and result['passes_per_compiler'] == count and result['catalog_count'] == 1593,
         'actual complete paired cohort: ' + str(folder))
    for compiler in ('194', '188'):
        test = folder / f'fd-{compiler}-test'
        value = load(test / 'result.json')
        need(value['exit'] == 0 and len(value['rows']) == count
             and len(set(value['selected'])) == count
             and sorted(n for n, state in value['rows'] if state == 'ok') == value['selected']
             and value['summary'] == [['ok', str(count), '0', '0', '0', str(1593-count)]],
             'exact pass/filter identities')
        for action in ('list', 'test', 'clippy'):
            phase = folder / f'fd-{compiler}-{action}'
            need(load(phase / 'result.json')['exit'] == 0, 'each complete phase passes')
            execution = load(phase / 'execution.json')
            need(execution['retired'] and not execution['errors'], 'each phase retires')
        if count == 65:
            text = (test / 'stdout.log').read_text()
            cases = re.findall(r'^HC074-NEGOTIATED case=([a-z-]+) result=ok ', text, re.M)
            need(cases == CASES and text.count('HC074-NATIVE-FD-FAULT native_cloexec=true '
                 'loss_injected=true guard_refused=true restored=true') == 1,
                 'all ten actual private scenarios and explicit flag fault')


def validate():
    strict = load(BASE / 'session/native-result.json')
    need(not strict['passed'] and strict['source_unchanged_except_resolved_lock']
         and strict['boot_unchanged'], 'first strict lock refusal remains failed')
    need(load(BASE / 'feature-audit/result.json')['passed'], 'separate saved-lock source review')
    floor = load(BASE / 'c/session/execution.json')
    need(floor['retired'] and floor['exit'] == 125 and floor['classification'] == 'resource-floor'
         and floor['errors'] == ['CapacityError: original 4 GiB floor']
         and not (BASE / 'c/session/native-result.json').exists(), 'first capacity result stays exact')
    first = load(BASE / 'r/private-bus/native-result.json')
    need(not first['passed'] and first['source_unchanged'] and first['boot_unchanged'],
         'first native characterization refusal remains failed')
    value = load(BASE / 'r/private-bus/fd-194-test/result.json')
    need(value['exit'] == 101 and value['summary'] == [['FAILED', '0', '1', '0', '0', '1592']],
         'first runtime failure not relabeled')
    for context in ('r', 'f2'):
        value = load(BASE / context / 'session/native-result.json')
        need(value['passed'] and value['source_unchanged'] and value['boot_unchanged'],
             'actual paired compiler context')
    verify_cohort(BASE / 'r/units', 64)
    verify_cohort(BASE / 'f2/units', 65)
    for context in ('f', 'f2'):
        value = load(BASE / context / 'freeze/result.json')
        need(value['changed_only_cfg_test'] and value['production_source_equal']
             and value['original_nine_cases_preserved'] and value['scenario_count'] == 10,
             'test-only projection')
    need(not (BASE / 'f/session').exists(), 'initial driver-render context never executed')


def main():
    validate()
    raw = {}
    for context in ('initial', 'c', 'r', 'f', 'f2'):
        folder = BASE if context == 'initial' else BASE / context
        for name in ('prepare', 'freeze', 'session', 'units', 'private-bus'):
            scope = folder / name
            if not scope.is_dir():
                continue
            for path in scope.rglob('*'):
                if path.is_file():
                    raw[f'{context}/{name}/' + path.relative_to(scope).as_posix()] = path.read_bytes()
            if (scope / 'intent.json').is_file():
                head = load(scope / 'intent.json')['head']
                need(re.fullmatch('[0-9a-f]{40}', head), 'actual committed dispatch')
                names = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', head,
                    'tools/dbus-native-transport-074'], cwd=ROOT).decode().splitlines()
                for member in names:
                    raw[f'{context}/{name}/committed-producer/' + Path(member).name] = \
                        subprocess.check_output(['git', 'show', head + ':' + member], cwd=ROOT)
        for name in SOURCE:
            path = folder / 'input/fd' / PKG / name
            if path.is_file():
                raw[f'{context}/input/fd/{PKG}/' + name] = path.read_bytes()
        for name in ('Cargo.toml', 'Cargo.lock', '.cargo/config.toml',
                     'source/zbus/Cargo.toml', 'source/zbus/Cargo.toml.orig',
                     'source/zbus/BACKPORT074.json', 'source/zbus/src/connection/socket/mod.rs'):
            path = folder / 'input/fd' / name
            if path.is_file():
                raw[f'{context}/input/fd/' + name] = path.read_bytes()
        for path in folder.glob('*-inputs.json'):
            raw[f'{context}/' + path.name] = path.read_bytes()
    for name in ('feature-audit',):
        for path in (BASE / name).rglob('*'):
            if path.is_file():
                raw[name + '/' + path.relative_to(BASE / name).as_posix()] = path.read_bytes()
    for path in (BASE / 'feature-audit-loader-refusal.json', BASE / 'review_saved.first.py',
                 BASE / 'f/fault_unit_native.first.py'):
        need(path.is_file(), 'retained first offline loader/render attempt')
        raw['first-offline/' + path.name] = path.read_bytes()
    for path in Path(__file__).parent.iterdir():
        if path.is_file():
            raw['producer/' + path.name] = path.read_bytes()
    raw['parent/legacy-index.json'] = (ROOT / 'docs/testing/performance/0.74/local-runs/'
                                     'dbus-legacy-integration-20261028/index.json').read_bytes()
    raw['source/plan.md'] = (ROOT / 'docs/testing/performance/0.74/'
                           'dbus-native-transport-plan-20261028.md').read_bytes()
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
    packet = dict(schema_version=1, files=members, root_changed=False, server=False,
        admission=False, performance_improvement_proven=False, binaries_excluded=True,
        old_zbus_removed_from_candidate_graph=True, original_windows_seconds=[300, 60, 10],
        first_refusals_retained=True, passes_per_compiler=65, private_scenarios_per_compiler=10,
        source_package_reconstruction_qualified=False, root_adoption=False)
    (OUT / 'index.json').write_text(json.dumps(packet, indent=2, sort_keys=True)+'\n',
                                  encoding='utf8', newline='\n')
    (OUT / '.gitattributes').write_text('objects/* -text -diff\nindex.json -text\n',
                                      encoding='ascii', newline='\n')
    print(json.dumps(dict(members=len(members), objects=len(list(objects.iterdir())),
                         index_sha256=sha((OUT / 'index.json').read_bytes()))))


if __name__ == '__main__':
    main()
