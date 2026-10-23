"""Bounded pure parser processes in the already running local Ubuntu only."""
from pathlib import Path
import hashlib
import json
import math
import statistics
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'target/status-field-scan-20261024'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')


def call(folder, lane, mode, label):
    build = OUT / lane
    receipt = json.loads((build / 'result.json').read_bytes())
    if receipt['exit'] or digest(build / 'parser') != receipt['image_sha256']:
        raise ValueError('Unverified parser image')
    for name, sha in json.loads((build / 'intent.json').read_bytes())['inputs'].items():
        if digest(build / name) != sha:
            raise ValueError('Changed build input')
    wsl_path = '/mnt/' + build.drive[0].lower() + build.as_posix()[2:] + '/parser'
    argv = ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--cd', '/', '--exec', '/usr/bin/env',
            '-i', 'PATH=/usr/bin:/bin', '/usr/bin/timeout', '--signal=TERM', '--kill-after=2s', '20s', wsl_path, mode]
    write(folder / (label + '-intent.json'), dict(argv=argv, image_sha256=receipt['image_sha256'], retry=False))
    with (folder / (label + '.stdout.log')).open('xb') as stdout, (folder / (label + '.stderr.log')).open('xb') as stderr:
        result = subprocess.run(argv, stdout=stdout, stderr=stderr, timeout=45, check=False)
    write(folder / (label + '-result.json'), dict(exit=result.returncode, live=False, server=False, admission=False, retry=False))
    if result.returncode:
        raise ValueError('First parser process refused; do not retry')
    raw = (folder / (label + '.stdout.log')).read_bytes()
    if mode == 'check':
        return raw
    rows = [json.loads(line) for line in raw.splitlines()]
    if len(rows) != 3 or [r['shape'] for r in rows] != [512, 2048, 8192]:
        raise ValueError('Measurement shape drift')
    for row in rows:
        if (row['iterations'] != 20_000 or row['seed'] != 0x07405ca1
                or row['admission'] is not False or row['product'] is not False
                or type(row['cpu_ns']) is not int or row['cpu_ns'] <= 0):
            raise ValueError('Workload or CPU clock drift')
    return rows


def estimate(pairs):
    if len(pairs) != 5:
        raise ValueError('Incomplete five-pair cohort')
    results = []
    for slot, shape in enumerate((512, 2048, 8192)):
        ratios = []
        for pair in pairs:
            a, b = pair['baseline'][slot], pair['candidate'][slot]
            if any(a[k] != b[k] for k in ('shape', 'iterations', 'seed', 'workload_witness')):
                raise ValueError('Unmatched workloads')
            ratios.append(b['cpu_ns'] / a['cpu_ns'])
        logs = [math.log(ratio) for ratio in ratios]
        mean = statistics.mean(logs)
        radius = 2.7764451051977987 * statistics.stdev(logs) / math.sqrt(5)
        results.append(dict(shape=shape, paired_cpu_ratios=ratios, geometric_mean=math.exp(mean),
            lower_95=math.exp(mean - radius), upper_95=math.exp(mean + radius), floor=0.85,
            passed=math.exp(mean + radius) <= 0.85))
    return results


def run(stage):
    if stage not in ('baseline-first', 'paired'):
        raise ValueError('Unknown separate stage')
    folder = OUT / stage
    folder.mkdir()  # The first outcome is consumed before launching anything.
    if stage == 'baseline-first':
        transcript = call(folder, 'baseline', 'check', 'semantic')
        rows = call(folder, 'baseline', 'measure', 'measurement')
        write(folder / 'summary.json', dict(rows=rows, semantic_sha256=hashlib.sha256(transcript).hexdigest(),
            live=False, server=False, admission=False, product=False, retry=False))
        print(rows)
        return
    transcripts = [call(folder, lane, 'check', lane + '-semantic') for lane in ('baseline', 'candidate')]
    if transcripts[0] != transcripts[1]:
        raise ValueError('Semantic transcript differs; no measurement')
    pairs = []
    for index in range(5):
        pair = {}
        order = ('baseline', 'candidate') if index % 2 == 0 else ('candidate', 'baseline')
        for lane in order:
            pair[lane] = call(folder, lane, 'measure', f'pair-{index}-{lane}')
        pairs.append(pair)
    results = estimate(pairs)
    write(folder / 'summary.json', dict(pairs=pairs, results=results, passed=all(row['passed'] for row in results),
        semantic_sha256=hashlib.sha256(transcripts[0]).hexdigest(), live=False, server=False,
        admission=False, product=False, retry=False, whole_runtime_gain_proven=False))
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    run(sys.argv[1])
