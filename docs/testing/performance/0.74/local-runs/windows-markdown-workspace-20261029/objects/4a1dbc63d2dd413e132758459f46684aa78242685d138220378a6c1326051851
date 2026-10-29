"""Complete test-image accounting inside one source-validated CI batch.

The caller checks the complete source inventory around the batch, after the
complete build has also passed its source postflight. Each actual executable
is independently hash-checked and each command retains its own original
LinuxGroup, absolute deadline, capacity polling and retirement. No product
validation observation is coalesced or removed.
"""
from pathlib import Path
import json
import os
import re
import subprocess
import sys
import time

import common as c
import native as n


def invoke(out, label, argv, env, deadline):
    c.need(time.monotonic() < deadline, 'original test-batch deadline before spawn')
    n.capacity()
    c.need(time.monotonic() < deadline, 'original deadline after per-case preflight')
    folder = out / label
    folder.mkdir(exist_ok=False)
    c.write(folder, 'intent.json', dict(argv=argv, environment=env, deadline=deadline,
        bound=1200, session_bound=1100, floor=4*1024**3, poll=5, retirement=5, retry=False, cwd=str(c.ROOT/c.PKG)))
    with (folder/'stdout.log').open('xb') as stdout, (folder/'stderr.log').open('xb') as stderr:
        child = subprocess.Popen([sys.executable, '-B', str(Path(__file__)), 'worker', str(folder)],
            stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr, start_new_session=True)
        code = child.wait(timeout=max(0, deadline-time.monotonic())+10)
    execution = json.loads((folder/'execution.json').read_bytes())
    c.need(execution['retired'] and not execution['errors'], 'exact per-image owned retirement')
    c.write(folder, 'result.json', dict(exit=code, source_inventory_scope='complete-batch-boundaries',
        server=False, release_admitted=False))
    print(json.dumps(dict(phase=label, exit=code)), flush=True)
    c.need(code == 0, 'first complete test-batch refusal: '+label)
    return folder


def run(compiler, out, deadline):
    c.need(compiler in ('194', '188'), 'fixed qualified test compiler')
    images = json.loads((out/'images.json').read_bytes())
    _, env = n.environment(compiler)
    totals = dict(passed=0, ignored=0, catalog=0, targets=len(images))
    catalogs = {}
    for index, image in enumerate(images):
        name = f'target-{index:03d}'
        path = Path(image['path'])
        c.need(path.resolve().is_relative_to(n.NATIVE/'builds'/compiler)
               and c.sha(path.read_bytes()) == image['sha256'], 'held actual test image')
        listed = invoke(out, name+'-list', [str(path), '--list'], env, deadline)
        catalog = sorted(line[:-6] for line in (listed/'stdout.log').read_text().splitlines()
                         if line.endswith(': test'))
        c.need(len(catalog) == len(set(catalog)), 'unique complete target catalog')
        if image['kind'] == ['lib']:
            c.need(len(catalog) == 1593, 'original complete library catalog')
        tested = invoke(out, name+'-test', [str(path), '--test-threads=1'], env, deadline)
        value = n.account((tested/'stdout.log').read_bytes(), catalog)
        c.write(tested, 'accounting.json', value)
        c.need(c.sha(path.read_bytes()) == image['sha256'], 'actual image postflight')
        for key in ('passed', 'ignored', 'catalog'):
            totals[key] += value[key]
        catalogs[image['kind'][0]+'/'+image['name']] = catalog
    c.write(out, 'catalogs.json', catalogs)
    lib = next(v for v in images if v['kind'] == ['lib'])
    selected = [s for s in catalogs['lib/'+lib['name']]
                if s.endswith('::dispatch_negotiated_private_bus_fault_matrix')]
    c.need(len(selected) == 1, 'one original ignored private-bus matrix')
    c.need(c.sha(Path(lib['path']).read_bytes()) == lib['sha256'], 'held private-matrix image')
    fault = invoke(out, 'private-bus', [lib['path'], '--ignored', '--exact', selected[0], '--show-output'], env, deadline)
    raw = (fault/'stdout.log').read_text()
    cases = re.findall(r'^HC074-NEGOTIATED case=([a-z-]+) result=ok ', raw, re.M)
    c.need(cases == ['bare-receiver','healthy','atomic-adapter','fragmented','lost-ack','foreign-sender',
                    'wrong-serial','disconnect','manager-error','no-capability'], 'all ten actual private faults')
    c.need(re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', raw, re.M)
           == [('ok','1','0','0','0','1592')], 'exact ignored-matrix accounting')
    c.need(c.sha(Path(lib['path']).read_bytes()) == lib['sha256'], 'private-matrix image postflight')
    c.write(out, 'totals.json', totals)


if __name__ == '__main__':
    if len(sys.argv) == 3 and sys.argv[1] == 'worker':
        folder = Path(sys.argv[2])
        c.need(folder.resolve().is_relative_to(c.BASE.resolve()), 'owned actual case output')
        intent = json.loads((folder/'intent.json').read_bytes())
        os.environ.clear()
        os.environ.update(intent['environment'])
        path = c.ROOT/'scripts/ci/watchdog_linux_group.py'
        c.need(c.sha(path.read_bytes()) == n.GROUP_SHA, 'unchanged original per-case Linux owner')
        group = c.load(path, 'root_package_original_test_case_owner')
        c.need(time.monotonic() < intent['deadline'], 'existing case deadline before owner spawn')
        c.need(intent['cwd']==str(c.ROOT/c.PKG),'original Cargo test working directory')
        execution = c.owned()._owned(group.LinuxGroup(), intent['argv'], intent['cwd'], n.capacity,
                                    bound=1200, deadline=intent['deadline'])
        c.write(folder, 'execution.json', execution)
        raise SystemExit(execution['exit'])
    c.need(len(sys.argv) == 4 and sys.argv[1] == 'run', 'one complete compiler test batch')
    run(sys.argv[2], c.BASE/('tests-'+sys.argv[2]), float(sys.argv[3]))
