"""Single-use read-only WSL clock observation, separate from original-owner admission."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
OUT = ROOT / 'target/wsl-clock-readonly-20261023'
ARCHIVE = ROOT / 'docs/testing/performance/0.74/local-runs/wsl-clock-readonly-20261023'
QUAL = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
OLD = '9f2fd0f457ec6d48f4e87e7bfc2bd1262bbf191cf51b760fe744eb369394b9f6'
FILES = [HERE / n for n in ('sample.rs', 'session.sh', 'diagnose.py', 'test_diagnose.py')]
FILES += [ROOT / 'tools/long-run-supervisor-074/src' / n for n in
          ('diagnostic_kernel_origin.rs', 'diagnostic_kernel_admission.rs')]
FILES += [ROOT / 'docs/testing/performance/0.74' / n for n in
          ('qualification-manifest.toml', 'local-runs/bootstrap-original-getunit-20261023/index.json')]
NAMES = ('boottime', 'realtime', 'monotonic', 'monotonic_raw')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def put(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def frozen():
    if sha(FILES[-2]) != QUAL or sha(FILES[-1]) != OLD:
        raise ValueError('Frozen manifest or first-refusal evidence changed')
    subprocess.run(['git', 'diff', '--exit-code', '8fb0fb5a', '--',
        'tools/long-run-supervisor-074/src', 'tools/long-run-supervisor-074/tests',
        'Cargo.lock', 'docs/COMPAT.md'], cwd=ROOT, check=True, capture_output=True)
    return {p.relative_to(ROOT).as_posix(): sha(p) for p in FILES}


def linux(path):
    value = path.resolve().as_posix()
    if not value.startswith('C:/Workspace/prj/jq/cashe/hydracache-074/'):
        raise ValueError('Not this workspace')
    return '/mnt/c/' + value[3:]


def command(name, args, timeout=60, required=True):
    with (OUT / (name + '.stdout.log')).open('xb') as out, (OUT / (name + '.stderr.log')).open('xb') as err:
        try:
            rc = subprocess.run(args, cwd=ROOT, stdout=out, stderr=err,
                                timeout=timeout, check=False).returncode
        except subprocess.TimeoutExpired:
            rc = 124  # Retain first failure; never automatically redispatch.
    result = {'argv': args, 'exit': rc,
              'stdout_sha256': sha(OUT / (name + '.stdout.log')),
              'stderr_sha256': sha(OUT / (name + '.stderr.log'))}
    put(OUT / (name + '.json'), result)
    if rc and required:
        raise RuntimeError(f'{name}: first exit {rc}; see retained logs')


def build():
    OUT.mkdir(parents=True, exist_ok=False)
    before = frozen()
    command('python-controls', [sys.executable, '-B', '-m', 'unittest', 'discover', '-s', str(HERE), '-v'])
    command('rustfmt', ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec',
        '/home/artur/.cargo/bin/rustfmt', '+1.94.0', '--edition', '2021', '--check', linux(HERE / 'sample.rs')])
    for version in ('1.94.0', '1.88.0'):
        base = ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec',
                '/home/artur/.cargo/bin/rustc', '+' + version, '--edition=2021', '-Dwarnings']
        suffix = version.replace('.', '-')
        command('compile-' + suffix, base + ['-O', linux(HERE / 'sample.rs'), '-o', linux(OUT / ('sample-' + suffix))])
        command('compile-test-' + suffix, base + ['--test', linux(HERE / 'sample.rs'), '-o', linux(OUT / ('test-' + suffix))])
        command('unit-' + suffix, ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec', linux(OUT / ('test-' + suffix))])
    # The GNU sampler is copied only inside the workspace, never installed.
    with (OUT / 'sample').open('xb') as stream:
        stream.write((OUT / 'sample-1-94-0').read_bytes())
    command('shell-syntax', ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec', '/bin/sh', '-n', linux(HERE / 'session.sh')])
    if before != frozen():
        raise ValueError('Source drift during build')
    put(OUT / 'build.json', {'sources': before, 'binary_sha256': sha(OUT / 'sample')})


def identity(raw):
    lines = raw.splitlines()
    # /proc/1/stat field 22 is starttime; comm may contain spaces/parentheses.
    tail = lines[1][lines[1].rindex(')') + 2:].split()
    return {'boot_id': lines[0], 'pid1_start_ticks': int(tail[19]),
            'namespaces_and_time_offsets': lines[2:]}


def analyze(raw):
    lines = [json.loads(line) for line in raw.splitlines()]
    header, rows = lines[0], lines[1:]
    if header != {'schema': 'hc074-clock-readonly-v1', 'samples': 601, 'period_ms': 100,
                  'clock_ids': [7, 0, 1, 4], 'order': 'clock-major libc-before,syscall,libc-after'}:
        raise ValueError('Changed observer method')
    if len(rows) != 601:
        raise ValueError(f'Incomplete first observation: {len(rows)} rows')
    stats = {n: {'backwards_by_api': [0, 0, 0], 'bracket_violations': 0,
                 'minimum_delta_by_api_ns': [None] * 3, 'maximum_delta_by_api_ns': [None] * 3,
                 'max_libc_bracket_ns': 0} for n in NAMES}
    first = None
    cpus = set()
    def event(value):
        nonlocal first
        if first is None:
            first = value
    for i, row in enumerate(rows):
        if row['i'] != i or len(row['cpu']) != 2 or len(row['clocks']) != 4:
            raise ValueError('Invalid or reordered observer row')
        if any(type(c) is not int or c < 0 for c in row['cpu']):
            raise ValueError('Invalid CPU observation')
        cpus.update(row['cpu'])
        for c, name in enumerate(NAMES):
            values = row['clocks'][c]
            if len(values) != 3 or any(type(v) is not int or v < 0 for v in values):
                raise ValueError('Invalid clock observation')
            s = stats[name]
            s['max_libc_bracket_ns'] = max(s['max_libc_bracket_ns'], values[2] - values[0])
            if not values[0] <= values[1] <= values[2]:
                s['bracket_violations'] += 1
                event({'row': i, 'kind': 'syscall_outside_libc_bracket', 'clock': name, 'values': values})
            if i:
                for api, (now, before) in enumerate(zip(values, rows[i - 1]['clocks'][c])):
                    delta = now - before
                    low = s['minimum_delta_by_api_ns'][api]
                    high = s['maximum_delta_by_api_ns'][api]
                    s['minimum_delta_by_api_ns'][api] = delta if low is None else min(low, delta)
                    s['maximum_delta_by_api_ns'][api] = delta if high is None else max(high, delta)
                    if delta < 0:
                        s['backwards_by_api'][api] += 1
                        event({'row': i, 'kind': 'backwards', 'clock': name, 'api': api, 'delta_ns': delta})
    # Non-simultaneous differences, not an exact clock offset or drift estimator.
    residuals = [(b['clocks'][1][1] - a['clocks'][1][1]) -
                 (b['clocks'][0][1] - a['clocks'][0][1]) for a, b in zip(rows, rows[1:])]
    return {'rows': len(rows), 'stats': stats, 'first_anomaly': first, 'cpus': sorted(cpus),
            'cpu_endpoint_changes': sum(r['cpu'][0] != r['cpu'][1] for r in rows),
            'direct_realtime_minus_boottime_delta_residual_ns': [min(residuals), max(residuals)],
            'direct_elapsed_ns': {n: rows[-1]['clocks'][c][1] - rows[0]['clocks'][c][1]
                                  for c, n in enumerate(NAMES)},
            'admission': False, 'caused_by_specific_service': None}


def observe():
    proof = json.loads((OUT / 'build.json').read_bytes())
    if proof['sources'] != frozen() or sha(OUT / 'sample') != proof['binary_sha256']:
        raise ValueError('Checked source or binary drift')
    put(OUT / 'intent.json', {'sources': frozen(), 'binary': proof['binary_sha256'],
        'git': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'distro': 'HydraCache074Native', 'original_owner_run': False, 'retry': False,
        'clock_change': False, 'service_change': False, 'server': False, 'admission': False,
        'samples': 601, 'period_ms': 100, 'helper_timeout_seconds': 80, 'outer_timeout_seconds': 120})
    command('wsl-version', ['wsl.exe', '--version'])
    # A stopped/unavailable Windows time service is metadata, not a reason to retry.
    command('windows-time-status', ['w32tm.exe', '/query', '/status'], required=False)
    command('session', ['wsl.exe', '-d', 'HydraCache074Native', '-u', 'root', '--cd', '/',
            '--exec', '/bin/sh', linux(HERE / 'session.sh'), linux(OUT / 'sample'), linux(OUT)], 120)
    report()


def report():
    before = identity((OUT / 'identity-before.txt').read_text())
    after = identity((OUT / 'identity-after.txt').read_text())
    result = analyze((OUT / 'samples.jsonl').read_text())
    result.update(identity_before=before, identity_after=after, same_session=before == after)
    if not result['same_session']:
        raise ValueError('WSL init identity changed: never mix sessions')
    if json.loads((OUT / 'build.json').read_bytes())['sources'] != frozen():
        raise ValueError('Sources changed during observation')
    put(OUT / 'analysis.json', result)
    print(json.dumps(result))


def retain():
    frozen()
    if not (OUT / 'analysis.json').is_file():
        raise ValueError('No complete analysis; preserve incomplete raw result separately')
    ARCHIVE.mkdir(parents=True, exist_ok=False)
    (ARCHIVE / 'objects').mkdir()
    (ARCHIVE / '.gitattributes').write_bytes(b'objects/* -text -diff -whitespace\n*.json text eol=lf\n')
    files = {}
    def add(name, source):
        raw = source.read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        obj = ARCHIVE / 'objects' / digest
        if not obj.exists():
            with obj.open('xb') as stream:
                stream.write(raw)
        files[name] = {'sha256': digest, 'bytes': len(raw)}
    for p in sorted(OUT.iterdir()):
        if p.is_file() and p.suffix in ('.json', '.jsonl', '.txt', '.log'):
            add('observation/' + p.name, p)
    for p in FILES:
        add('source/' + p.relative_to(ROOT).as_posix(), p)
    put(ARCHIVE / 'index.json', {'schema': 'hc074-clock-readonly-evidence-v1', 'files': files,
        'first_refusal_index_sha256': OLD, 'qualification_sha256': QUAL,
        'admission': False, 'server': False, 'product': False, 'original_owner_run': False})
    verify()


def verify():
    frozen()
    index = json.loads((ARCHIVE / 'index.json').read_bytes())
    if index['admission'] or index['server'] or index['product'] or index['original_owner_run']:
        raise ValueError('Not read-only diagnostic evidence')
    for entry in index['files'].values():
        digest = entry['sha256']
        if len(digest) != 64 or any(c not in '0123456789abcdef' for c in digest):
            raise ValueError('Invalid digest')
        path = ARCHIVE / 'objects' / digest
        if sha(path) != digest or path.stat().st_size != entry['bytes']:
            raise ValueError('Evidence object changed')
    print(json.dumps({'files': len(index['files']), 'index_sha256': sha(ARCHIVE / 'index.json'), 'verified': True}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('build', 'observe', 'analyze', 'retain', 'verify'))
    args = parser.parse_args()
    {'build': build, 'observe': observe, 'analyze': report, 'retain': retain, 'verify': verify}[args.mode]()
