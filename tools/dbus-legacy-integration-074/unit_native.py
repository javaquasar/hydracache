"""Local exact 64-case integration gate, no ignored/live/service selection."""
from pathlib import Path
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
import time

ROOT = Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
BASE = ROOT / 'target/dbus-legacy-integration-20261028/tests'
NATIVE = Path('/home/artur/.cache/hydracache-074-dbus-legacy-integration/20261028-a7')
OUT = BASE / 'units'
FILES = ('diagnostic_held_delivery_wire_tests.rs', 'diagnostic_original_manager_message_units.rs',
         'diagnostic_original_pipe_transport_tests.rs', 'diagnostic_dispatch_exchange_tests.rs',
         'diagnostic_dispatch_reply_flags_tests.rs', 'diagnostic_dispatch_reply_state_tests.rs',
         'diagnostic_original_reply_binding_tests.rs')
MODULES = ('diagnostic_manager', 'diagnostic_loaded', 'systemd_unit')


def need(value, reason):
    if not value:
        raise ValueError(reason)


def guard():
    pins = json.loads((BASE / 'units-inputs.json').read_bytes())
    for name, digest in pins['files'].items():
        need(hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == digest, 'fixed unit driver drift')
    need(hashlib.sha256((BASE / 'freeze/result.json').read_bytes()).hexdigest() == pins['freeze_sha256'],
         'unchanged compiled source receipt')


def select(catalog, source):
    selected = []
    for name in FILES:
        for leaf in re.findall(r'#\[test\]\s*fn (\w+)', (source / name).read_text()):
            found = [n for n in catalog if n.endswith('::' + leaf)]
            need(len(found) == 1, 'one exact original wire leaf: ' + leaf)
            selected.extend(found)
    need(len(selected) == 40, 'all forty original wire cases')
    for module in MODULES:
        for leaf in re.findall(r'#\[test\]\s*fn (\w+)', (source / (module + '.rs')).read_text()):
            name = module + '::tests::' + leaf
            need(catalog.count(name) == 1, 'one original caller case: ' + name)
            selected.append(name)
    leaf = 'negotiated_credential_adapter_keeps_optional_fields_and_process_fd_alias'
    found = [n for n in catalog if n.endswith('::' + leaf)]
    need(len(found) == 1, 'one retained credential control')
    selected.extend(found)
    need(len(selected) == len(set(selected)) == 64, 'exact 23 caller plus 41 wire/DTO set')
    need(not any('privileged_' in n or 'original_service::' in n for n in selected), 'no live selector')
    return sorted(selected)


def validate_accounting(rows, summaries, catalog, selected, code):
    need(len(catalog) == len(set(catalog)) == 1593 and len(selected) == len(set(selected)) == 64,
         'exact complete/selected catalogs')
    need(len(rows) == 64 and sorted(n for n, state in rows) == selected
         and all(state == 'ok' for _, state in rows), 'all original selected cases pass')
    need(summaries == [('ok', '64', '0', '0', '0', '1529')] and code == 0,
         'exact pass/filtered/exit accounting')


def configured():
    guard()
    path = ROOT / 'tools/dbus-linux-074/native.py'
    need(hashlib.sha256(path.read_bytes()).hexdigest() ==
         '09ab4fb404b93af4ec5581b189d3e7fe54562fff14626bfabcbc07c11c9f51b1', 'original CI owner')
    spec = importlib.util.spec_from_file_location('legacy_unit_owned_phase', path)
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    owner.BASE, owner.NATIVE, owner.OUT = BASE, NATIVE, OUT
    owner.prepared = lambda: BASE / 'freeze/result.json'
    original = owner.command

    def command(label, review):
        parts = label.split('-')
        need(len(parts) == 3 and parts[0] == 'fd' and parts[1] in ('194', '188')
             and parts[2] in ('build', 'list', 'test', 'clippy'), 'fixed unit phase')
        argv, env, artifact, target, folder, action = original(label, review)
        if action in ('build', 'clippy'):
            argv[argv.index('dbus-probe-074')] = 'hydracache-long-run-supervisor-074'
        if action == 'clippy':
            at = argv.index('--all-targets')
            argv[at:at + 1] = ['--lib', '--tests']
        if action == 'test':
            listing = json.loads((OUT / ('fd-' + parts[1] + '-list/result.json')).read_bytes())
            need(len(listing['selected']) == 64, 'held exact catalog')
            argv += ['--exact', *listing['selected']]
        return argv, env, artifact, target, folder, action

    owner.command = command
    return owner


