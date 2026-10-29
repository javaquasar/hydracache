"""Local v2 runner. --plan is offline; --execute is a separate measurement action."""
import argparse
import json
import math
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time
import balanced as m

OPTIONAL = ('/proc/cpuinfo', '/proc/loadavg', '/proc/pressure/cpu', '/proc/stat', '/proc/self/cgroup',
            '/sys/fs/cgroup/cpu.stat', '/sys/fs/cgroup/cpu.max', '/sys/fs/cgroup/cpuset.cpus.effective')
NAMESPACES = ('mnt', 'pid', 'user', 'cgroup', 'net', 'ipc', 'uts', 'time')
OWN_ROOT = Path('/home/artur/.cache/hydracache-074-runtime-balanced-v2')
ARCHIVE_PATHS = ['Cargo.toml', 'Cargo.lock', '.cargo', 'crates', 'tools', 'fuzz', 'tests',
    'docs/testing/performance/0.74/diagnostic-worker-issuer-public.json',
    'docs/testing/performance/0.74/diagnostic-signer-issuer-public.json',
    'docs/testing/performance/0.74/diagnostic-signer-issuer-selection.json',
    'docs/testing/performance/0.74/diagnostic-builder-public-policy.json',
    'docs/testing/performance/0.74/rental-pilot-draft',
    'docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json']


def raw_clock():
    return time.clock_gettime_ns(time.CLOCK_MONOTONIC_RAW)


def optional_context(cpu, read=None, clock=None):
    read = read or (lambda path: Path(path).read_text())
    clock = clock or raw_clock
    before = clock(); values = {}
    paths = OPTIONAL + tuple(f'/sys/devices/system/cpu/cpu{cpu}/cpufreq/{name}'
                             for name in ('scaling_cur_freq', 'cpuinfo_cur_freq'))
    for path in paths:
        try: values[path] = dict(status='ok', text=read(path))
        except OSError as error: values[path] = dict(status='unavailable', reason=type(error).__name__)
    after = clock()
    m.need(type(before) is type(after) is int and 0 < before <= after, 'Context clock refused')
    return dict(start_raw_ns=before, end_raw_ns=after, values=values)


def validate_context(value, cpu):
    paths = set(OPTIONAL) | {f'/sys/devices/system/cpu/cpu{cpu}/cpufreq/{name}'
                           for name in ('scaling_cur_freq', 'cpuinfo_cur_freq')}
    m.need(set(value) == {'start_raw_ns', 'end_raw_ns', 'values'} and
           type(value['start_raw_ns']) is type(value['end_raw_ns']) is int and
           0 < value['start_raw_ns'] <= value['end_raw_ns'] and set(value['values']) == paths, 'Context membership/clock drift')
    for entry in value['values'].values():
        m.need((set(entry) == {'status', 'text'} and entry['status'] == 'ok' and isinstance(entry['text'], str)) or
               (set(entry) == {'status', 'reason'} and entry['status'] == 'unavailable' and isinstance(entry['reason'], str)),
               'Invalid optional context')


def validate_host(value):
    m.need(set(value) == {'head', 'dirty', 'boot', 'kernel', 'uid', 'gid', 'configs', 'qualification',
                         'affinity', 'taskset_sha256', 'cpuinfo_identity_sha256', 'namespaces'}, 'Incomplete stable host context')
    m.need(re.fullmatch('[0-9a-f]{40}', value['head']) and value['dirty'] == '' and
           value['qualification'] == m.common.QUAL and type(value['uid']) is type(value['gid']) is int and
           value['uid'] > 0 and value['gid'] > 0 and value['boot'] and
           value['kernel'].lower().endswith('microsoft-standard-wsl2'), 'Host identity refused')
    m.need(value['affinity'] and all(type(cpu) is int and cpu >= 0 for cpu in value['affinity']) and
           value['affinity'] == sorted(set(value['affinity'])) and set(value['namespaces']) == set(NAMESPACES) and
           all(isinstance(v, str) and v for v in value['namespaces'].values()), 'Placement/namespace drift')
    configs = {'/etc/wsl.conf', '/etc/systemd/timesyncd.conf', '/mnt/c/Users/ArturBuzov/.wslconfig'}
    m.need(set(value['configs']) == configs and all(v is None or re.fullmatch('[0-9a-f]{64}', v)
           for v in value['configs'].values()) and all(re.fullmatch('[0-9a-f]{64}', value[key])
           for key in ('taskset_sha256', 'cpuinfo_identity_sha256')), 'Configuration/tool identity drift')


