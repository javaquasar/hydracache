"""Apply qualified rustfmt to the single corrected fixture before a new freeze."""
from pathlib import Path
import json
import os
import subprocess
import sys

import common as c

OLD = c.ROOT/'target/dbus-root-package-20261029-spool7'
NEW = c.ROOT/'target/dbus-root-package-20261029-formatted8'
TOOLS = Path(__file__).parent
TEST = c.PKG+'/tests/diagnostic_spool.rs'
OUT = OLD/'fixture-format-2021'


def worker():
    _, env = c.windows_environment()
    tool = env['RUSTFMT']
    os.environ.clear()
    os.environ.update(env)
    owner = c.owned()
    execution = owner._owned(owner.load_job().WindowsJob(),
        [tool, '--edition', '2021', '--config', 'skip_children=true', str(c.ROOT/TEST)],
        str(c.ROOT), c.capacity, bound=60)
    c.write(OUT, 'execution.json', execution)
    raise SystemExit(execution['exit'])


def main():
    c.need(c.BASE == OLD and not NEW.exists(), 'fresh formatted fixture context')
    receipt = json.loads((OLD/'freeze/runtime-reviewed.json').read_bytes())
    c.guard(receipt)
    first = json.loads((OLD/'format/native-result.json').read_bytes())
    execution = json.loads((OLD/'format/execution.json').read_bytes())
    c.need(not first['passed'] and first['source_unchanged'] and first['boot_unchanged']
           and execution['retired'] and not execution['errors'], 'preserved first format-only refusal')
    OUT.mkdir(exist_ok=False)
    (OUT/'producer.py').write_bytes(Path(__file__).read_bytes())
    prior_body = (c.ROOT/TEST).read_bytes()
    (OUT/'before.rs').write_bytes(prior_body)
    with (OUT/'stdout.log').open('xb') as stdout, (OUT/'stderr.log').open('xb') as stderr:
        code = subprocess.call([sys.executable, '-B', str(Path(__file__)), 'worker'],
                               cwd=c.ROOT, stdout=stdout, stderr=stderr)
    execution = json.loads((OUT/'execution.json').read_bytes())
    c.need(code == 0 and execution['retired'] and not execution['errors'], 'qualified owned formatter')
    body = (c.ROOT/TEST).read_bytes()
    nl = b'\r\n' if b'\r\n' in prior_body else b'\n'
    expected = prior_body.replace(b'include_bytes!("fixtures/diagnostic-pilot-root-Cargo.lock")',
        b'include_bytes!('+nl+b'                "fixtures/diagnostic-pilot-root-Cargo.lock"'+nl+b'            )')
    c.need(expected == body, 'exact rustfmt line wrapping only')
    source = dict(receipt['source'])
    source[TEST] = c.sha(body)
    c.need(c.hashes(source) == source, 'only formatted test source changed')
    old_helpers = OUT/'old-helpers'
    old_helpers.mkdir()
    for name in receipt['helpers']:
        if name.startswith('tools/dbus-root-package-074/'):
            (old_helpers/Path(name).name).write_bytes((c.ROOT/name).read_bytes())
    common_body = (TOOLS/'common.py').read_text()
    old_scope = "BASE = ROOT / 'target/dbus-root-package-20261029-spool7'"
    c.need(common_body.count(old_scope) == 1, 'exact CI output scope')
    (TOOLS/'common.py').write_text(common_body.replace(old_scope,
        "BASE = ROOT / 'target/dbus-root-package-20261029-formatted8'"), encoding='utf8', newline='\n')
    retention_body = (TOOLS/'retain.py').read_text()
    c.need(retention_body.count("(c.BASE, 'spool7')):") == 1, 'exact previous retention scope')
    (TOOLS/'retain.py').write_text(retention_body.replace("(c.BASE, 'spool7')):",
        "(c.ROOT/'target/dbus-root-package-20261029-spool7', 'spool-format-first'), (c.BASE, 'formatted8')):"),
        encoding='utf8', newline='\n')
    names = set(receipt['helpers']) | {p.relative_to(c.ROOT).as_posix() for p in TOOLS.iterdir() if p.is_file()}
    helpers = {name:c.sha((c.ROOT/name).read_bytes()) for name in sorted(names)}
    NEW.mkdir(exist_ok=False)
    (NEW/'freeze').mkdir()
    (NEW/'seed').mkdir()
    for name in ('result.json','native-result.json'):
        origin = c.ROOT/'target/dbus-root-package-20261028/seed'/name
        value = json.loads(origin.read_bytes())
        c.need(value['passed'], 'verified immutable registry seed only')
        value.update(reused_from=str(origin), reused_sha256=c.sha(origin.read_bytes()), phase='seed-reuse')
        c.write(NEW/'seed', name, value)
    value = dict(receipt, source=source, helpers=helpers,
        previous_freeze_sha256=c.sha((OLD/'freeze/runtime-reviewed.json').read_bytes()),
        only_formatting_since_previous=True, complete_gate_reuse={}, partial_tests_reused=False)
    c.write(NEW/'freeze', 'runtime-reviewed.json', value)
    c.write(OUT, 'result.json', dict(passed=True, before_sha256=c.sha(prior_body), after_sha256=c.sha(body),
        tokens_unchanged=True, runtime_changed=False, windows_changed=False, release_admitted=False))
    print(json.dumps(dict(context=str(NEW), formatting_applied=True, runtime_changed=False)))


if __name__ == '__main__':
    if sys.argv[1:] == ['worker']:
        worker()
    else:
        c.need(not sys.argv[1:], 'one exact fixture-format application')
        main()
