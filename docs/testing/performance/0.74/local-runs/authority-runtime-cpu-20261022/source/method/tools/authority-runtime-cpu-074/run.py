"""Fixed local actual-engine CPU cohort. No live or server route."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import importlib.util
import json
import os
from pathlib import Path
import re
import time

ROOT = Path(__file__).resolve().parents[2]
TOOLS = Path(__file__).resolve().parent
HELPER = ROOT / 'tools/authority-runtime-allocation-074/run.py'
HELPER_SHA = '1d238388466a4a8421698ac0bf6a9313a8bdead69e25dbf6e79549c4b35a840e'
spec = importlib.util.spec_from_file_location('allocation_method_074', HELPER)
common = importlib.util.module_from_spec(spec)
spec.loader.exec_module(common)
common.need(common.sha(HELPER) == HELPER_SHA, 'Frozen source/build helper changed')
PREFIX = 'diagnostic_worker_policy::authority_registry::fixture::tests::runtime_cpu::'
CONTROLS = [PREFIX + name for name in ('runtime_cpu_clocks', 'runtime_cpu_semantics')]
MEASURE = PREFIX + 'runtime_cpu_measurement'
HOOK = '\n#[path = "../tests/support/runtime_cpu_probe.rs"]\nmod runtime_cpu;\n'
PAIRS = 7


def generated(role):
    return {common.PACKAGE + 'tests/support/runtime_cpu_probe.rs': (TOOLS / 'probe.rs').read_bytes(),
        common.PACKAGE + 'tests/support/runtime_cpu_storage.rs': common.INSPECTORS[role].encode()}


def validate(folder, originals, role):
    expected = dict(originals); expected[common.TESTS] += HOOK.encode(); expected.update(generated(role))
    actual = {path.relative_to(folder).as_posix(): path for path in (folder / common.PACKAGE).rglob('*') if path.is_file()}
    common.need(set(actual) == set(expected), 'CPU overlay membership changed')
    rows = []
    for path in sorted(expected):
        common.need(not actual[path].is_symlink() and actual[path].read_bytes() == expected[path], 'CPU overlay edit: ' + path)
        rows.append(dict(path=path, bytes=len(expected[path]), sha256=common.sha_bytes(expected[path])))
    return rows


def stable():
    cpuinfo = '\n'.join(line for line in Path('/proc/cpuinfo').read_text().splitlines()
        if line.partition(':')[0].strip() in ('processor', 'vendor_id', 'cpu family', 'model', 'model name', 'stepping', 'flags'))
    return dict(common.snapshot(), affinity=sorted(os.sched_getaffinity(0)),
        taskset_sha256=common.sha('/usr/bin/taskset'),
        cpuinfo_identity_sha256=common.sha_bytes(cpuinfo.encode()))


def pressure():
    # Volatile descriptive context, not a reason to discard or rerun a pair.
    paths = ('/proc/cpuinfo', '/proc/loadavg', '/proc/pressure/cpu', '/proc/self/cgroup', '/sys/fs/cgroup/cpu.stat',
        '/sys/fs/cgroup/cpu.max', '/sys/fs/cgroup/cpuset.cpus.effective')
    return {path: Path(path).read_text() if Path(path).exists() else None for path in paths}


def argv(cpu, executable, test):
    return ['/usr/bin/taskset', '--cpu-list', str(cpu), executable, test, '--exact', '--nocapture', '--test-threads=1']


def build(lane, folders, output, own, env, cpu):
    compiler = {'194': '1.94.0', '188': '1.88.0'}[lane]
    common.invoke(['/home/artur/.cargo/bin/rustc', '+' + compiler, '--version'], ROOT, output, lane + '-compiler', env)
    images = {}
    for role in ('baseline', 'installed'):
        target = own / ('target-' + lane + '-' + role)
        common.seed_dependencies(lane, target, output, role)
        child = dict(env, CARGO_TARGET_DIR=str(target))
        name = lane + '-' + role
        args = ['--release', '--offline', '--locked', '-p', 'hydracache-long-run-supervisor-074']
        raw = common.invoke(['/home/artur/.cargo/bin/cargo', '+' + compiler, 'test', *args,
            '--lib', '--no-run', '--message-format=json'], folders[role], output, name + '-build', child)
        executable = common.image(raw)
        common.need(Path(executable).resolve().is_relative_to(target.resolve()), 'Wrong CPU image root')
        digest = common.sha(executable)
        common.invoke(['/home/artur/.cargo/bin/cargo', '+' + compiler, 'clippy', *args,
            '--lib', '--tests', '--', '-D', 'warnings'], folders[role], output, name + '-clippy', child)
        for index, test in enumerate(CONTROLS):
            raw = common.invoke(argv(cpu, executable, test), folders[role], output, name + '-control-' + str(index), child, 120)
            common.passed(raw, test)
        common.need(common.sha(executable) == digest, 'CPU control image drift')
        images[role] = dict(path=executable, sha256=digest, compiler=compiler)
    common.need(images['baseline']['path'] != images['installed']['path'] and
        images['baseline']['sha256'] != images['installed']['sha256'], 'CPU roles reused one image')
    return lane, images


def run(attempt):
    common.need(re.fullmatch('[a-z0-9-]{1,40}', attempt), 'Unsafe attempt')
    common.need(ROOT.as_posix() == '/mnt/c/Workspace/prj/jq/cashe/hydracache-074' and os.getuid() > 0 and os.getgid() > 0,
        'Exact local nonroot workspace required')
    common.need(not any(key.startswith(('HC074_', 'CARGO_PROFILE_')) or key in
        ('RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'LD_PRELOAD', 'CARGO_ENCODED_RUSTFLAGS') for key in os.environ), 'Unreviewed environment')
    output = ROOT / 'target' / ('authority-runtime-cpu-' + attempt)
    own = Path('/home/artur/.cache/hydracache-074-runtime-cpu-20261022') / attempt
    common.need(not output.exists() and not own.exists(), 'Never overwrite CPU evidence')
    output.mkdir(); own.mkdir(parents=True)
    result = dict(schema='authority-runtime-cpu-attempt-074-v1', attempt=attempt, failure=None,
        runtime_sources=dict(baseline=common.BASE, installed=common.CURRENT), owned_root=str(own),
        server=False, product=False, live=False, clocks_changed=False, services_changed=False, allocation_wrapper=False)
    try:
        before = stable(); result['build_before'] = before
        common.need(before['qualification'] == common.QUAL and before['affinity'], 'Qualification/affinity drift')
        cpu = before['affinity'][0]; result['cpu'] = cpu
        common.need(not common.git('diff', '--name-only', common.CURRENT, '--', common.PACKAGE).strip(), 'Installed runtime changed')
        common.need(sorted(common.git('diff', '--name-only', common.BASE, common.CURRENT, '--', common.PACKAGE).decode().splitlines()) == common.CHANGED, 'Runtime delta changed')
        for path in ('Cargo.toml', 'Cargo.lock'):
            common.need(common.source(common.BASE, path) == common.source(common.CURRENT, path), 'Profile/dependency drift')
        paths = ['Cargo.toml', 'Cargo.lock', '.cargo', 'crates', 'tools', 'fuzz', 'tests',
            'docs/testing/performance/0.74/diagnostic-worker-issuer-public.json',
            'docs/testing/performance/0.74/diagnostic-signer-issuer-public.json',
            'docs/testing/performance/0.74/diagnostic-signer-issuer-selection.json',
            'docs/testing/performance/0.74/diagnostic-builder-public-policy.json',
            'docs/testing/performance/0.74/rental-pilot-draft',
            'docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json']
        archive = common.git('archive', '--format=tar', common.CURRENT, '--', *paths)
        result['archive_sha256'] = common.sha_bytes(archive)
        with (own / 'source.tar').open('xb') as stream: stream.write(archive)
        folders = {role: own / role for role in ('baseline', 'installed')}
        originals = {}
        for role, folder in folders.items():
            common.extract(archive, folder)
            if role == 'baseline':
                for path in common.CHANGED: (folder / path).write_bytes(common.source(common.BASE, path))
            originals[role] = {path.relative_to(folder).as_posix(): path.read_bytes() for path in
                (folder / common.PACKAGE).rglob('*') if path.is_file()}
            (folder / common.TESTS).write_bytes(originals[role][common.TESTS] + HOOK.encode())
            for path, raw in generated(role).items(): (folder / path).write_bytes(raw)
            common.publish(output / (role + '-source.json'), validate(folder, originals[role], role))
            common.publish(output / (role + '-includes.json'), common.validate_includes(folder))
        inputs = output / 'compiled-inputs'; inputs.mkdir()
        tool_hashes = {}
        for name in ('probe.rs', 'run.py'):
            raw = (TOOLS / name).read_bytes(); tool_hashes[name] = common.sha_bytes(raw)
            with (inputs / name).open('xb') as stream: stream.write(raw)
        result['tool_hashes'] = tool_hashes; result['helper_sha256'] = common.sha(HELPER)
        env = dict(os.environ, PATH='/home/artur/.cargo/bin:' + os.environ.get('PATH', '/usr/bin:/bin'),
            CARGO_NET_OFFLINE='true', CARGO_BUILD_JOBS='2', RUSTFLAGS='-Dwarnings')
        result['build_environment'] = {key: env[key] for key in ('CARGO_NET_OFFLINE', 'CARGO_BUILD_JOBS', 'RUSTFLAGS')}
        with ThreadPoolExecutor(max_workers=2) as pool:
            images = dict(pool.map(lambda lane: build(lane, folders, output, own, env, cpu), ('194', '188')))
        result['images'] = images
        common.publish(output / 'ready.json', dict(images=images, tool_hashes=tool_hashes, cpu=cpu, build_before=before))
        deadline = time.monotonic() + 600
        while not (output / 'method.json').exists():
            common.need(time.monotonic() < deadline, 'CPU method freeze absent; no measurement executed')
            time.sleep(0.25)
        method = json.loads((output / 'method.json').read_text())['source']
        common.need(re.fullmatch('[0-9a-f]{40}', method), 'Exact method source required')
        for name, digest in tool_hashes.items():
            common.need(common.sha_bytes(common.source(method, 'tools/authority-runtime-cpu-074/' + name)) == digest == common.sha(TOOLS / name), 'CPU method/compiled input mismatch')
        common.need(common.sha_bytes(common.source(method, 'tools/authority-runtime-allocation-074/run.py')) == HELPER_SHA == common.sha(HELPER), 'CPU helper changed')
        for role, folder in folders.items(): validate(folder, originals[role], role)
        measured = stable(); result['measure_before'] = measured; result['pressure_before'] = pressure()
        common.need(measured['head'] == method and not measured['dirty'], 'CPU measurement must be committed and clean')
        common.need({key: value for key, value in before.items() if key not in ('head', 'dirty')} ==
            {key: value for key, value in measured.items() if key not in ('head', 'dirty')}, 'CPU build/measure context changed')
        for lane in ('194', '188'):
            for pair in range(PAIRS):
                for role in (('baseline', 'installed') if pair % 2 == 0 else ('installed', 'baseline')):
                    selected = images[lane][role]
                    common.need(common.sha(selected['path']) == selected['sha256'], 'CPU image replaced')
                    raw = common.invoke(argv(cpu, selected['path'], MEASURE), folders[role], output,
                        lane + '-pair-' + str(pair) + '-' + role, env, 300)
                    common.passed(raw, MEASURE)
                    common.need(common.sha(selected['path']) == selected['sha256'], 'CPU image drift')
        result['pressure_after'] = pressure(); result['measure_after'] = stable()
        common.need(result['measure_after'] == measured, 'CPU measurement postflight changed')
        for role, folder in folders.items(): validate(folder, originals[role], role)
    except Exception as error:
        result['failure'] = type(error).__name__ + ': ' + str(error)
    common.publish(output / 'result.json', result)
    common.need(result['failure'] is None, result['failure'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--attempt', required=True)
    run(parser.parse_args().attempt)