def run_cohort(backend, output, images, expected, cpu, reference):
    """One serial bounded cohort. Test backends never launch an image."""
    m.validate_images(images); rows = m.validate_schedule(m.schedule())
    m.need(not output.exists(), 'Never resume or overwrite a partial cohort')
    output.mkdir()
    m.common.publish(output / 'plan.json', m.plan())
    result = dict(schema='authority-runtime-balanced-cohort-074-v2', mode=backend.mode,
                  schedule_sha256=m.SCHEDULE_SHA, completed=0, failure=None, attempted=0)
    m.need(backend.mode in ('native', 'synthetic'), 'Unknown backend')
    started = backend.monotonic(); deadline = started + m.COHORT_SECONDS
    result.update(started_monotonic=started, deadline_monotonic=deadline)
    def remaining():
        value = deadline - backend.monotonic()
        m.need(value > 0, 'Cohort budget exhausted')
        return min(m.PROCESS_SECONDS, value)
    try:
        for ordinal, row in enumerate(rows):
            record = dict(ordinal=ordinal, selection=row, stage='before', failure=None)
            result['attempted'] += 1
            try:
                remaining()
                m.need(backend.stable() == reference, 'Live source/boot/namespace context drift')
                image = images[row['compiler']][m.ROLE[row['role']]]
                backend.verify_image(image)
                record['before'] = backend.context(); validate_context(record['before'], cpu)
                record['stage'] = 'invoke'; record['launch_monotonic'] = backend.monotonic()
                timeout = min(m.PROCESS_SECONDS, deadline - record['launch_monotonic'])
                m.need(math.isfinite(timeout) and timeout > 0, 'Cohort budget exhausted before launch')
                record['timeout_seconds'] = timeout
                raw, process = backend.invoke(ordinal, row, image, timeout)
                record['process'] = process
                m.need(type(process['pid']) is int and process['pid'] > 0 and
                       all(type(process[key]) is int for key in ('start_raw_ns', 'spawned_raw_ns', 'exit_raw_ns')) and
                       record['before']['end_raw_ns'] <= process['start_raw_ns'] <= process['spawned_raw_ns'] <= process['exit_raw_ns'],
                       'Parent process boundary drift')
                record['stage'] = 'parse'
                header, _ = m.parse(raw, expected)
                record['clock_metadata'] = header
                record['stage'] = 'after'
                record['after'] = backend.context(); validate_context(record['after'], cpu)
                m.need(process['exit_raw_ns'] <= record['after']['start_raw_ns'], 'Post-context precedes process exit')
                backend.verify_image(image)
                m.need(backend.stable() == reference, 'Live postflight source/boot/namespace context drift')
                remaining()
                record['stage'] = 'complete'
                result['completed'] += 1
            except BaseException as error:
                record['failure'] = type(error).__name__ + ': ' + str(error)
                raise
            finally:
                m.common.publish(output / f'{ordinal:03d}-step.json', record)
        m.need(result['completed'] == 64, 'Incomplete cohort cannot be summarized')
    except BaseException as error:
        result['failure'] = type(error).__name__ + ': ' + str(error)
    result['finished_monotonic'] = backend.monotonic()
    if result['failure'] is None and not result['started_monotonic'] <= result['finished_monotonic'] < deadline:
        result['failure'] = 'ValueError: Cohort budget exhausted at completion'
    m.common.publish(output / 'result.json', result)
    return result


class LinuxBackend:
    mode = 'native'

    def __init__(self, output, folders, env, cpu):
        self.output = output; self.folders = folders; self.env = env; self.cpu = cpu

    @staticmethod
    def monotonic(): return time.monotonic()

    @staticmethod
    def stable():
        return dict(m.cpu.stable(), namespaces={name: os.readlink('/proc/self/ns/' + name) for name in NAMESPACES})

    @staticmethod
    def verify_image(image):
        path = Path(image['path'])
        m.need(path.is_file() and not path.is_symlink() and m.common.sha(path) == image['sha256'], 'Image replaced')

    def context(self): return optional_context(self.cpu)

    def invoke(self, ordinal, selection, image, timeout):
        name = f'{ordinal:03d}-profile'
        args = m.cpu.argv(self.cpu, image['path'], m.cpu.MEASURE)
        cwd = self.folders[m.ROLE[selection['role']]]
        process = None; expired = False; stdout = b''; stderr = b''
        stamps = dict(start_raw_ns=raw_clock())
        error = None
        try:
            process = subprocess.Popen(args, cwd=cwd, env=self.env, stdout=subprocess.PIPE,
                                       stderr=subprocess.PIPE, start_new_session=True)
            stamps.update(pid=process.pid, spawned_raw_ns=raw_clock())
            try: stdout, stderr = process.communicate(timeout=timeout)
            except subprocess.TimeoutExpired:
                expired = True
                try: os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                stdout, stderr = process.communicate()
        except BaseException as caught:
            error = type(caught).__name__ + ': ' + str(caught)
            if process is not None and process.poll() is None:
                try: os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                stdout, stderr = process.communicate()
        finally:
            stamps['exit_raw_ns'] = raw_clock()
            for suffix, raw in (('stdout', stdout), ('stderr', stderr)):
                with (self.output / (name + '.' + suffix + '.log')).open('xb') as stream: stream.write(raw)
            m.common.publish(self.output / (name + '.json'), dict(name=name, argv=args, cwd=str(cwd),
                exit=process.returncode if process else None, timeout=expired, error=error, **stamps,
                stdout_sha256=m.common.sha_bytes(stdout), stderr_sha256=m.common.sha_bytes(stderr)))
        m.need(error is None and not expired and process.returncode == 0, 'First measured child refused: ' + name)
        return stdout, stamps


