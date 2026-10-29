"""Bounded WSL build/seed/timeout validation. No measurement entry is called."""
import argparse
import json
import os
from pathlib import Path
import re
import select
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[2]
TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / 'tools/authority-runtime-balanced-074'))
import balanced as m  # noqa: E402
import runner as r  # noqa: E402

OWN_ROOT = Path('/home/artur/.cache/hydracache-074-runtime-native-validation')
SYNTHETIC_TIMEOUT = 5
EXIT_OBSERVATION = 10
VALIDATION_SECONDS = 1200
NATIVE_INPUTS = [f'tools/authority-runtime-native-074/{name}' for name in
                 ('validate.py', 'cleanup_child.py', 'test_validate.py', 'README.md')]


def seed_audit(lane):
    saved = m.history.PACKET / 'measured' / (lane + '-baseline-dependency-seed.json')
    expected = json.loads(saved.read_text())
    origin = Path('/home/artur/.cache/hydracache-074-runtime-allocation-20261022/fifth') / ('target-' + lane)
    m.need(str(origin) == expected['origin'] and origin.is_dir(), 'Exact historical dependency seed missing')
    rows = []
    for path in sorted(origin.rglob('*')):
        m.need(not path.is_symlink(), 'Dependency seed symlink refused')
        relative = path.relative_to(origin)
        if any('hydracache_long_run_supervisor_074' in part or 'hydracache-long-run-supervisor-074' in part
               for part in relative.parts): continue
        if path.is_file(): rows.append(dict(path=relative.as_posix(), sha256=m.common.sha(path)))
    m.need(rows == expected['files'], 'Dependency seed differs from sealed historical manifest')
    return dict(origin=str(origin), files=rows, historical_receipt_sha256=m.common.sha(saved))


def cleanup(root):
    """Exercise the real adapter with a non-product two-process target."""
    root.mkdir(mode=0o700)
    image = root / 'synthetic-timeout'
    image.write_bytes((TOOLS / 'cleanup_child.py').read_bytes()); image.chmod(0o700)
    observed = {}; handles = []; errors = []
    def watch():
        try:
            deadline = time.monotonic() + SYNTHETIC_TIMEOUT - 0.5
            while not (root / 'ready.json').exists():
                m.need(time.monotonic() < deadline, 'Synthetic readiness deadline refused')
                time.sleep(0.01)
            # The file can be observed between exclusive create and completed JSON.
            while True:
                try: ready = json.loads((root / 'ready.json').read_text()); break
                except json.JSONDecodeError:
                    m.need(time.monotonic() < deadline, 'Synthetic readiness publication refused')
                    time.sleep(0.01)
            m.need(ready['uid'] == os.getuid() and ready['measurement'] is False and ready['product'] is False,
                   'Synthetic owner/scope drift')
            leader = ready['leader']['pid']
            for role in ('leader', 'child'):
                identity = ready[role]
                raw = Path(f"/proc/{identity['pid']}/stat").read_text()
                fields = raw[raw.rindex(')') + 2:].split()
                m.need(int(fields[19]) == identity['start_ticks'] and int(fields[2]) == leader and
                       int(fields[3]) == leader, 'Synthetic process/group identity drift')
                fd = os.pidfd_open(identity['pid']); handles.append(fd)
                m.need(not select.select([fd], [], [], 0)[0], 'Synthetic process exited before timeout')
            m.need(ready['child']['ppid'] == leader, 'Synthetic descendant identity drift')
            observed.update(ready)
        except BaseException as error: errors.append(type(error).__name__ + ': ' + str(error))
    watcher = threading.Thread(target=watch, daemon=True); watcher.start()
    adapter = r.LinuxBackend(root, dict(baseline=root), {}, min(os.sched_getaffinity(0)))
    refused = None
    try:
        try:
            adapter.invoke(0, m.schedule()[0], dict(path=str(image)), SYNTHETIC_TIMEOUT)
        except ValueError as error: refused = str(error)
        watcher.join(timeout=EXIT_OBSERVATION)
        m.need(not watcher.is_alive() and not errors and len(handles) == 2, 'Process identity observation refused: ' + str(errors))
        receipt = json.loads((root / '000-profile.json').read_text())
        m.need(refused and receipt['timeout'] is True and receipt['exit'] == -9 and receipt['error'] is None and
               receipt['pid'] == observed['leader']['pid'], 'Expected owned synthetic timeout did not occur')
        deadline = time.monotonic() + EXIT_OBSERVATION
        pending = set(handles)
        while pending:
            budget = deadline - time.monotonic()
            m.need(budget > 0, 'Synthetic process exit observation exhausted')
            done, _, _ = select.select(list(pending), [], [], budget)
            pending.difference_update(done)
        m.need((root / '000-profile.stdout.log').read_bytes() == b'HC074-SYNTHETIC-TIMEOUT-ONLY\n',
               'Unexpected synthetic output')
        value = dict(schema='authority-native-cleanup-control-v1', passed=True, expected_timeout_seconds=SYNTHETIC_TIMEOUT,
            pidfds_observed_exited=2, ready=observed, first_refusal=refused, measurement_executed=False, product=False)
        m.common.publish(root / 'control.json', value)
        return value
    finally:
        for fd in handles: os.close(fd)


