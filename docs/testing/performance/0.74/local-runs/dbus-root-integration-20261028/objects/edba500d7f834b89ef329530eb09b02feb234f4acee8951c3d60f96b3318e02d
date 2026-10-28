"""One fixed offline Cargo metadata command under the original Windows Job owner."""
from pathlib import Path
import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import time
import traceback

import review as r

OUT = r.BASE / 'metadata'
FLOOR = 4 * 1024**3
CARGO = Path(r'C:/Users/ArturBuzov/.rustup/toolchains/1.94.0-x86_64-pc-windows-msvc/bin/cargo.exe')
OWNER = r.ROOT / 'tools/windows-workspace-preflight-074/owned.py'
OWNER_SHA = 'e7c5a3c0c4fea90f39772512c4ad96c60693fc0b482092a608dcf039e151eade'
PREP_SHA = 'c7f35ef9f497f548cf3de4c11fd6bff833621b2ba3a498621a57a7402af1224b'
_owner = None


def capacity():
    if shutil.disk_usage(r.ROOT).free < FLOOR:
        raise owner().CapacityError('original 4 GiB physical floor')


def owner():
    global _owner
    if _owner is not None:
        return _owner
    r.need(r.sha(OWNER.read_bytes()) == OWNER_SHA, 'original Windows CI mechanism')
    sys.path.insert(0, str(OWNER.parent))
    import prepare
    r.need(r.sha(Path(prepare.__file__).read_bytes()) == PREP_SHA, 'same original CI source')
    spec = importlib.util.spec_from_file_location('root_metadata_original_owner', OWNER)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    _owner = value
    return _owner


def guard(review, lock_may_change=False):
    actual = r.inventory(r.BASE / 'input')
    expected = dict(review['files'])
    if lock_may_change:
        r.need('workspace/Cargo.lock' in actual, 'candidate lock still exists')
        actual.pop('workspace/Cargo.lock')
        expected.pop('workspace/Cargo.lock')
    r.need(actual == expected, 'exact candidate source except intentional resolution lock')
    for names in (review['root_files'], review['protected'], review['helpers']):
        for name, digest in names.items():
            r.checked(name, digest)


def snapshot():
    return dict(head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=r.ROOT).decode().strip(),
        tracked=subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=no'], cwd=r.ROOT).decode())


def command():
    r.need(CARGO.is_file() and not CARGO.is_symlink(), 'fixed qualified Cargo image')
    toolchain = CARGO.parent
    rustc = toolchain / 'rustc.exe'
    r.need(rustc.is_file(), 'fixed matching rustc')
    env = {k: os.environ[k] for k in ('SystemRoot', 'WINDIR', 'COMSPEC', 'TEMP', 'TMP',
           'USERPROFILE', 'APPDATA', 'LOCALAPPDATA', 'USERNAME') if k in os.environ}
    env.update(PATH=str(toolchain) + os.pathsep + str(Path(os.environ['SystemRoot']) / 'System32'),
        CARGO_HOME=str(Path(os.environ['USERPROFILE']) / '.cargo'), CARGO_NET_OFFLINE='true',
        CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='2', RUSTC=str(rustc),
        RUSTC_WRAPPER='', RUSTC_WORKSPACE_WRAPPER='',
        CARGO_TARGET_DIR=str(r.BASE / 'metadata-target'))
    argv = [str(CARGO), 'metadata', '--offline', '--format-version', '1', '--manifest-path',
            str(r.BASE / 'input/workspace/Cargo.toml')]
    return argv, env


def config_snapshot(env):
    folders = [r.BASE / 'input/workspace', *(r.BASE / 'input/workspace').parents]
    paths = {Path(env['CARGO_HOME']) / name for name in ('config', 'config.toml')}
    paths |= {p / '.cargo' / name for p in folders for name in ('config', 'config.toml')}
    return {str(p): r.sha(p.read_bytes()) if p.is_file() else None for p in sorted(paths)}


def validate_metadata(metadata, review):
    r.need(metadata['version'] == 1 and metadata['resolve'] is not None, 'actual complete resolved metadata')
    workspace = r.BASE / 'input/workspace'
    packages = {p['id']: p for p in metadata['packages']}
    r.need(len(packages) == len(metadata['packages']), 'unique actual package identities')
    expected = {str((workspace / p / 'Cargo.toml').resolve()) for p in review['publication_scope']['members']}
    actual = {str(Path(packages[k]['manifest_path']).resolve()) for k in metadata['workspace_members']}
    r.need(actual == expected and len(metadata['workspace_members']) == len(expected),
           'all exact product workspace members; no vendor test targets added or product excluded')
    vendor = {str((workspace / r.VENDOR / n / 'Cargo.toml').resolve()) for n in ('zbus', 'zbus_macros', 'zbus_utils')}
    path_packages = {str(Path(p['manifest_path']).resolve()) for p in packages.values() if p['source'] is None}
    r.need(path_packages == expected | vendor, 'only reviewed local source packages')
    r.need(all(p['source'] is None or p['source'] == 'registry+https://github.com/rust-lang/crates.io-index'
               for p in packages.values()), 'unchanged registry source policy')
    r.need(not any(p['name'] == 'zbus' and p['version'].startswith('5.') for p in packages.values()),
           'old backend absent from full workspace resolution')
    nodes = metadata['resolve']['nodes']
    r.need({n['id'] for n in nodes} == set(packages)
           and all(d['pkg'] in packages for n in nodes for d in n['deps']), 'complete actual resolved graph')


