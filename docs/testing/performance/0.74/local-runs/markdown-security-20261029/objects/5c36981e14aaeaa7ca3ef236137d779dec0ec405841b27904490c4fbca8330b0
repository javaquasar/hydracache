"""One offline native lock resolution; no build, tests, bus or host admission."""
from pathlib import Path
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import tarfile
import time

ROOT = Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
BASE = ROOT / 'target/dbus-native-20261028'
NATIVE = Path('/home/artur/.cache/hydracache-074-dbus-native/20261028-a1')
OUT = BASE / 'session'


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def configured():
    path = ROOT / 'tools/dbus-linux-074/native.py'
    if hashlib.sha256(path.read_bytes()).hexdigest() != '09ab4fb404b93af4ec5581b189d3e7fe54562fff14626bfabcbc07c11c9f51b1':
        raise ValueError('same original native owner primitives')
    owner = load(path, 'native_transport_lock_owner')
    owner.BASE, owner.NATIVE, owner.OUT = BASE, NATIVE, OUT
    owner.prepared = lambda: BASE / 'freeze/result.json'
    original = owner.command

    def command(label, review):
        owner.need(label == 'fd-194-check', 'one fixed offline resolution phase')
        _, env, artifact, target, folder, _ = original(label, review)
        argv = ['/home/artur/.rustup/toolchains/1.94.0-x86_64-unknown-linux-gnu/bin/cargo',
                'generate-lockfile', '--offline', '--manifest-path', str(folder / 'Cargo.toml')]
        return argv, env, artifact, target, folder, 'resolve'

    owner.command = command
    return owner


def session():
    owner = configured()
    os.umask(0o077)
    review = json.loads(owner.prepared().read_bytes())
    deadline = time.monotonic() + 1100
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    result = dict(passed=False, resolution_only=True, tests_executed=False,
        root_changed=False, server=False, services=False, clocks_changed=False,
        release_admitted=False, boot_before=boot, clocks_before=owner.clocks())
    changed = None
    try:
        owner.need(os.getuid() == os.getgid() == 1000, 'original nonroot local Linux')
        parent = NATIVE.parent
        if not parent.exists():
            parent.mkdir(mode=0o700)
        owner.need(parent.resolve() == parent and parent.stat().st_uid == os.getuid()
                   and not parent.stat().st_mode & 0o077, 'private native parent')
        NATIVE.mkdir(mode=0o700, exist_ok=False)
        owner.capacity()
        owner.need(owner.tree(BASE / 'input') == review['files'], 'complete exact source projection')
        for name in review['files']:
            source = BASE / 'input' / name
            owner.need(not source.is_symlink(), 'regular copied input')
            dest = NATIVE / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(source.read_bytes())
        vendor = {}
        for row in review['registry']:
            archive = NATIVE / 'archives' / (row['name'] + '.crate')
            owner.need(owner.sha(archive.read_bytes()) == row['sha256'], 'exact retained registry archive')
            directory = NATIVE / 'vendor' / row['name']
            directory.mkdir(parents=True)
            members = {}
            with tarfile.open(archive, 'r:gz') as tar:
                for entry in tar:
                    owner.need(entry.name.startswith(row['name'] + '/') and '..' not in Path(entry.name).parts,
                               'archive member scope')
                    if entry.isdir():
                        continue
                    owner.need(entry.isfile(), 'regular archive member only')
                    name = entry.name[len(row['name']) + 1:]
                    dest = directory / name
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    raw = tar.extractfile(entry).read()
                    dest.write_bytes(raw)
                    members[name] = owner.sha(raw)
            owner.write(directory, '.cargo-checksum.json', dict(package=row['sha256'], files=members))
            vendor[row['name']] = members
        owner.write(OUT, 'vendor-receipt.json', vendor)
        owner.sources(review)
        label = 'fd-194-check'
        out = OUT / label
        out.mkdir()
        argv, env, _, _, _, _ = owner.command(label, review)
        owner.write(out, 'intent.json', dict(argv=argv, environment=env,
            absolute_deadline=deadline, bound=1200, session_bound=1100,
            floor=4*1024**3, poll=5, retirement=5, retry=False, resolution_only=True))
        with (out / 'stdout.log').open('xb') as stdout, (out / 'stderr.log').open('xb') as stderr:
            child = subprocess.Popen(['/usr/bin/python3', '-B', str(Path(__file__)),
                'worker', label, str(deadline)], stdin=subprocess.DEVNULL,
                stdout=stdout, stderr=stderr, start_new_session=True)
            code = child.wait(timeout=max(0, deadline-time.monotonic()) + 10)
        execution = json.loads((out / 'execution.json').read_bytes())
        owner.need(execution['retired'] and not execution['errors'], 'original owned resolution retirement')
        owner.write(out, 'result.json', dict(exit=code, tests_executed=False, resolution_only=True))
        lock = (NATIVE / 'fd/Cargo.lock').read_bytes()
        changed = dict(review, files=dict(review['files'], **{'fd/Cargo.lock': owner.sha(lock)}))
        owner.sources(changed)
        with (OUT / 'Cargo.lock').open('xb') as stream:
            stream.write(lock)
        owner.need(code == 0, 'first actual offline resolver refusal')
        audit = load(ROOT / 'tools/dbus-native-transport-074/lock_review.py', 'native_transport_lock_audit')
        delta = audit.review((BASE / 'input/fd/Cargo.lock').read_bytes(), lock)
        owner.write(OUT, 'lock-review.json', delta)
        for name, expected in vendor.items():
            actual = owner.tree(NATIVE / 'vendor' / name)
            actual.pop('.cargo-checksum.json')
            owner.need(actual == expected, 'vendor source postflight')
        result['passed'] = True
    except BaseException as error:
        result['first_refusal'] = dict(type=type(error).__name__, reason=str(error))
    finally:
        try:
            owner.sources(changed or review)
            result['source_unchanged_except_resolved_lock'] = True
        except BaseException as error:
            result.update(passed=False, source_unchanged_except_resolved_lock=False, source_refusal=str(error))
        result.update(source_unchanged=False, boot_after=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                      clocks_after=owner.clocks())
        result['boot_unchanged'] = result['boot_after'] == boot
        owner.write(OUT, 'native-result.json', result)
    raise SystemExit(0 if result['passed'] and result['boot_unchanged'] else 1)


if __name__ == '__main__':
    if len(sys.argv) == 4 and sys.argv[1] == 'worker':
        configured().worker(sys.argv[2], float(sys.argv[3]))
        raise SystemExit('worker exits')
    if sys.argv[1:] != ['session']:
        raise ValueError('fixed offline resolution session')
    session()
