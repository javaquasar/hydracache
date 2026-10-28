"""Read the same consumed remote attempt and retain its unmodified first result."""
import importlib.util
import json
import subprocess
from pathlib import Path

import run

NAMES = ('started.json', 'preflight.json', 'execution.json', 'postflight.json',
         'result.json', 'live.stdout.log', 'live.stderr.log')


def collect():
    receipt = json.loads((run.OUT / 'remote.json').read_bytes())
    root = receipt['root']
    import re
    run.need(re.fullmatch(r'/tmp/hc074-preparation-root-20261028\.[A-Za-z0-9]{6}', root),
             'exact consumed root')
    dest = run.OUT / 'collection'
    dest.mkdir(exist_ok=False)
    for name in NAMES:
        run.put(dest / (name + '.intent.json'), dict(path=root + '/' + name, read_only=True))
        with (dest / name).open('xb') as stdout, (dest / (name + '.transport.stderr')).open('xb') as stderr:
            code = subprocess.run(run.SSH + ['sudo -n cat -- ' + root + '/' + name],
                                  stdout=stdout, stderr=stderr, timeout=45, check=False).returncode
        run.put(dest / (name + '.transport.json'), dict(exit=code, read_only=True))
        run.need(code == 0, 'first collection error: ' + name)
    result = json.loads((dest / 'result.json').read_bytes())
    execution = json.loads((dest / 'execution.json').read_bytes())
    run.need(execution['retired'] and not execution['errors'], 'owned CI retirement')
    run.need(json.loads((dest / 'preflight.json').read_bytes()) ==
             json.loads((dest / 'postflight.json').read_bytes()) and result['postflight_passed'],
             'host postflight')
    spec = importlib.util.spec_from_file_location('hc074_retained_profile_review',
                    run.ROOT / 'tools/native-server-control-074/profile_review.py')
    review = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(review)
    analysis = review.review((dest / 'live.stdout.log').read_bytes(),
                             execution['observed_child_exit'], result['exit'], 0)
    run.need(analysis['matrix_exit'] == 101 and analysis['original']['returned'] is False,
             'retain actual first refused profile, no success promotion')
    run.put(dest / 'analysis.json', analysis)
    print(json.dumps(dict(exit=result['exit'], phases=len(analysis['phases']),
                         first_failed_phase=analysis['first_failed_phase'],
                         postflight_passed=True, admission=False)))


def retain():
    dest = run.ROOT / 'docs/testing/performance/0.74/local-runs/server-preparation-profile-20261028'
    dest.mkdir(exist_ok=False)
    objects = dest / 'objects'
    objects.mkdir()
    sources = {'capture/' + p.relative_to(run.OUT).as_posix(): p
               for p in run.OUT.rglob('*') if p.is_file() and p.name not in
               ('libtest', 'timing-controls-074', 'payload.tar')}
    sources.update({'producer/' + p.name: p for p in Path(__file__).parent.iterdir() if p.is_file()})
    sources['source/approved-plan.md'] = run.ROOT / 'docs/testing/performance/0.74/server-preparation-profile-plan-20261028.md'
    sources['source/parent-original-manager-index.json'] = run.ROOT / 'docs/testing/performance/0.74/local-runs/original-manager-baseline-20261028/index.json'
    rows = {}
    for name, path in sorted(sources.items()):
        raw = path.read_bytes()
        digest = run.sha(path)
        held = objects / digest
        if not held.exists():
            with held.open('xb') as stream:
                stream.write(raw)
        rows[name] = dict(sha256=digest, bytes=len(raw))
    analysis = json.loads((run.OUT / 'collection/analysis.json').read_bytes())
    run.put(dest / 'index.json', dict(schema_version=1, files=rows, server=True,
        admission=False, product=False, original_send=False, retry=False,
        source_commit=json.loads((run.OUT / 'dispatch-once.json').read_bytes())['head'],
        image_sha256=run.IMAGE_SHA, original_windows_seconds=[300, 60, 10],
        first_failed_phase=analysis['first_failed_phase'], postflight_passed=True,
        binaries_excluded=True, performance_improvement_proven=False))
    with (dest / '.gitattributes').open('x', encoding='ascii', newline='\n') as stream:
        stream.write('objects/* -text\nindex.json -text\n')
    print(json.dumps(dict(members=len(rows), objects=len(list(objects.iterdir())),
                         index_sha256=run.sha(dest / 'index.json'))))


if __name__ == '__main__':
    import sys
    run.need(sys.argv[1:] in (['collect'], ['retain']), 'read/retention only')
    (collect if sys.argv[1:] == ['collect'] else retain)()
