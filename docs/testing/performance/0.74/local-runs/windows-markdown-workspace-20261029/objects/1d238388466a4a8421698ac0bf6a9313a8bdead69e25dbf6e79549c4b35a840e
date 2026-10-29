"""Local source overlays, allocation controls and one frozen paired cohort. No live entry."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import tarfile
import time

ROOT = Path(__file__).resolve().parents[2]
TOOLS = Path(__file__).resolve().parent
BASE = '33c54b0cda8c330b115009cdf092ad002077cedf'
CURRENT = '66d5f371680d085a24de8f2558e1f049c71b9e7b'
PACKAGE = 'tools/long-run-supervisor-074/'
TESTS = PACKAGE + 'src/diagnostic_authority_fixture_tests.rs'
CHANGED = sorted([PACKAGE + 'src/' + name for name in ('diagnostic_authority_fixture.rs', 'diagnostic_authority_fixture_tests.rs')] +
    [PACKAGE + 'tests/support/' + name for name in ('authority_breakdown_controls.rs', 'authority_leaf_name_alloc.rs',
    'authority_leaf_name_controls.rs', 'authority_open_name_alloc.rs', 'authority_open_name_controls.rs', 'authority_owners_controls.rs')])
HOOK = '\n#[path = "../tests/support/runtime_allocation_probe.rs"]\nmod runtime_allocation;\n'
INSPECTORS = {
    'baseline': 'use super::*;\npub(super) fn scratch(_: &FixtureAuthorityRegistry) -> (usize, usize) { (0, 0) }\n',
    'installed': 'use super::*;\npub(super) fn scratch(registry: &FixtureAuthorityRegistry) -> (usize, usize) {\n    let mut slots = 0; let mut bytes = 0;\n    for entry in &registry.entries { for leaf in [&entry.record, &entry.marker] {\n        if let Some(raw) = &leaf.scratch { slots += 1; bytes += raw.capacity(); }\n    }}\n    (slots, bytes)\n}\n',
}
PREFIX = 'diagnostic_worker_policy::authority_registry::fixture::tests::runtime_allocation::'
CONTROLS = [PREFIX + 'meter::' + name for name in (
    'counter_realloc_releases_old_layout_and_reconciles_drop',
    'counter_cross_capture_lifetime_and_zeroed_forwarding',
    'counter_unwind_restores_inactive_state_and_does_not_count_other_threads')] + [PREFIX + 'runtime_allocation_semantic_control']
MEASURE = PREFIX + 'runtime_allocation_measurement'
QUAL = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'


def need(value, message):
    if not value:
        raise ValueError(message)


def sha_bytes(raw):
    return hashlib.sha256(raw).hexdigest()


def sha(path):
    return sha_bytes(Path(path).read_bytes())


def publish(path, value):
    with Path(path).open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(value, stream, sort_keys=True, indent=2)
        stream.write('\n')


def git(*args):
    result = subprocess.run(['/mnt/c/Program Files/Git/cmd/git.exe', '--no-replace-objects', '--no-optional-locks',
        '-C', 'C:/Workspace/prj/jq/cashe/hydracache-074', *args], capture_output=True, timeout=180)
    need(result.returncode == 0, 'Git source query refused: ' + result.stderr.decode(errors='replace')[:300])
    return result.stdout


def source(revision, path):
    return git('show', revision + ':' + path)


def generated(role):
    return {
        PACKAGE + 'tests/support/runtime_allocation_probe.rs': (TOOLS / 'probe.rs').read_bytes(),
        PACKAGE + 'tests/support/runtime_allocation_counter.rs': (TOOLS / 'counter.rs').read_bytes(),
        PACKAGE + 'tests/support/runtime_allocation_storage.rs': INSPECTORS[role].encode(),
    }


def snapshot():
    paths = [Path('/etc/wsl.conf'), Path('/etc/systemd/timesyncd.conf'), Path('/mnt/c/Users/ArturBuzov/.wslconfig')]
    return dict(head=git('rev-parse', 'HEAD').decode().strip(), dirty=git('status', '--porcelain', '--untracked-files=no').decode(),
        boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(), kernel=os.uname().release,
        uid=os.getuid(), gid=os.getgid(), configs={str(path): sha(path) if path.exists() else None for path in paths},
        qualification=sha(ROOT / 'docs/testing/performance/0.74/qualification-manifest.toml'))


def member(path):
    need(path and not path.startswith('/') and '\\' not in path and ':' not in path and '\x00' not in path and
        all(part not in ('', '.', '..') for part in path.split('/')), 'Unsafe source member')


def validate_package(folder, originals, role):
    expected = dict(originals)
    expected[TESTS] += HOOK.encode()
    expected.update(generated(role))
    actual = {path.relative_to(folder).as_posix(): path for path in (folder / PACKAGE).rglob('*') if path.is_file()}
    need(set(actual) == set(expected), 'Overlay package membership changed')
    rows = []
    for path in sorted(expected):
        need(not actual[path].is_symlink() and actual[path].read_bytes() == expected[path], 'Unexpected overlay edit: ' + path)
        rows.append(dict(path=path, bytes=len(expected[path]), sha256=sha_bytes(expected[path])))
    return rows


def extract(raw, folder):
    folder.mkdir()
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:') as archive:
        for item in archive.getmembers():
            member(item.name.rstrip('/'))
            need(item.isfile() or item.isdir(), 'Archive links or special files refused')
        archive.extractall(folder, filter='data')


def validate_includes(folder):
    result = {}
    for path in (folder / PACKAGE).rglob('*.rs'):
        for relative in re.findall(r'include_(?:bytes|str)!\(\s*"([^"]+)"', path.read_text()):
            target = (path.parent / relative).resolve()
            need(target.is_relative_to(folder.resolve()) and target.is_file(), 'Missing or escaping original include: ' + relative)
            result[target.relative_to(folder.resolve()).as_posix()] = sha(target)
    return result


def invoke(args, cwd, output, name, env, timeout=1800):
    process = subprocess.Popen(args, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    expired = False
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        expired = True
        os.killpg(process.pid, signal.SIGKILL)
        stdout, stderr = process.communicate()
    for suffix, raw in [('stdout', stdout), ('stderr', stderr)]:
        with (output / (name + '.' + suffix + '.log')).open('xb') as stream:
            stream.write(raw)
    row = dict(name=name, argv=args, cwd=str(cwd), exit=process.returncode, timeout=expired,
        stdout_sha256=sha_bytes(stdout), stderr_sha256=sha_bytes(stderr))
    publish(output / (name + '.json'), row)
    need(not expired and process.returncode == 0, 'Local step refused: ' + name)
    return stdout


def image(stdout):
    rows = [json.loads(line) for line in stdout.decode().splitlines()]
    paths = [row['executable'] for row in rows if row.get('reason') == 'compiler-artifact' and row.get('executable') and
        row['target']['name'] == 'hydracache_long_run_supervisor_074' and row['profile']['test'] and row['profile']['opt_level'] == '3' and row.get('fresh') is False]
    need(len(paths) == 1, 'One release libtest image required')
    return paths[0]


def passed(raw, name):
    text = raw.decode()
    need(re.search(r'(?m)^running 1 test$', text) and 'test ' + name + ' ... ' in text and
        (('test ' + name + ' ... ok') in text or re.search(r'(?m)^ok$', text)) and
        re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;', text), 'Wrong exact test outcome')


def build_lane(lane, folders, output, own, env):
    compiler = {'194': '1.94.0', '188': '1.88.0'}[lane]
    invoke(['/home/artur/.cargo/bin/rustc', '+' + compiler, '--version'], ROOT, output, lane + '-compiler', env)
    built = {}
    for role in ('baseline', 'installed'):
        target = own / ('target-' + lane + '-' + role)
        seed_dependencies(lane, target, output, role)
        child = dict(env, CARGO_TARGET_DIR=str(target))
        name = lane + '-' + role
        common = ['--release', '--offline', '--locked', '-p', 'hydracache-long-run-supervisor-074']
        raw = invoke(['/home/artur/.cargo/bin/cargo', '+' + compiler, 'test', *common, '--lib', '--no-run', '--message-format=json'], folders[role], output, name + '-build', child)
        executable = image(raw)
        need(Path(executable).resolve().is_relative_to(target.resolve()), 'Wrong test image root')
        digest = sha(executable)
        invoke(['/home/artur/.cargo/bin/cargo', '+' + compiler, 'clippy', *common, '--lib', '--tests', '--', '-D', 'warnings'], folders[role], output, name + '-clippy', child)
        for index, test in enumerate(CONTROLS):
            raw = invoke([executable, test, '--exact', '--nocapture', '--test-threads=1'], folders[role], output, name + '-control-' + str(index), child, 120)
            passed(raw, test)
        need(sha(executable) == digest, 'Image changed during controls')
        built[role] = dict(path=executable, sha256=digest, compiler=compiler)
    need(built['baseline']['path'] != built['installed']['path'] and
        built['baseline']['sha256'] != built['installed']['sha256'], 'Indistinguishable runtime images refused')
    return lane, built


def seed_dependencies(lane, target, output, role):
    # Copy bytes only: no hard links, source writes or package artifacts. Cargo
    # still checks dependencies and must freshly compile the selected libtest.
    origin = Path('/home/artur/.cache/hydracache-074-runtime-allocation-20261022/fifth') / ('target-' + lane)
    need(origin.is_dir() and not target.exists(), 'Exact immutable dependency seed required')
    files = []
    for path in sorted(origin.rglob('*')):
        need(not path.is_symlink(), 'Dependency seed symlink refused')
        relative = path.relative_to(origin)
        if any('hydracache_long_run_supervisor_074' in part or 'hydracache-long-run-supervisor-074' in part for part in relative.parts):
            continue
        if path.is_file():
            files.append((relative, sha(path)))
    target.mkdir()
    for relative, digest in files:
        destination = target / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        copy_dependency(origin / relative, destination, digest)
    publish(output / (lane + '-' + role + '-dependency-seed.json'), dict(origin=str(origin), target=str(target),
        files=[dict(path=relative.as_posix(), sha256=digest) for relative, digest in files], package_artifacts_copied=False))


def copy_dependency(original, destination, digest):
    shutil.copy2(original, destination)
    need(not os.path.samefile(original, destination) and original.stat().st_mode == destination.stat().st_mode,
        'Dependency copy must preserve permissions without links')
    need(sha(destination) == digest == sha(original), 'Copied dependency seed changed')


def run(attempt):
    need(re.fullmatch('[a-z0-9-]{1,40}', attempt), 'Unsafe attempt name')
    need(ROOT.as_posix() == '/mnt/c/Workspace/prj/jq/cashe/hydracache-074', 'Wrong local repository')
    need(os.getuid() > 0 and os.getgid() > 0, 'Nonroot local owner required')
    need(not any(key.startswith(('HC074_', 'CARGO_PROFILE_')) or key in ('RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'LD_PRELOAD', 'CARGO_ENCODED_RUSTFLAGS') for key in os.environ), 'Unreviewed build/live environment')
    output = ROOT / 'target' / ('authority-runtime-allocation-' + attempt)
    own = Path('/home/artur/.cache/hydracache-074-runtime-allocation-20261022') / attempt
    need(not output.exists() and not own.exists(), 'Never overwrite an allocation attempt')
    output.mkdir(); own.mkdir(parents=True)
    result = dict(schema='authority-runtime-allocation-attempt-074-v1', attempt=attempt, failure=None,
        runtime_sources=dict(baseline=BASE, installed=CURRENT), server=False, product=False,
        live=False, clocks_changed=False, services_changed=False, output=str(output), owned_root=str(own))
    try:
        before = snapshot(); result['build_before'] = before
        need(before['qualification'] == QUAL, 'Qualification drift')
        need(not git('diff', '--name-only', CURRENT, '--', PACKAGE).strip(), 'Installed package changed')
        changes = git('diff', '--name-only', BASE, CURRENT, '--', PACKAGE).decode().splitlines()
        need(sorted(changes) == CHANGED, 'Unexpected baseline/runtime source delta')
        for path in ('Cargo.toml', 'Cargo.lock'):
            need(source(BASE, path) == source(CURRENT, path), 'Dependency or profile drift')
        archive_paths = ['Cargo.toml', 'Cargo.lock', '.cargo', 'crates', 'tools', 'fuzz', 'tests',
            'docs/testing/performance/0.74/diagnostic-worker-issuer-public.json',
            'docs/testing/performance/0.74/diagnostic-signer-issuer-public.json',
            'docs/testing/performance/0.74/diagnostic-signer-issuer-selection.json',
            'docs/testing/performance/0.74/diagnostic-builder-public-policy.json',
            'docs/testing/performance/0.74/rental-pilot-draft',
            'docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json']
        raw = git('archive', '--format=tar', CURRENT, '--', *archive_paths)
        result['archive_sha256'] = sha_bytes(raw)
        with (own / 'source.tar').open('xb') as stream:
            stream.write(raw)
        folders = {role: own / role for role in ('baseline', 'installed')}
        originals = {}
        for role, folder in folders.items():
            extract(raw, folder)
            if role == 'baseline':
                for path in CHANGED:
                    (folder / path).write_bytes(source(BASE, path))
            originals[role] = {path.relative_to(folder).as_posix(): path.read_bytes() for path in (folder / PACKAGE).rglob('*') if path.is_file()}
            (folder / TESTS).write_bytes(originals[role][TESTS] + HOOK.encode())
            for path, data in generated(role).items():
                (folder / path).write_bytes(data)
            publish(output / (role + '-source.json'), validate_package(folder, originals[role], role))
            publish(output / (role + '-includes.json'), validate_includes(folder))
        tool_hashes = {name: sha(TOOLS / name) for name in ('probe.rs', 'counter.rs', 'run.py')}
        result['tool_hashes'] = tool_hashes
        env = dict(os.environ, PATH='/home/artur/.cargo/bin:' + os.environ.get('PATH', '/usr/bin:/bin'),
            CARGO_NET_OFFLINE='true', CARGO_BUILD_JOBS='2', RUSTFLAGS='-Dwarnings')
        result['build_environment'] = {key: env[key] for key in ('CARGO_NET_OFFLINE', 'CARGO_BUILD_JOBS', 'RUSTFLAGS')}
        with ThreadPoolExecutor(max_workers=2) as pool:
            images = dict(pool.map(lambda lane: build_lane(lane, folders, output, own, env), ('194', '188')))
        result['images'] = images
        publish(output / 'ready.json', dict(images=images, tool_hashes=tool_hashes, build_before=before))
        deadline = time.monotonic() + 600
        while not (output / 'method.json').exists():
            need(time.monotonic() < deadline, 'Method freeze signal absent; no measurement executed')
            time.sleep(0.25)
        method = json.loads((output / 'method.json').read_text())['source']
        need(re.fullmatch('[0-9a-f]{40}', method), 'Exact method commit required')
        for name, digest in tool_hashes.items():
            need(sha_bytes(source(method, 'tools/authority-runtime-allocation-074/' + name)) == digest == sha(TOOLS / name), 'Compiled harness differs from frozen method')
        for role, folder in folders.items():
            validate_package(folder, originals[role], role)
        measured_before = snapshot(); result['measure_before'] = measured_before
        need(measured_before['head'] == method and not measured_before['dirty'], 'Measurement method must be committed and clean')
        need({key: value for key, value in before.items() if key not in ('head', 'dirty')} ==
            {key: value for key, value in measured_before.items() if key not in ('head', 'dirty')}, 'Build/measure host context changed')
        for lane in ('194', '188'):
            for pair in range(5):
                for role in (('baseline', 'installed') if pair % 2 == 0 else ('installed', 'baseline')):
                    selected = images[lane][role]
                    need(sha(selected['path']) == selected['sha256'], 'Measured image replaced')
                    raw = invoke([selected['path'], MEASURE, '--exact', '--nocapture', '--test-threads=1'], folders[role], output,
                        lane + '-pair-' + str(pair) + '-' + role, env, 300)
                    passed(raw, MEASURE)
                    need(sha(selected['path']) == selected['sha256'], 'Measured image drift')
        result['measure_after'] = snapshot()
        need(result['measure_after'] == measured_before, 'Measurement postflight changed')
        for role, folder in folders.items():
            validate_package(folder, originals[role], role)
    except Exception as error:
        result['failure'] = type(error).__name__ + ': ' + str(error)
    publish(output / 'result.json', result)
    need(result['failure'] is None, result['failure'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--attempt', required=True)
    run(parser.parse_args().attempt)
