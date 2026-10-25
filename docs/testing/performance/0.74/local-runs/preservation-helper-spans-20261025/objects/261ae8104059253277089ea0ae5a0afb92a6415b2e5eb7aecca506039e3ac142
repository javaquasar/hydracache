"""Bounded local helper diagnostics, not package replay or product admission."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

import helper_review
import review
import sweep_review

ROOT = Path(__file__).resolve().parents[2]
MODES = ('reference', 'off', 'profile', 'fixture-healthy', 'fixture-panic')


def erase(source, contract, baseline):
    fields = {name: re.findall(r'pub const ' + name + r': &str = r#"(.*?)"#;', contract, re.S)
              for name in ('OLD_READ', 'NEW_READ', 'OLD_GIT', 'NEW_GIT')}
    review.require(all(len(values) == 1 for values in fields.values()), 'exact passive helper templates')
    hooks = re.findall(r'pub const HOOK: &str = ("(?:[^"\\]|\\.)*");', contract)
    review.require(len(hooks) == 1, 'one hook literal')
    hook = json.loads(hooks[0])
    for new, old in [(hook, ''), (fields['NEW_READ'][0], fields['OLD_READ'][0]),
                     (fields['NEW_GIT'][0], fields['OLD_GIT'][0])]:
        review.require(source.count(new) == 1, 'exact observer occurrence')
        source = source.replace(new, old, 1)
    review.require(source == baseline, 'observer cannot waive source drift')
    return source


def target_result(raw, expected, *, filtered=0):
    review.require(type(filtered) is int and filtered >= 0, 'explicit filtered scope')
    lines = raw.decode().splitlines()
    starts = [review.START.fullmatch(line) for line in lines if review.START.fullmatch(line)]
    ends = [review.END.fullmatch(line) for line in lines if review.END.fullmatch(line)]
    cases = [review.CASE.fullmatch(line) for line in lines if review.CASE.fullmatch(line)]
    review.require(len(starts) == len(ends) == 1 and int(starts[0][1]) == len(expected), 'complete target header')
    review.require(len(cases) == len(expected) and {row[1] for row in cases} == set(expected)
                   and all(row[2] == 'ok' for row in cases), 'exact successful case catalog')
    end = ends[0]
    review.require(end[1] == 'ok' and list(map(int, end.group(2, 3, 4, 5, 6))) == [len(expected), 0, 0, 0, filtered], 'target verdict')
    return dict(cases=[row[1] for row in cases], reported_ns=review.nanoseconds(end[7]), target_pass=True)


def original_cases():
    values = sweep_review.load_snapshot(ROOT)
    intent = review.decode_json(values[sweep_review.PHASE + 'intent.json'])
    result = review.decode_json(values[sweep_review.PHASE + 'result.json'])
    manifest = review.decode_json(values[sweep_review.PHASE + 'intent.json'])['source']
    import tomllib
    catalog = review.catalog(manifest, tomllib.loads(values['reviewed-source/crates/xtask/Cargo.toml'].decode()))
    report = review.parse_transcript(values[sweep_review.PHASE + 'stdout.log'].decode(),
        values[sweep_review.PHASE + 'stderr.log'].decode(), catalog, result['exit'], result['outer_refusal'], intent['bound_seconds'])
    target, = [row for row in report['targets'] if row['target'] == sweep_review.TARGET]
    review.require(target['reported_ns'] is not None and target['passed'] == 20, 'retained twenty-case scope')
    return [row['name'] for row in target['cases']]


def capture(build, out, mode):
    review.require(mode in MODES, 'fixed mode')
    out.mkdir(parents=True, exist_ok=False)
    def write(name, value):
        with (out / name).open('x', encoding='utf-8') as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write('\n')
    bound = 30 if mode.startswith('fixture-') else 780
    write('intent.json', dict(mode=mode, timeout_seconds=bound, disk_floor_bytes=4*1024**3,
        diagnostic_log_budget_bytes=16*1024**2, retry=False, package_replay=False, product=False,
        services=False, clocks=False, server=False, threads=1 if mode.startswith('fixture-') else 4))
    process = None
    try:
        review.require(os.name == 'nt' and shutil.disk_usage(ROOT).free >= 4*1024**3, 'platform/disk')
        review.require(not any(key.startswith(('HC074_', 'RUST_TEST_')) for key in os.environ), 'inherited diagnostic overrides')
        receipt = review.decode_json((build / 'result.json').read_bytes())
        source = review.decode_json((build / 'intent.json').read_bytes())['source']
        review.require(receipt['exit'] == 0 and receipt['source_unchanged'] and receipt['head_unchanged'], 'build receipt')
        review.require(all(review.sha((ROOT/name).read_bytes()) == digest for name, digest in source.items()), 'compiled source drift')
        baseline = ROOT / 'target/helper-spans-20261025/baseline'
        review.require(review.sha((baseline/'source.rs').read_bytes()) == sweep_review.SOURCE_SHA256, 'original source pin')
        target_source = (ROOT/sweep_review.SOURCE).read_text()
        erase(target_source, (ROOT/'tools/xtask-package-attribution-074/helper_contract.rs').read_text(),
              (baseline/'source.rs').read_text())
        old = review.decode_json((ROOT/'target/source-continuation-20261024/helper-spans-20261025-baseline-build/intent.json').read_bytes())['source']
        allowed = {sweep_review.SOURCE, 'crates/xtask/tests/authority_context_074.rs'}
        review.require(all(name in allowed or review.sha((ROOT/name).read_bytes()) == digest for name, digest in old.items()),
                       'unreviewed baseline/runtime input drift')
        artifacts = [review.decode_json(line) for line in (build/'stdout.log').read_text().splitlines() if line.startswith('{')]
        target = 'authority_context_074' if mode.startswith('fixture-') else 'diagnostic_artifact_fixture_074'
        artifact, = [row for row in artifacts if row.get('reason') == 'compiler-artifact' and row.get('target', {}).get('name') == target]
        binary = Path(artifact['executable']).resolve()
        review.require(binary.parent == (ROOT/'target/debug/deps').resolve(), 'compiled executable path')
        if mode == 'reference':
            binary = baseline/'diagnostic_artifact_fixture_074.exe'
            review.require(review.sha(binary.read_bytes()) == 'c0ffe82e2760044326ea2fca25447519b47138d68f27a2c6ca5344a804b0c0e9', 'unobserved executable pin')
        env = dict(os.environ)
        if mode in ('off', 'profile'):
            prior = 'reference' if mode == 'off' else 'off'
            prior_result = review.decode_json((out.parent/prior/'result.json').read_bytes())
            review.require(prior_result['mode'] == prior and prior_result['target_pass'] is True and
                           prior_result['repeated'] is False, 'finite protocol leg order')
        if mode in ('profile', 'fixture-healthy', 'fixture-panic'):
            env['HC074_PRESERVATION_PROFILE'] = 'v1'
        if mode.startswith('fixture-'):
            env['HC074_HELPER_FIXTURE'] = mode[len('fixture-'):]
            argv = [str(binary), '--exact', 'helper_observer_fixture_child', '--test-threads=1']
            expected = ['helper_observer_fixture_child']
        else:
            argv = [str(binary), '--test-threads=4']
            expected = original_cases()
        write('preflight.json', dict(argv=argv, binary_sha256=review.sha(binary.read_bytes()),
            compiled_source=source, cases=expected, observer_only_runtime_delta_verified=True,
            effect_review='original twenty-case read-only Git/source graph plus reviewed bounded stderr-only observer; not old full-package clearance'))
        with (out/'stdout.log').open('xb') as stdout, (out/'stderr.log').open('xb') as stderr:
            process = subprocess.Popen(argv, cwd=ROOT, env=env, stdout=stdout, stderr=stderr)
            start = time.monotonic_ns()
            while process.poll() is None:
                review.require(time.monotonic_ns()-start < bound*10**9, 'outer timeout')
                review.require(shutil.disk_usage(ROOT).free >= 4*1024**3, 'disk floor')
                review.require(sum((out/name).stat().st_size for name in ('stdout.log', 'stderr.log')) <= 16*1024**2, 'log budget')
                try:
                    process.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    pass
        elapsed = time.monotonic_ns()-start
        review.require(sum((out/name).stat().st_size for name in ('stdout.log', 'stderr.log')) <= 16*1024**2, 'final log budget')
        review.require(process.returncode == 0, 'first child refused')
        result = target_result((out/'stdout.log').read_bytes(), expected, filtered=17 if mode.startswith('fixture-') else 0)
        error = (out/'stderr.log').read_bytes()
        if mode in ('reference', 'off'):
            review.require(b'HC074_HELPER_TRACE ' not in error, 'disabled observer emitted')
        elif mode == 'fixture-panic':
            frames = [review.decode_json(line[len('HC074_HELPER_TRACE '):]) for line in error.decode().splitlines() if line.startswith('HC074_HELPER_TRACE ')]
            review.require(len(frames) == 1 and frames[0]['valid'] is True and len(frames[0]['records']) == 2,
                           'unwind model/producer join')
            review.require(all(row['unwinding'] is True and row['returned'] is False for row in frames[0]['records']), 'unwind labels')
            # The child itself downcasts/asserts the original payload. Successful
            # libtest capture need not print a caught panic to raw stderr.
            result['original_panic_assertion_passed'] = True
            try:
                helper_review.traces(error, expected)
            except ValueError:
                result['healthy_trace_refused'] = True
            else:
                raise ValueError('unwind accepted as healthy')
        else:
            allowed_cases = {f'helper-fixture-{index}' for index in range(4)} if mode == 'fixture-healthy' else set(expected)
            trace = helper_review.traces(error, allowed_cases)
            if mode == 'profile':
                for name, _, runtime_count, tail, _ in sweep_review.SWEEPS:
                    review.require(trace['groups'][name+'/read']['calls'] >= runtime_count+len(tail), 'preservation reads missing')
                    review.require(sum(row['operation']=='git' and row['arguments'][0]=='show' for frame in trace['frames']
                        if frame['case']==name for row in frame['records']) >= runtime_count+len(tail), 'preservation shows missing')
            else:
                review.require(len(trace['frames']) == 4 and trace['counts'] == dict(git=4, output=4, decode=4, read=4), 'four isolated fixture threads')
            result['trace'] = trace
        review.require(all(review.sha((ROOT/name).read_bytes()) == digest for name, digest in source.items()), 'source changed')
        result.update(mode=mode, process_wall_ns=elapsed, package_pass=False, repeated=False, product_candidate=False)
        write('result.json', result)
        print(json.dumps(dict(mode=mode, cases=len(expected), elapsed_seconds=elapsed/10**9,
            trace_records=sum(len(frame['records']) for frame in result.get('trace', {}).get('frames', [])))))
    except Exception as error:
        write('refusal.json', dict(type=type(error).__name__, reason=str(error), exit=None if process is None else process.poll()))
        if process is not None and process.poll() is None:
            with (out/'owned-cleanup.log').open('xb') as log:
                subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'], stdout=log, stderr=subprocess.STDOUT, timeout=30, check=False)
            process.wait(timeout=30)
        raise


if __name__ == '__main__':
    capture(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve(), sys.argv[3])