def session():
    owner = configured()
    review = json.loads(owner.prepared().read_bytes())
    deadline = time.monotonic() + 1100
    os.umask(0o077)
    result = dict(passed=False, server=False, services=False, clocks_changed=False,
                  release_admitted=False, tests_executed=True, phases=[],
                  boot_before=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                  clocks_before=owner.clocks())
    try:
        need(os.getuid() == os.getgid() == 1000, 'nonroot original local Linux')
        checked = json.loads((BASE / 'session/native-result.json').read_bytes())
        need(checked['passed'] and checked['source_unchanged'] and checked['boot_unchanged'],
             'complete paired library check/Clippy first')
        catalogs = []
        for compiler in ('194', '188'):
            for action in ('build', 'list', 'test', 'clippy'):
                need(time.monotonic() < deadline, 'original absolute session bound')
                guard()
                owner.sources(review)
                owner.capacity()
                label = f'fd-{compiler}-{action}'
                out = OUT / label
                out.mkdir()
                argv, env, artifact, target, folder, _ = owner.command(label, review)
                owner.write(out, 'intent.json', dict(argv=argv, environment=env, artifact=artifact,
                    absolute_deadline=deadline, bound=1200, session_bound=1100, floor=4*1024**3,
                    poll=5, retirement=5, retry=False, selected_only=True))
                with (out / 'stdout.log').open('xb') as stdout, (out / 'stderr.log').open('xb') as stderr:
                    child = subprocess.Popen(['/usr/bin/python3', '-B', str(Path(__file__)),
                        'worker', label, str(deadline)], stdin=subprocess.DEVNULL,
                        stdout=stdout, stderr=stderr, start_new_session=True)
                    code = child.wait(timeout=max(0, deadline-time.monotonic())+10)
                execution = json.loads((out / 'execution.json').read_bytes())
                need(execution['retired'] and not execution['errors'], 'owned phase retirement')
                owner.sources(review)
                value = dict(exit=code, artifact=artifact, source_unchanged=True, selected_only=True)
                raw = (out / 'stdout.log').read_text()
                if action == 'build' and code == 0:
                    messages = [json.loads(line) for line in raw.splitlines()]
                    found = [m for m in messages if m.get('reason') == 'compiler-artifact' and
                        m['target']['name'] == 'hydracache_long_run_supervisor_074' and
                        m['target']['kind'] == ['lib'] and m['profile']['test'] and m.get('executable')]
                    need(len(found) == 1 and [m['success'] for m in messages if m.get('reason') ==
                         'build-finished'] == [True], 'actual complete test image')
                    image = Path(found[0]['executable'])
                    need(image.resolve().is_relative_to(target) and Path(found[0]['target']['src_path']) ==
                         folder / 'tools/long-run-supervisor-074/src/lib.rs', 'actual image/source path')
                    value['artifact'] = dict(path=str(image), bytes=image.stat().st_size,
                                             sha256=owner.sha(image.read_bytes()))
                if action == 'list' and code == 0:
                    catalog = sorted(line[:-6] for line in raw.splitlines() if line.endswith(': test'))
                    need(len(catalog) == len(set(catalog)) == 1593, 'complete original catalog retained')
                    value.update(inventory=catalog, selected=select(catalog,
                        NATIVE / 'fd/tools/long-run-supervisor-074/src'))
                    catalogs.append(catalog)
                if action == 'test':
                    listing = json.loads((OUT / f'fd-{compiler}-list/result.json').read_bytes())
                    rows = re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)$', raw, re.M)
                    summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
                        r'(\d+) ignored; (\d+) measured; (\d+) filtered out;', raw, re.M)
                    value.update(rows=rows, summary=summaries, selected=listing['selected'])
                    owner.write(out, 'accounting.json', value)
                    validate_accounting(rows, summaries, listing['inventory'], listing['selected'], code)
                if artifact:
                    need(owner.sha(Path(artifact['path']).read_bytes()) == artifact['sha256'],
                         'actual native image postflight')
                owner.write(out, 'result.json', value)
                result['phases'].append(dict(label=label, exit=code))
                print(json.dumps(dict(phase=label, exit=code)), flush=True)
                need(code == 0, 'first unit phase refusal: ' + label)
        need(len(catalogs) == 2 and catalogs[0] == catalogs[1], 'paired full compiler catalogs')
        result.update(passed=True, catalog_count=1593, passes_per_compiler=64)
    except BaseException as error:
        result['first_refusal'] = dict(type=type(error).__name__, reason=str(error))
    finally:
        try:
            owner.sources(review)
            guard()
            result['source_unchanged'] = True
        except BaseException as error:
            result.update(source_unchanged=False, passed=False, source_refusal=str(error))
        result.update(boot_after=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                      clocks_after=owner.clocks())
        result['boot_unchanged'] = result['boot_before'] == result['boot_after']
        owner.write(OUT, 'native-result.json', result)
    raise SystemExit(0 if result['passed'] and result['boot_unchanged'] else 1)


if __name__ == '__main__':
    if len(sys.argv) == 4 and sys.argv[1] == 'worker':
        configured().worker(sys.argv[2], float(sys.argv[3]))
        raise SystemExit('worker exits')
    need(sys.argv[1:] == ['session'], 'fixed unit session')
    session()
