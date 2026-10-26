"""One local Git process-boundary control, never a package or timing cohort.

The batch process receives no object request. Keeping stdin open permits the
existing bounded held-lineage reader to observe the real launcher/native edge.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
from types import SimpleNamespace

from owned_process_observer import Refusal, validate
from process_lineage import HeldLineage, SnapshotApi, candidates, normalize

ROOT = Path(__file__).resolve().parents[2]
IMAGES = {
    r'C:\Program Files\Git\cmd\git.exe': '34a408843194be320d8a87a3c12cd5c7d2e08d03b24567a41db32e21d12569d2',
    r'C:\Program Files\Git\mingw64\bin\git.exe': '755d4896d35663d0ff08924f84507f35236b83d240635b512c519bf43cc71a87',
}
LAUNCHER, NATIVE = IMAGES
ARGV = [LAUNCHER, '--no-optional-locks', 'cat-file', '--batch']
RUN_SECONDS, DISCOVERY_SECONDS, SNAPSHOTS, CLEANUP_SECONDS = 10, 5, 100, 5


def require(value, reason):
    if not value:
        raise Refusal(reason)


def pins():
    require(os.name == 'nt', 'Windows-only diagnostic')
    require(normalize(shutil.which('git') or '') == normalize(LAUNCHER), 'independent launcher path drift')
    actual = {name: hashlib.sha256(Path(name).read_bytes()).hexdigest() for name in IMAGES}
    require(actual == IMAGES, 'independently pinned Git image bytes drift')
    return actual


def observe(child, api, origin, *, clock=time.monotonic, sleep=time.sleep):
    """Borrow the original Popen handle; own only the existing reader's leaf FD.

    Absence during bounded discovery is not a failed attachment. Any actual
    attachment/refusal is final. No alternate image or PID can replace a node.
    """
    require(child.poll() is None, 'root exited before attachment')
    handle = int(child._handle)
    pid, image = api.identity(handle)
    require(pid == child.pid and normalize(image) == normalize(LAUNCHER), 'original root identity drift')
    first = api.read(handle)
    validate(None, first)
    require(first['terminal'] is False, 'original root no longer live')
    root = SimpleNamespace(handle=handle, pid=pid, image=image, samples=[first], refusal=None)
    deadline = origin + RUN_SECONDS
    discovery_deadline = origin + DISCOVERY_SECONDS
    policy = [(NATIVE,)]
    lineage = None
    try:
        for index in range(SNAPSHOTS):
            require(clock() < discovery_deadline and child.poll() is None, 'bounded discovery expired/root exited')
            if candidates(api.snapshot(), pid, policy):
                require(clock() < discovery_deadline, 'late discovery witness')
                # One attachment attempt. Failure never resumes discovery.
                lineage = HeldLineage(root, api, policy, max_samples=2)
                require(clock() < discovery_deadline, 'late attachment witness')
                break
            sleep(0.01)
        require(lineage is not None, 'bounded discovery exhausted')
        child.stdin.close()  # EOF only; no object request and no program replacement.
        remaining = deadline - clock()
        require(remaining > 0, 'run window expired before EOF wait')
        code = child.wait(timeout=remaining)
        require(clock() < deadline, 'late root exit witness')
        require(code == 0 and lineage.sample()['terminal'] is True, 'root/native termination refused')
        last = api.read(handle)
        validate(first, last)
        require(last['terminal'] is True and last['creation_100ns'] == first['creation_100ns'],
                'original root terminal identity drift')
        return dict(identities=lineage.identities, root_samples=[first, last],
                    native_samples=lineage.samples, exit_code=code, discovery_snapshots=index + 1)
    finally:
        if lineage is not None:
            lineage.close()  # Query handles only; never closes the borrowed Popen handle.


def review(result):
    require(result['scope'] == 'git-launcher-native-boundary-only' and result['argv'] == ARGV,
            'fixed control scope/command drift')
    require(result['refusal'] is None and result['cleanup_errors'] == [] and result['exit_code'] == 0,
            'refused/unclean control')
    require(result['retry'] is False and type(result['object_requests']) is int and result['object_requests'] == 0
            and (result['run_seconds'], result['discovery_seconds'], result['cleanup_seconds']) == (10, 5, 5)
            and type(result['discovery_snapshots']) is int and 1 <= result['discovery_snapshots'] <= SNAPSHOTS,
            'control limits drift')
    require(result['image_pins_before'] == result['image_pins_after'] == IMAGES, 'image pin receipts drift')
    require(all(result[key] is False for key in ('package_pass', 'performance_cohort', 'product_admission',
                'mapped_image_attested', 'all_descendants_accounted', 'original_helper_executed')),
            'control cannot promote its scope')
    identities = result['identities']
    require(len(identities) == 2 and [normalize(row['image']) for row in identities] ==
            [normalize(LAUNCHER), normalize(NATIVE)], 'fixed two-image chain drift')
    require(all(type(row['pid']) is int and row['pid'] > 0 for row in identities) and
            identities[0]['pid'] != identities[1]['pid'] and
            all(type(row['creation_100ns']) is int for row in identities) and
            0 < identities[0]['creation_100ns'] <= identities[1]['creation_100ns'], 'node identity drift')
    for key, identity in zip(('root_samples', 'native_samples'), identities):
        samples = result[key]
        require(len(samples) == 2 and samples[0]['terminal'] is False and samples[1]['terminal'] is True,
                'missing live/terminal witness')
        validate(None, samples[0])
        validate(samples[0], samples[1])
        require(all(row['creation_100ns'] == identity['creation_100ns'] for row in samples), 'held creation drift')
    require(result['root_samples'][-1]['exit_100ns'] >= result['native_samples'][-1]['exit_100ns'],
            'launcher exit precedes native terminal witness')
    return dict(two_distinct_held_processes=True, directly_created_counter_excludes_native=True,
                numeric_speedup_established=False, old_cohort_topology_attested=False,
                current_full_helper_cost_attributed=False, package_pass=False)


def run(label):
    require(re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', label) is not None, 'bounded local label required')
    out = ROOT / 'target/git-process-boundary-20261026' / label
    out.mkdir(parents=True, exist_ok=False)
    result = dict(scope='git-launcher-native-boundary-only', argv=ARGV,
                  run_seconds=RUN_SECONDS, discovery_seconds=DISCOVERY_SECONDS,
                  cleanup_seconds=CLEANUP_SECONDS, retry=False, object_requests=0,
                  package_pass=False, performance_cohort=False, product_admission=False,
                  mapped_image_attested=False, all_descendants_accounted=False,
                  original_helper_executed=False, refusal=None, cleanup_errors=[], passed=False)
    def write(name, value):
        with (out / name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write('\n')
    # An exclusive intent precedes API, image, environment and child preflights.
    write('intent.json', dict(result, independent_images=IMAGES))
    child = None
    try:
        result['image_pins_before'] = pins()
        sys.path.insert(0, str(ROOT / 'tools/historical-blob-batch-074'))
        import package_effects_review_v6 as gate
        result['environment'] = gate.environment_preflight(ROOT)
        api = SnapshotApi()
        with (out / 'stdout.log').open('xb') as stdout, (out / 'stderr.log').open('xb') as stderr:
            origin = time.monotonic()
            child = subprocess.Popen(ARGV, cwd=ROOT, stdin=subprocess.PIPE, stdout=stdout, stderr=stderr)
            result.update(observe(child, api, origin))
        require(not (out / 'stdout.log').read_bytes() and not (out / 'stderr.log').read_bytes(), 'unexpected batch output')
        result['image_pins_after'] = pins()
        result['analysis'] = review(result)
        result['passed'] = True
    except BaseException as error:
        result['refusal'] = f'{type(error).__name__}: {error}'
    finally:
        if child is not None:
            cleanup_deadline = time.monotonic() + CLEANUP_SECONDS
            try:
                if child.stdin is not None and not child.stdin.closed:
                    child.stdin.close()
                child.wait(timeout=max(0, cleanup_deadline - time.monotonic()))
                require(time.monotonic() < cleanup_deadline, 'late cleanup witness')
            except Exception as error:
                result['cleanup_errors'].append(str(error))
                # Failure fallback uses only the still-owned root handle. This
                # neither certifies native retirement nor yields a passing result.
                if child.poll() is None:
                    try:
                        child.kill()
                        child.wait(timeout=max(0, cleanup_deadline - time.monotonic()))
                    except Exception as kill_error:
                        result['cleanup_errors'].append(str(kill_error))
            if child._handle is not None:
                try:
                    child._handle.Close()
                except Exception as error:
                    result['cleanup_errors'].append(str(error))
            if time.monotonic() >= cleanup_deadline:
                result['cleanup_errors'].append('late cleanup witness')
        if result['cleanup_errors']:
            result['passed'] = False
        write('result.json', result)
    return result


if __name__ == '__main__':
    require(len(sys.argv) == 3 and sys.argv[1] == 'control', 'usage: git_process_boundary.py control NEW_LABEL')
    result = run(sys.argv[2])
    print(json.dumps(dict(passed=result['passed'], refusal=result['refusal'], package_pass=False)))
    raise SystemExit(0 if result['passed'] else 1)
