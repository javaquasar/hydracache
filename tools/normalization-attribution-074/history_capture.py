"""Bounded first-only observation of four existing historical prefixes."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
PATHS = tuple('tools/long-run-supervisor-074/src/diagnostic_' + name + '.rs'
              for name in ('authority_registry', 'kernel_origin', 'kernel_admission'))
OPERATIONS = ('authority-context-prefix', 'authority-hex-prefix',
              'remaining-preparation-prefix', 'document-scratch-prefix')
BASELINE = 'd2984650'


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def review(raw, *, namespace=False):
    lines = raw.decode().splitlines()
    def rows(marker):
        return [json.loads(line[len(marker):]) for line in lines if line.startswith(marker)]
    prep, samples, done = (rows(marker) for marker in
        ('HC074_HISTORY_PREP ', 'HC074_HISTORY ', 'HC074_HISTORY_DONE '))
    count = 18 if namespace else 72
    operations = ('namespace-chain',) if namespace else OPERATIONS
    require(len(prep) == 3 and len(samples) == count, 'fixed complete scope')
    expected = dict(samples=count, blocks=6, iterations=16, prefix_warmup=False,
        disjoint_intervals=False, full_fixed_slice_chain=False, product=False, package_replay=False)
    require(done == [expected] and all(type(done[0][k]) is type(v) for k, v in expected.items()), 'completion')
    def integer(row, key, minimum=0):
        require(type(row.get(key)) is int and row[key] >= minimum, key)
    def digest(value):
        require(isinstance(value, str) and len(value) == 64 and
                all(char in '0123456789abcdef' for char in value), 'digest')
    inputs, outputs, groups = {}, {}, {}
    for path, row in zip(PATHS, prep):
        require(row.get('path') == path and row.get('prefix_warmup') is False, 'preparation order')
        integer(row, 'wall_ns')
        integer(row, 'input_bytes', 1)
        digest(row.get('input_sha256'))
        inputs[path] = row['input_bytes']
    index = 0
    for block in range(6):
        for path in PATHS:
            for operation in operations if block % 2 == 0 else operations[::-1]:
                row = samples[index]
                index += 1
                integer(row, 'block')
                require((row['block'], row.get('path'), row.get('operation')) == (block, path, operation), 'sample order')
                integer(row, 'iterations', 1)
                require(row['iterations'] == 16, 'iteration count')
                for metric in ('wall_ns', 'parent_cpu_ns', 'output_bytes', 'input_bytes'):
                    integer(row, metric)
                require(row['input_bytes'] == inputs[path], 'input length drift')
                digest(row.get('output_sha256'))
                key = path + '/' + operation
                output = row['output_bytes'], row['output_sha256']
                require(outputs.setdefault(key, output) == output, 'output drift')
                groups.setdefault(key, []).append(row)
    summary = {key: {metric: dict(samples=len(group),
        median_per_call=statistics.median(row[metric] / 16 for row in group),
        min_per_call=min(row[metric] / 16 for row in group),
        max_per_call=max(row[metric] / 16 for row in group))
        for metric in ('wall_ns', 'parent_cpu_ns')} for key, group in groups.items()}
    result = dict(preparation=prep, samples=samples, summary=summary,
        nested_prefixes_not_disjoint=True, observer_subtracted=False,
        semantic_equivalence_of_different_prefixes_claimed=False,
        independent_host_replications=False, full_fixed_slice_chain=False,
        product=False, package_replay=False, production_admission=False)
    if namespace:
        result['namespace_chain'] = True
    return result


def capture(build, out, *, namespace=False):
    out.mkdir(parents=True, exist_ok=False)
    def write(name, value):
        with (out / name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write('\n')
    count = 18 if namespace else 72
    operations = ('namespace-chain',) if namespace else OPERATIONS
    write('intent.json', dict(paths=PATHS, operations=operations, blocks=6, samples=count,
        namespace_chain=namespace,
        iterations=16, timeout_seconds=180, disk_floor_bytes=4 * 1024**3,
        prefix_warmup=False, retry=False, server=False, services=False, clocks=False))
    process = None
    try:
        require(os.name == 'nt' and shutil.disk_usage(ROOT).free >= 4 * 1024**3, 'platform/disk preflight')
        require(not any(key.startswith(('HC074_', 'RUST_TEST_')) for key in os.environ), 'inherited fixture overrides')
        result = json.loads((build / 'result.json').read_bytes())
        source = json.loads((build / 'intent.json').read_bytes())['source']
        require(result['exit'] == 0 and result['source_unchanged'] and result['head_unchanged'], 'build receipt')
        require(all(sha((ROOT / name).read_bytes()) == digest for name, digest in source.items()), 'compiled source drift')
        head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip()
        artifacts = [json.loads(line) for line in (build / 'stdout.log').read_text().splitlines() if line.startswith('{')]
        artifact, = [row for row in artifacts if row.get('reason') == 'compiler-artifact'
                     and row.get('target', {}).get('name') == 'authority_context_074']
        binary = Path(artifact['executable']).resolve()
        require(binary.parent == (ROOT / 'target/debug/deps').resolve(), 'binary path')
        paths = []
        for name in PATHS:
            committed = subprocess.check_output(['git', 'show', BASELINE + ':' + name], cwd=ROOT)
            current = (ROOT / name).read_bytes()
            require(committed.replace(b'\r\n', b'\n') == current.replace(b'\r\n', b'\n'), 'baseline input drift')
            paths.append(dict(path=name, sha256=sha(current), committed_sha256=sha(committed)))
        write('pins.json', dict(paths=paths, production_authority=False, namespace_chain=namespace))
        write('preflight.json', dict(head=head, binary_sha256=sha(binary.read_bytes()), compiled_source=source))
        with (out / 'stdout.log').open('xb') as stdout, (out / 'stderr.log').open('xb') as stderr:
            child = 'history_namespace_cost_child' if namespace else 'history_prefix_cost_child'
            process = subprocess.Popen([str(binary), '--exact', child, '--nocapture', '--test-threads=1'],
                cwd=ROOT, stdout=stdout, stderr=stderr,
                env=dict(os.environ, HC074_HISTORY_CHAIN_INPUT=str((out / 'pins.json').resolve())))
            started = time.monotonic()
            while process.poll() is None:
                require(time.monotonic() - started < 180, 'outer timeout')
                require(shutil.disk_usage(ROOT).free >= 4 * 1024**3, 'disk floor')
                try:
                    process.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    pass
        require(process.returncode == 0, 'first child refused')
        require(all(sha((ROOT / name).read_bytes()) == digest for name, digest in source.items()), 'source changed')
        require(subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip() == head, 'head changed')
        write('result.json', review((out / 'stdout.log').read_bytes(), namespace=namespace))
        print(json.dumps(dict(samples=count, iterations=16, namespace_chain=namespace, measured_child_repeated=False)))
    except Exception as error:
        write('refusal.json', dict(type=type(error).__name__, reason=str(error),
            exit=None if process is None else process.poll()))
        if process is not None and process.poll() is None:
            with (out / 'cleanup.log').open('xb') as log:
                subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                    stdout=log, stderr=subprocess.STDOUT, timeout=30, check=False)
            process.wait(timeout=30)
        raise


if __name__ == '__main__':
    require(len(sys.argv) in (3, 4) and (len(sys.argv) == 3 or sys.argv[3] == '--namespace'), 'fixed arguments')
    capture(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve(), namespace=len(sys.argv) == 4)