def environment_guard():
    m.need(sys.platform == 'linux' and m.ROOT.as_posix() == '/mnt/c/Workspace/prj/jq/cashe/hydracache-074' and
           os.getuid() > 0 and os.getgid() > 0 and os.uname().release.lower().endswith('microsoft-standard-wsl2'),
           'Exact local nonroot WSL workspace required')
    m.need(not any(key.startswith(('HC074_', 'CARGO_PROFILE_')) or key in
        ('RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'LD_PRELOAD', 'CARGO_ENCODED_RUSTFLAGS') for key in os.environ),
        'Unreviewed build/live environment')


def prepare(own, output):
    m.need(not m.common.git('diff', '--name-only', m.common.CURRENT, '--', m.common.PACKAGE).strip(), 'Installed runtime changed')
    m.need(sorted(m.common.git('diff', '--name-only', m.common.BASE, m.common.CURRENT, '--', m.common.PACKAGE).decode().splitlines()) ==
           m.common.CHANGED, 'Eight-file runtime delta changed')
    for path in ('Cargo.toml', 'Cargo.lock'):
        m.need(m.common.source(m.common.BASE, path) == m.common.source(m.common.CURRENT, path), 'Profile/dependency drift')
    archive = m.common.git('archive', '--format=tar', m.common.CURRENT, '--', *ARCHIVE_PATHS)
    with (own / 'source.tar').open('xb') as stream: stream.write(archive)
    folders = {role: own / role for role in m.ROLE.values()}; originals = {}
    for role, folder in folders.items():
        m.common.extract(archive, folder)
        if role == 'baseline':
            for path in m.common.CHANGED: (folder / path).write_bytes(m.common.source(m.common.BASE, path))
        originals[role] = {path.relative_to(folder).as_posix(): path.read_bytes() for path in
                           (folder / m.common.PACKAGE).rglob('*') if path.is_file()}
        (folder / m.common.TESTS).write_bytes(originals[role][m.common.TESTS] + m.cpu.HOOK.encode())
        for path, raw in m.cpu.generated(role).items(): (folder / path).write_bytes(raw)
        m.common.publish(output / (role + '-source.json'), m.cpu.validate(folder, originals[role], role))
        m.common.publish(output / (role + '-includes.json'), m.common.validate_includes(folder))
    return folders, originals, m.common.sha_bytes(archive)


def build(lane, folders, output, own, env, cpu):
    # The original builder freshly compiles/lints/controls the unchanged probe.
    _, images = m.cpu.build(lane, folders, output, own, env, cpu)
    for role in m.ROLE.values():
        child = dict(env, CARGO_TARGET_DIR=str(own / ('target-' + lane + '-' + role)))
        m.common.invoke(['/home/artur/.cargo/bin/cargo', '+' + m.COMPILERS[lane], 'check', '--release', '--offline',
            '--locked', '-p', 'hydracache-long-run-supervisor-074', '--lib', '--tests'], folders[role], output,
            lane + '-' + role + '-check', child)
    return images


def wait_method(output, frozen_inputs, clock=None, sleep=None):
    m.need(set(frozen_inputs) == set(m.INPUTS) and all(re.fullmatch('[0-9a-f]{64}', digest)
           for digest in frozen_inputs.values()), 'Complete frozen method inputs required')
    clock = clock or time.monotonic; sleep = sleep or time.sleep
    deadline = clock() + m.FREEZE_SECONDS
    while not (output / 'method.json').exists():
        m.need(clock() < deadline, 'Method freeze absent; no measurement executed')
        sleep(0.25)
    m.need(clock() < deadline, 'Method freeze arrived after original deadline')
    value = json.loads((output / 'method.json').read_text())
    m.need(set(value) == {'source', 'schedule_sha256'} and re.fullmatch('[0-9a-f]{40}', value['source']) and
           value['schedule_sha256'] == m.SCHEDULE_SHA, 'Exact method/schedule freeze required')
    for name, digest in frozen_inputs.items():
        m.need(m.common.sha(m.ROOT / name) == digest and m.history.same_checkout_source(
            (m.ROOT / name).read_bytes(), m.common.source(value['source'], name)), 'Frozen method input changed: ' + name)
    return value['source']


def execute(attempt):
    m.need(re.fullmatch('[a-z0-9-]{1,40}', attempt), 'Unsafe attempt')
    environment_guard()
    expected = m.reference(); m.validate_schedule(m.schedule())
    output = m.ROOT / 'target' / ('authority-runtime-balanced-' + attempt)
    own = OWN_ROOT / attempt
    m.need(not output.exists() and not own.exists(), 'Never overwrite an existing attempt')
    output.mkdir(); own.mkdir(parents=True)
    result = dict(schema='authority-runtime-balanced-attempt-074-v2', failure=None, plan=m.plan(),
                  runtime_sources=dict(baseline=m.common.BASE, installed=m.common.CURRENT),
                  output=str(output), owned_root=str(own), server=False, product=False, live=False,
                  clocks_changed=False, services_changed=False, allocation_wrapper=False)
    try:
        before = LinuxBackend.stable(); result['build_before'] = before
        validate_host(before)
        m.need(before['qualification'] == m.common.QUAL and before['affinity'], 'Qualification/placement drift')
        cpu = before['affinity'][0]; result['cpu'] = cpu
        folders, originals, digest = prepare(own, output); result['archive_sha256'] = digest
        frozen_inputs = m.inputs(); result['tool_hashes'] = frozen_inputs
        compiled = output / 'compiled-inputs'; compiled.mkdir()
        for name in frozen_inputs:
            target = compiled / name; target.parent.mkdir(parents=True, exist_ok=True)
            with target.open('xb') as stream: stream.write((m.ROOT / name).read_bytes())
        env = dict(os.environ, PATH='/home/artur/.cargo/bin:' + os.environ.get('PATH', '/usr/bin:/bin'),
                   CARGO_NET_OFFLINE='true', CARGO_BUILD_JOBS='2', RUSTFLAGS='-Dwarnings')
        result['build_environment'] = {key: env[key] for key in ('CARGO_NET_OFFLINE', 'CARGO_BUILD_JOBS', 'RUSTFLAGS')}
        images = {lane: build(lane, folders, output, own, env, cpu) for lane in m.COMPILERS}
        m.validate_images(images); result['images'] = images
        m.common.publish(output / 'ready.json', dict(images=images, tool_hashes=frozen_inputs, plan=m.plan(), cpu=cpu))
        method = wait_method(output, frozen_inputs); result['method'] = method
        for role, folder in folders.items(): m.cpu.validate(folder, originals[role], role)
        reference = LinuxBackend.stable(); result['measure_before'] = reference
        m.need(reference['head'] == method and not reference['dirty'], 'Measurement method must be committed and clean')
        m.need({k: v for k, v in before.items() if k not in ('head', 'dirty')} ==
               {k: v for k, v in reference.items() if k not in ('head', 'dirty')}, 'Build/measure context drift')
        cohort = output / 'cohort'; backend = LinuxBackend(cohort, folders, env, cpu)
        completed = run_cohort(backend, cohort, images, expected, cpu, reference)
        m.need(completed['failure'] is None, completed['failure'])
        result['measure_after'] = LinuxBackend.stable()
        m.need(result['measure_after'] == reference and m.inputs() == frozen_inputs, 'Final metadata/method drift')
        for role, folder in folders.items(): m.cpu.validate(folder, originals[role], role)
    except BaseException as error:
        result['failure'] = type(error).__name__ + ': ' + str(error)
    m.common.publish(output / 'result.json', result)
    m.need(result['failure'] is None, result['failure'])


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--plan', action='store_true'); modes.add_argument('--execute', action='store_true')
    parser.add_argument('--attempt')
    args = parser.parse_args(argv)
    if args.plan:
        m.need(args.attempt is None, '--attempt only belongs to explicit execution')
        print(json.dumps(m.plan(), sort_keys=True))
    else:
        m.need(args.attempt is not None, 'Explicit execution needs a fresh attempt')
        execute(args.attempt)


if __name__ == '__main__': main()
