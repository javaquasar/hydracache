"""One bounded local Linux component session; no original-owner or service work."""
import base64
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import tomllib


def clocks():
    return {name:time.clock_gettime_ns(getattr(time, name)) for name in
            ('CLOCK_REALTIME','CLOCK_MONOTONIC','CLOCK_MONOTONIC_RAW','CLOCK_BOOTTIME')}


def safe_relative(name):
    path = PurePosixPath(name)
    return not any(c in name for c in ('\\', ':', '\0')) and not path.is_absolute() and bool(path.parts) and all(p not in ('.','..') for p in path.parts)


def lock_subset(original, resolved):
    known = {(p['name'], p['version'], p.get('source'), p.get('checksum'))
             for p in tomllib.loads(original.decode())['package']}
    packages = tomllib.loads(resolved.decode())['package']
    for item in packages:
        if item['name'] == 'hc074-historical-blob-linux-fixture':
            if item['version'] != '0.0.0' or 'source' in item:
                raise ValueError('fixture package identity')
        elif (item['name'], item['version'], item.get('source'), item.get('checksum')) not in known:
            raise ValueError('resolved package outside existing lock: ' + item['name'])
    return len(packages)


def session(payload):
    os.umask(0o077)
    base = Path(tempfile.mkdtemp(prefix='hc074-blob-linux-20261024-')).resolve()
    if base.parent != Path('/tmp') or not base.name.startswith('hc074-blob-linux-20261024-'):
        raise ValueError('owned temporary root')
    before = clocks()
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    receipt = dict(schema='hc074-local-linux-blob-session-v1', root=str(base), uid=os.getuid(),
        uname=list(os.uname()), boot_before=boot, clocks_before=before, phases=[], sources={},
        server=False, clocks_or_services_changed=False, package_gate=False, admission=False)
    active_groups = set()
    original = base / 'repo/Cargo.lock'
    project = base / 'repo/crates/xtask'

    def run(name, argv, env, bound):
        out, err = base / (name + '.stdout'), base / (name + '.stderr')
        started = time.monotonic()
        reason = None
        with out.open('xb') as stdout, err.open('xb') as stderr:
            child = subprocess.Popen(argv, cwd=project, env=env, stdout=stdout, stderr=stderr,
                                     start_new_session=True)
            active_groups.add(child.pid)
            try:
                child.wait(timeout=bound)
            except subprocess.TimeoutExpired:
                reason = 'owned-group-timeout'
                os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=10)
            try:
                os.killpg(child.pid, 0)
            except ProcessLookupError:
                active_groups.remove(child.pid)
            value = dict(name=name, argv=argv, bound=bound, code=child.returncode, reason=reason,
                elapsed=time.monotonic()-started,
                stdout_b64=base64.b64encode(out.read_bytes()).decode(),
                stderr_b64=base64.b64encode(err.read_bytes()).decode())
            receipt['phases'].append(value)
            if child.returncode or reason:
                raise ValueError('first phase refusal: ' + name)

    try:
        for name, item in payload['files'].items():
            if not safe_relative(name):
                raise ValueError('source path')
            raw = base64.b64decode(item['base64'], validate=True)
            if hashlib.sha256(raw).hexdigest() != item['sha256']:
                raise ValueError('source digest')
            path = base / name
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('xb') as file:
                file.write(raw)
            receipt['sources'][name] = item['sha256']
        original_lock = original.read_bytes()
        for index, version in enumerate(('1.94.0','1.88.0')):
            toolchain = Path('/home/artur/.rustup/toolchains') / (version + '-x86_64-unknown-linux-gnu') / 'bin'
            if not (toolchain / 'cargo').is_file() or not (toolchain / 'rustc').is_file():
                raise ValueError('existing toolchain unavailable: ' + version)
            env = dict(os.environ, PATH=str(toolchain)+':/usr/local/bin:/usr/bin:/bin',
                CARGO_HOME='/home/artur/.cargo', CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0',
                CARGO_TARGET_DIR=str(base / 'build'),
                HC074_BLOB_TRANSPORT_EVIDENCE=str(base / 'evidence' / version))
            if shutil.disk_usage(base).free < 4 * 1024**3:
                raise ValueError('local Linux disk floor')
            cargo = str(toolchain / 'cargo')
            if index == 0:
                shutil.copyfile(original, project / 'Cargo.lock')
                # Unlike generate-lockfile, ordinary metadata resolution starts
                # from the supplied lock instead of deliberately regenerating it.
                run('lock-seed', [cargo,'metadata','--offline','--format-version','1'], env, 60)
                receipt['resolved_lock_b64'] = base64.b64encode((project / 'Cargo.lock').read_bytes()).decode()
                receipt['locked_packages'] = lock_subset(original_lock, (project / 'Cargo.lock').read_bytes())
            run(version+'-build', [cargo,'test','--offline','--locked','--no-run'], env, 300)
            for target in ('historical_blob_batch_074','historical_blob_fault_074','historical_blob_transport_074'):
                run(version+'-'+target, [cargo,'test','--offline','--locked','--test',target], env, 90)
        receipt['component_tests_passed'] = True
    except Exception as error:
        receipt['first_refusal'] = type(error).__name__ + ': ' + str(error)
    finally:
        for pid in list(active_groups):
            try:
                os.killpg(pid, signal.SIGKILL)
            except ProcessLookupError:
                active_groups.remove(pid)
        receipt['active_groups_after'] = sorted(active_groups)
        receipt['clocks_after'] = clocks()
        receipt['boot_after'] = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
        evidence = base / 'evidence'
        receipt['evidence'] = {}
        if evidence.exists():
            for path in sorted(evidence.rglob('*')):
                if path.is_file() and not path.is_symlink():
                    receipt['evidence'][str(path.relative_to(evidence))] = base64.b64encode(path.read_bytes()).decode()
        receipt['source_unchanged'] = all(hashlib.sha256((base / name).read_bytes()).hexdigest() == digest
                                         for name, digest in receipt['sources'].items())
        # Keep artifacts on uncertainty; never remove an unrelated or broad root.
        if not active_groups and base.parent == Path('/tmp') and base.name.startswith('hc074-blob-linux-20261024-'):
            shutil.rmtree(base)
            receipt['owned_root_removed'] = not base.exists()
        else:
            receipt['owned_root_removed'] = False
    return receipt


if __name__ == '__main__':
    value = session(json.load(sys.stdin))
    print(json.dumps(value, sort_keys=True))
    raise SystemExit(int('first_refusal' in value or not value['source_unchanged'] or not value['owned_root_removed']))