def worker():
    review = json.loads((r.BASE / 'freeze/result.json').read_bytes())
    guard(review)
    capacity()
    argv, env = command()
    configs = config_snapshot(env)
    deadline = time.monotonic() + 1100
    r.write(OUT, 'cargo-intent.json', dict(argv=argv, environment=env, deadline=deadline,
        session_bound=1100, outer_bound=1200, floor=FLOOR, poll=5, retirement=5,
        locked=False, only_candidate_lock_may_change=True, offline=True, retry=False,
        configuration=configs,
        cargo_sha256=r.sha(CARGO.read_bytes()), rustc_sha256=r.sha(Path(env['RUSTC']).read_bytes())))
    child = None
    code = None
    try:
        with (OUT / 'cargo.stdout.json').open('xb') as stdout, (OUT / 'cargo.stderr.log').open('xb') as stderr:
            child = subprocess.Popen(argv, cwd=r.BASE / 'input/workspace', env=env,
                                     stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
            try:
                code = child.wait(timeout=max(0, deadline-time.monotonic()))
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=5)
                raise ValueError('original 1100-second metadata session expired')
        guard(review, lock_may_change=True)
        r.need(config_snapshot(env) == configs, 'Cargo configuration postflight')
        r.need(r.sha(CARGO.read_bytes()) == json.loads((OUT / 'cargo-intent.json').read_bytes())['cargo_sha256'],
               'same Cargo image postflight')
        r.write(OUT, 'cargo-result.json', dict(exit=code, source_unchanged_except_candidate_lock=True,
            root_unchanged=True, candidate_lock_sha256=r.sha((r.BASE / 'input/workspace/Cargo.lock').read_bytes()),
            metadata_only=True, compilation_executed=False, release_admitted=False))
    except BaseException as error:
        r.write(OUT, 'worker-refusal.json', dict(type=type(error).__name__, reason=str(error), exit=code))
        raise
    raise SystemExit(code)


def main():
    r.need(os.name == 'nt', 'original local Windows CI owner only')
    OUT.mkdir(exist_ok=False)
    (OUT / 'producer.py').write_bytes(Path(__file__).read_bytes())
    result = None
    with (OUT / 'stdout.log').open('xb') as stdout, (OUT / 'stderr.log').open('xb') as stderr:
        try:
            review = json.loads((r.BASE / 'freeze/result.json').read_bytes())
            before = snapshot()
            guard(review)
            argv, env = command()
            # Source custody is separate from the unchanged runtime census guard.
            r.write(OUT, 'intent.json', dict(source=before, argv=argv, metadata_only=True,
                bound=1200, session_bound=1100, floor=FLOOR, poll=5, retirement=5, retry=False))
            helper = subprocess.Popen([sys.executable, '-B', str(Path(__file__)), '--owned-worker'],
                cwd=r.ROOT, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
            try:
                helper.wait(timeout=1210)
            except subprocess.TimeoutExpired:
                helper.kill()
                helper.wait(timeout=5)
                raise ValueError('metadata owned helper disappeared')
            result = json.loads((OUT / 'execution.json').read_bytes())
            r.need(result['retired'] and not result['errors'], 'original owned Job retirement')
            guard(review, lock_may_change=True)
            r.need(snapshot() == before, 'root/head postflight')
            r.need(result['exit'] == 0, 'first metadata refusal retained')
            metadata = json.loads((OUT / 'cargo.stdout.json').read_bytes())
            validate_metadata(metadata, review)
            r.write(OUT, 'result.json', dict(passed=True, root_changed=False, metadata_only=True,
                root_adopted=False, release_admitted=False, packages=len(metadata['packages']),
                workspace_members=len(metadata['workspace_members']), source_unchanged_except_candidate_lock=True))
            print(json.dumps(dict(passed=True, packages=len(metadata['packages']), root_changed=False)))
        except BaseException as error:
            stderr.write(traceback.format_exc().encode())
            r.write(OUT, 'refusal.json', dict(type=type(error).__name__, reason=str(error), execution=result))
            raise


if __name__ == '__main__':
    if sys.argv[1:] == ['--worker']:
        worker()
    elif sys.argv[1:] == ['--owned-worker']:
        value = owner()
        result = value._owned(value.load_job().WindowsJob(),
            [sys.executable, '-B', str(Path(__file__)), '--worker'], str(r.ROOT), capacity, bound=1200)
        r.write(OUT, 'execution.json', result)
        raise SystemExit(result['exit'])
    else:
        r.need(sys.argv[1:] == ['metadata'], 'fixed offline metadata mode')
        main()
