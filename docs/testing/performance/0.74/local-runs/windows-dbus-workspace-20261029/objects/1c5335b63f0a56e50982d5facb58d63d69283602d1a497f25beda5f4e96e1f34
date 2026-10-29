"""One fixed local capture profile; installed implementation remains unchanged."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import time

import caller_integration
import census_cost


def mirror_check(original, profile):
    original, profile = original.replace('\r\n', '\n'), profile.replace('\r\n', '\n')
    old = original[original.index('pub fn run_owned_with_io('):original.index('\npub fn git(')].strip()
    new = profile[profile.index('pub fn run_profiled('):].strip()
    new = '\n'.join(line for line in new.splitlines() if 'observe(CaptureEvent::' not in line
                    and 'observe: &mut impl FnMut(' not in line)
    new = new.replace('pub fn run_profiled(', 'pub fn run_owned_with_io(', 1)
    census_cost.require(old == new, 'profile must erase to exact installed function')
    old_helpers = original[original.index('struct Owned {'):original.index('\n/// No pipes')].strip()
    new_helpers = profile[profile.index('struct Owned {'):profile.index('pub fn run_profiled(')].strip()
    census_cost.require(old_helpers == new_helpers, 'private cleanup/bytes/timely helper drift')
    for line in ('const CLEANUP: Duration = Duration::from_secs(5);',
                 'const INPUT: usize = 256 * 1024;', 'const STDERR: usize = 64 * 1024;'):
        census_cost.require(line in original and line in profile, 'constant drift')


def review(raw):
    lines = raw.decode().splitlines()
    rows = [json.loads(v[len('HC074_TRANSPORT '):]) for v in lines if v.startswith('HC074_TRANSPORT ')]
    done, = [json.loads(v[len('HC074_TRANSPORT_DONE '):]) for v in lines if v.startswith('HC074_TRANSPORT_DONE ')]
    census_cost.require(len(rows) == 24 and done == dict(blocks=6, samples=24,
        legacy_is_unobserved_installed_transport=True, current_is_profile_mirror=True,
        product_candidate=False, checks_or_deadlines_changed=False), 'fixed capture scope')
    phases = ('source-tempfile', 'input-write-seek', 'output-tempfiles', 'setup-and-spawn',
              'metadata', 'try-wait', 'sleep', 'capture-read', 'stop')
    groups = {'unobserved': [], 'profiled': []}
    component, child_cpu = {name: [] for name in phases}, []
    for i, row in enumerate(rows):
        block, slot = divmod(i, 4)
        order = ('legacy', 'current', 'current', 'legacy') if block % 2 == 0 else ('current', 'legacy', 'legacy', 'current')
        census_cost.require((row['block'], row['slot'], row['variant']) == (block, slot, order[slot]), 'sample order')
        census_cost.require(row['stdout_bytes'] == 34850 and row['stderr_bytes'] == 0, 'Git result bytes')
        census_cost.require(all(type(row[k]) is int and row[k] >= 0 for k in ('wall_ns', 'parent_cpu_ns')), 'time shape')
        groups['unobserved' if row['variant'] == 'legacy' else 'profiled'].append(row)
        if row['variant'] == 'legacy':
            census_cost.require(row['spans'] == row['child'] == [], 'unobserved path instrumented')
            continue
        before, after = row['child']
        census_cost.require(before['pid'] == after['pid'] and before['created_100ns'] == after['created_100ns']
                            and after['exited_100ns'] > 0, 'held child identity or terminal state')
        child_cpu.append(after['kernel_ns'] + after['user_ns'])
        totals = dict.fromkeys(phases, 0)
        names = []
        for span in row['spans']:
            census_cost.require(span['phase'] in totals and type(span['wall_ns']) is int and span['wall_ns'] >= 0, 'phase')
            totals[span['phase']] += span['wall_ns']
            names.append(span['phase'])
        fixed_prefix = list(phases[:4])
        census_cost.require(names[:4] == fixed_prefix and names[-4:] == ['metadata', 'try-wait', 'capture-read', 'stop'], 'phase boundary')
        middle = names[4:-4]
        census_cost.require(len(middle) % 3 == 0 and middle == ['metadata', 'try-wait', 'sleep'] * (len(middle)//3), 'poll order')
        census_cost.require(sum(totals.values()) <= row['wall_ns'], 'overlapping spans')
        for name, value in totals.items():
            component[name].append(value)
    def describe(values):
        return dict(samples=len(values), min=min(values), median=statistics.median(values), max=max(values), mean=statistics.mean(values))
    return dict(samples=rows, summary={name: {metric: describe([row[metric] for row in group])
            for metric in ('wall_ns', 'parent_cpu_ns')} for name, group in groups.items()},
        phase_totals_ns={name: describe(values) for name, values in component.items()},
        whole_child_cpu_ns=describe(child_cpu), independent_host_replicates=False,
        observer_overhead_subtracted=False, product_performance_claim=False, package_replay=False)


def capture(root, build, out):
    out.mkdir(parents=True, exist_ok=False)
    def write(name, value):
        with (out / name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write('\n')
    write('intent.json', dict(build=str(build), blocks=6, samples=24, outer_seconds=180,
          disk_floor_bytes=4*1024**3, warmup=False, retry=False, source=census_cost.snapshot(root),
          server=False, services=False, clocks_changed=False))
    process = None
    try:
        census_cost.require(os.name == 'nt', 'Windows only')
        census_cost.require(shutil.disk_usage(root).free >= 4*1024**3, 'disk floor')
        census_cost.require(not census_cost.environment_overrides(os.environ), 'ambient overrides')
        census_cost.require(not any(k.startswith(('HC074_TRANSPORT_', 'HC074_PACKAGE_')) for k in os.environ), 'fixture overrides')
        folder = root / 'tools/historical-blob-batch-074'
        mirror_check((folder / 'transport.rs').read_text(), (folder / 'transport_profile.rs').read_text())
        receipt = json.loads((build / 'result.json').read_text())
        census_cost.require(receipt['exit'] == 0 and receipt['source_unchanged'] and receipt['head_unchanged'], 'build receipt')
        expected = json.loads((build / 'intent.json').read_text())['source']
        for name, digest in expected.items():
            if name.endswith(('.rs', '.toml', '.lock')):
                census_cost.require(hashlib.sha256((root / name).read_bytes()).hexdigest() == digest, 'compiled source drift')
        messages = [json.loads(v) for v in (build / 'stdout.log').read_text().splitlines() if v.startswith('{')]
        artifact, = [v for v in messages if v.get('reason') == 'compiler-artifact' and v.get('target', {}).get('name') == 'census_cost_074']
        binary = Path(artifact['executable']).resolve()
        census_cost.require(binary.parent == (root / 'target/debug/deps').resolve(), 'binary path')
        cases = caller_integration.pins(root)
        caller_integration.reference_source(cases)
        write('pins.json', dict(cases=cases, test_fixture_pin_integrity=True, production_pin_authority=False))
        write('preflight.json', dict(binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), mirror_exact=True))
        before = census_cost.snapshot(root)
        with (out / 'stdout.log').open('xb') as stdout, (out / 'stderr.log').open('xb') as stderr:
            process = subprocess.Popen([str(binary), '--exact', 'transport_capture_child', '--nocapture', '--test-threads=1'],
                cwd=root, stdout=stdout, stderr=stderr,
                env=dict(os.environ, HC074_TRANSPORT_CAPTURE_INPUT=str((out / 'pins.json').resolve())))
            start = time.monotonic()
            while process.poll() is None:
                census_cost.require(time.monotonic() - start < 180, 'outer timeout')
                census_cost.require(shutil.disk_usage(root).free >= 4*1024**3, 'disk floor')
                try:
                    process.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    pass
        census_cost.require(process.returncode == 0 and before == census_cost.snapshot(root), 'first capture result')
        write('result.json', review((out / 'stdout.log').read_bytes()))
        print(json.dumps(dict(samples=24, mirror_exact=True, source_unchanged=True, first_result_retained=True)))
    except BaseException as error:
        write('refusal.json', dict(type=type(error).__name__, reason=str(error)))
        if process is not None and process.poll() is None:
            try:
                with (out / 'cleanup.log').open('xb') as stream:
                    subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                        stdout=stream, stderr=subprocess.STDOUT, timeout=30, check=False)
                process.wait(timeout=30)
            except BaseException as cleanup_error:
                write('cleanup-refusal.json', dict(type=type(cleanup_error).__name__, reason=str(cleanup_error)))
        raise


if __name__ == '__main__':
    capture(Path(__file__).resolve().parents[2], Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