def build_only(lane, folders, output, own, env, remaining):
    compiler = m.COMPILERS[lane]
    m.common.invoke(['/home/artur/.cargo/bin/rustc', '+' + compiler, '--version'], ROOT, output, lane + '-compiler', env,
                    min(1800, remaining()))
    images = {}
    for role in m.ROLE.values():
        target = own / ('target-' + lane + '-' + role)
        m.common.seed_dependencies(lane, target, output, role)
        child = dict(env, CARGO_TARGET_DIR=str(target))
        common = ['--release', '--offline', '--locked', '-p', 'hydracache-long-run-supervisor-074']
        cargo = ['/home/artur/.cargo/bin/cargo', '+' + compiler]; name = lane + '-' + role
        raw = m.common.invoke(cargo + ['test'] + common + ['--lib', '--no-run', '--message-format=json'],
            folders[role], output, name + '-build', child, min(1800, remaining()))
        executable = Path(m.common.image(raw))
        m.need(executable.resolve().is_relative_to(target.resolve()), 'Image escaped its fresh target')
        digest = m.common.sha(executable)
        m.common.invoke(cargo + ['clippy'] + common + ['--lib', '--tests', '--', '-D', 'warnings'],
            folders[role], output, name + '-clippy', child, min(1800, remaining()))
        m.common.invoke(cargo + ['check'] + common + ['--lib', '--tests'], folders[role], output,
            name + '-check', child, min(1800, remaining()))
        m.need(m.common.sha(executable) == digest, 'Build/check image drift')
        images[role] = dict(path=str(executable), sha256=digest, compiler=compiler)
    return images


def run(attempt):
    m.need(re.fullmatch('[a-z0-9-]{1,40}', attempt), 'Unsafe native-validation attempt')
    r.environment_guard(); m.reference()
    output = ROOT / 'target' / ('authority-runtime-native-' + attempt); own = OWN_ROOT / attempt
    m.need(not output.exists() and not own.exists(), 'Never overwrite native validation')
    output.mkdir(); own.mkdir(parents=True, mode=0o700)
    result = dict(schema='authority-runtime-native-validation-074-v1', attempt=attempt, failure=None,
        owned_root=str(own), source=None, measurement_executed=False, runtime_images_executed=False,
        server=False, product=False, clocks_changed=False, services_changed=False, phases=[])
    deadline = time.monotonic() + VALIDATION_SECONDS
    def remaining():
        value = deadline - time.monotonic(); m.need(value > 0, 'Native validation budget exhausted')
        return value
    try:
        before = r.LinuxBackend.stable(); r.validate_host(before)
        result['before'] = before; result['source'] = before['head']
        hashes = {name: m.common.sha(ROOT / name) for name in m.INPUTS + NATIVE_INPUTS}
        for name in hashes:
            m.need(m.history.same_checkout_source((ROOT / name).read_bytes(), m.common.source(before['head'], name)),
                   'Native validation source must be committed: ' + name)
        result['input_hashes'] = hashes
        result['cleanup'] = cleanup(own / 'cleanup'); result['phases'].append('cleanup')
        for lane in m.COMPILERS:
            remaining(); value = seed_audit(lane)
            m.common.publish(output / (lane + '-seed-before.json'), value)
        result['phases'].append('seed-audit')
        folders, originals, digest = r.prepare(own, output); result['archive_sha256'] = digest
        env = dict(os.environ, PATH='/home/artur/.cargo/bin:' + os.environ.get('PATH', '/usr/bin:/bin'),
                   CARGO_NET_OFFLINE='true', CARGO_BUILD_JOBS='2', RUSTFLAGS='-Dwarnings')
        result['build_environment'] = {key: env[key] for key in ('CARGO_NET_OFFLINE', 'CARGO_BUILD_JOBS', 'RUSTFLAGS')}
        images = {lane: build_only(lane, folders, output, own, env, remaining) for lane in m.COMPILERS}
        m.validate_images(images); result['images'] = images; result['phases'].append('build-only')
        for role, folder in folders.items(): m.cpu.validate(folder, originals[role], role)
        for lane in m.COMPILERS:
            current = seed_audit(lane)
            m.need(current == json.loads((output / (lane + '-seed-before.json')).read_text()), 'Original dependency seed changed')
            m.common.publish(output / (lane + '-seed-after.json'), current)
        after = r.LinuxBackend.stable(); result['after'] = after
        m.need(after == before and hashes == {name: m.common.sha(ROOT / name) for name in hashes}, 'Native postflight drift')
        remaining(); result['phases'].append('postflight')
    except BaseException as error:
        result['failure'] = type(error).__name__ + ': ' + str(error)
    m.common.publish(output / 'result.json', result)
    m.need(result['failure'] is None, result['failure'])
    print(json.dumps(dict(phases=result['phases'], measurement_executed=False, images_built=4, runtime_images_executed=False)))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--attempt', required=True)
    run(parser.parse_args().attempt)
