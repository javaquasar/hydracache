"""One non-admitting, private-namespace preparation profile; no manager start."""
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import types

UNIT = 'hydracache-performance-supervisor-074.service'
BOOT = 'e81671d6-bf23-4ebf-b8fe-5dad1f45866f'
SELECTOR = ('diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::'
            'tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::'
            'collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::'
            'starting_tests::privileged_root::cgroup::held_live::privileged_consuming_held_live_matrix')
MARKERS = ('campaigns/active-campaign', 'campaigns/active-diagnostic.json',
           'campaigns/.active-diagnostic.pending', 'campaigns/diagnostic-requests.json',
           'campaigns/.diagnostic-requests.pending', 'campaign-lifecycle-smoke-v1.json',
           'controller-loss-smoke-v1.json', 'diagnostic-intents-074', 'worker-authority-074')
FLOOR = 4 * 1024**3


def need(ok, reason):
    if not ok:
        raise ValueError(reason)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def put(path, value):
    with path.open('x', encoding='utf8', newline='\n') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def query(*argv):
    return subprocess.check_output(argv, timeout=10, text=True).strip()


def snapshot():
    need(os.geteuid() == os.getegid() == 0, 'root fixture only')
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    need(boot == BOOT, 'reviewed host boot changed')
    state = query('/usr/bin/systemctl', 'show', UNIT, '-p', 'ActiveState', '-p',
                  'SubState', '-p', 'MainPID', '-p', 'NRestarts', '-p', 'InvocationID')
    fields = dict(line.split('=', 1) for line in state.splitlines())
    need(fields['ActiveState'] == 'active' and fields['SubState'] == 'running'
         and fields['MainPID'] == '377415' and fields['NRestarts'] == '0',
         'stable supervisor differs: no stop, restart or lock acquisition')
    for name in MARKERS:
        need(not os.path.lexists('/var/lib/hydracache-performance/' + name),
             'active campaign/lifecycle context: ' + name)
    leftovers = sorted(str(p) for base in ('/tmp', '/sys/fs/cgroup')
                       for p in Path(base).iterdir()
                       if p.name.startswith(('hc074-owned-root-', 'hc074-root-fixture-')))
    need(not leftovers, 'preexisting or surviving fixture context: ' + repr(leftovers))
    paths = ('/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074',
             '/var/lib/hydracache-performance/provisioning-receipt-074.json')
    return dict(boot=boot, supervisor=fields, protected={p: sha(Path(p)) for p in paths},
                manager=query('/usr/bin/systemctl', '--version').splitlines()[0],
                kernel=os.uname().release, host_mounts=Path('/proc/self/mountinfo').read_text(),
                host_namespaces={name: os.readlink('/proc/1/ns/' + name) for name in ('mnt', 'user')},
                clock_services=query('/usr/bin/systemctl', 'show', 'systemd-timesyncd.service',
                                     'chrony.service', 'ntp.service', '-p', 'Id', '-p',
                                     'LoadState', '-p', 'ActiveState', '-p', 'MainPID'))


class CapacityError(RuntimeError):
    pass


def capacity():
    if shutil.disk_usage('/tmp').free < FLOOR:
        raise CapacityError('original physical 4 GiB floor')


def verify(root, manifest):
    for name, digest in manifest['files'].items():
        path = root / name
        need(path.is_file() and not path.is_symlink() and sha(path) == digest,
             'exact private root input: ' + name)
        stat = path.stat()
        need(stat.st_uid == stat.st_gid == 0 and stat.st_mode & 0o222 == 0,
             'root-owned immutable input: ' + name)


def argv(root):
    # This exact selector's healthy branch ends after join/drop/fixture cleanup.
    # There is no original_service selector, materialization mode or send/release.
    return ['/usr/bin/env', '-i', 'PATH=/usr/sbin:/usr/bin:/sbin:/bin',
            'HC074_OWNED_ROOT_FIXTURE=local-only-v1',
            'HC074_START_MATERIAL_BUNDLE=' + str(root / 'bundle'),
            'HC074_CONSUMING_HELD_LIVE_CASE=healthy', 'HC074_CONSUMING_HELD_COST=1',
            '/usr/bin/nice', '-n', '10', str(root / 'libtest'), SELECTOR,
            '--ignored', '--exact', '--nocapture', '--test-threads=1']


def run(root):
    need(root.parent == Path('/tmp') and root.name.startswith('hc074-preparation-root-20261028.'),
         'exact owned /tmp scope')
    need(not root.is_symlink() and root.stat().st_uid == root.stat().st_gid == 0
         and root.stat().st_mode & 0o7777 == 0o700, 'private root directory')
    put(root / 'started.json', dict(retry=False, server=True, product=False, admission=False,
                                  original_send=False, original_windows_seconds=[300, 60, 10]))
    result = dict(exit=125, retry=False, admission=False, product=False, server=True,
                  original_send=False, service_changes=False, clock_changes=False,
                  all_production_writers_retired_proven=False, exclusive_host=False)
    before = None
    try:
        manifest = json.loads((root / 'inputs.json').read_bytes())
        verify(root, manifest)
        before = snapshot()
        put(root / 'preflight.json', before)
        need(before == manifest['expected_host'], 'pre-dispatch host context changed')
        capacity()
        spec = importlib.util.spec_from_file_location('hc074_profile_owned_group', root / 'group.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        namespace = dict(time=time, math=math, CapacityError=CapacityError,
                         prepare=types.SimpleNamespace(require=need))
        # Exact existing, tested CI-envelope function, not a production watchdog.
        exec(compile((root / 'owned_function.py').read_bytes(), 'owned_function.py', 'exec'), namespace)
        execution = namespace['_owned'](module.LinuxGroup(), argv(root), str(root), capacity,
                                        bound=1200)
        put(root / 'execution.json', execution)
        result.update(exit=execution['exit'], retired=execution['retired'],
                      errors=execution['errors'], classification=execution['classification'])
        verify(root, manifest)
    except BaseException as error:
        result['refusal'] = dict(type=type(error).__name__, reason=str(error))
    finally:
        try:
            after = snapshot()
            put(root / 'postflight.json', after)
            result['postflight_passed'] = before is not None and before == after
            need(result['postflight_passed'], 'host/campaign/mount/supervisor postflight changed')
        except BaseException as error:
            result['postflight_passed'] = False
            result['postflight_refusal'] = dict(type=type(error).__name__, reason=str(error))
        if not result.get('postflight_passed'):
            result['exit'] = 125
        put(root / 'result.json', result)
    return result['exit']


if __name__ == '__main__':
    if sys.argv[1:] == ['--preflight']:
        print(json.dumps(snapshot(), sort_keys=True))
    else:
        need(len(sys.argv) == 1, 'fixed single attempt only')
        raise SystemExit(run(Path(__file__).parent))
