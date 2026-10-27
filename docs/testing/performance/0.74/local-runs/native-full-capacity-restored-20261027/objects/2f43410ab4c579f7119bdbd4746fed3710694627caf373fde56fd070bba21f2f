"""One fixed full native package attempt; no qualification or product authority."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

CONTROL = Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074')
OUT = CONTROL / 'target/native-full-package-capacity-restored-20261027'
PINS = {'analysis.py': 'dd53d98e0b299e892a0ccd3597335409a3c3e476db08dd2317706918e1092474',
        'gnu_review.py': '816c467195b4d58ee1e25cd530258b15241023cf61bbea56ac284bf80b21c1c4'}
GROUP_SHA = 'e5ff52835fc2486171dbdbbb1b6df47177f583b4539f561779992c2c5065a691'
PARSER_SHA = '0229d4067fffd0d0b5acee7d864efc64aadfb201de1ea3f17944a2f8dddf3c4d'
BOUND = 780
FLOOR = 4 * 1024**3
ENV = dict(RUSTUP_AUTO_INSTALL='0', CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='2')
CONTROL_PHASE = 'native-full-coordinator-restored-final-controls'


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    import hashlib
    return hashlib.sha256(raw).hexdigest()


def load(path, name, expected):
    require(sha(path.read_bytes()) == expected, 'current helper drift')
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def check_controls(root):
    """A stale/red/missing local gate cannot reach Cargo, even in a bad shell chain."""
    folder = root / 'target/roots-lock-view-20261027' / CONTROL_PHASE
    intent = json.loads((folder / 'intent.json').read_bytes())
    result = json.loads((folder / 'result.json').read_bytes())
    require(result['exit'] == 0 and result['outer_refusal'] is None
        and result['source_unchanged'] and result['commit_unchanged'], 'native controls not green')
    expected = ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec', '/usr/bin/python3', '-m',
        'unittest', 'discover', '-s', '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/package-closure-offline-074',
        '-p', 'test_gnu_package.py', '-v']
    require(intent['argv'] == expected, 'native controls command drift')
    for name in ('gnu_package.py', 'test_gnu_package.py', 'analysis.py', 'gnu_review.py'):
        relative = 'tools/package-closure-offline-074/' + name
        digest = sha((root / relative).read_bytes())
        require(intent['source'][relative] == result['source_after'][relative] == digest, 'native controls source drift')


def spawn_separate(group, argv, cwd, out, deadline):
    """Single-threaded owner only: child inherits two exact exclusive files."""
    prior = {name: os.environ.get(name) for name in ENV}
    with (out / 'stdout.log').open('xb') as stdout, (out / 'stderr.log').open('xb') as stderr:
        sys.stdout.flush()
        sys.stderr.flush()
        saved = [os.dup(1), os.dup(2)]
        try:
            os.environ.update(ENV)
            os.dup2(stdout.fileno(), 1)
            os.dup2(stderr.fileno(), 2)
            return group.spawn(argv, cwd, None, deadline)
        finally:
            os.dup2(saved[0], 1)
            os.dup2(saved[1], 2)
            for descriptor in saved:
                os.close(descriptor)
            for name, value in prior.items():
                if value is None:
                    os.environ.pop(name, None)
                else:
                    os.environ[name] = value


def owned(group, argv, cwd, out, capacity, *, bound=BOUND, pause=time.sleep, now=time.monotonic):
    """Private CI primitive. The CLI supplies only fixed BOUND/argv/capacity.

    A small synthetic test bound is not a package limit override. Retain the
    original five-second CI retirement budget and kernel ECHILD requirement.
    """
    started = now()
    deadline = started + bound
    child, code, reason = None, None, 'runner-loss'
    cleanup_errors = []
    try:
        capacity()
        child = spawn_separate(group, argv, cwd, out, deadline)
        beat = started
        while True:
            capacity()
            code = child.poll()  # Nonreaping original leader observation.
            if code is not None and (code != 0 or group.is_empty()):
                reason = 'success' if code == 0 else 'test-failure'
                break
            at = now()
            if at >= deadline:
                reason = 'timeout'
                break
            if at >= beat:
                print(f'GNU package heartbeat elapsed={at-started:.1f}s remaining={max(0,deadline-at):.1f}s', flush=True)
                beat = at + 30
            pause(min(0.2, max(0.01, deadline - at)))
    except KeyboardInterrupt:
        reason = 'cancelled'
    except Exception as error:
        reason = 'resource-floor' if isinstance(error, CapacityError) else 'runner-loss'
        cleanup_errors.append(type(error).__name__ + ': ' + str(error))
    finally:
        cleanup_deadline = now() + 5
        try:
            if not group.is_empty():
                group.terminate()
            while not group.is_empty():
                require(now() < cleanup_deadline, 'owned CI retirement exceeded five seconds')
                pause(0.01)
            if child is not None:
                child.wait(timeout=max(0, cleanup_deadline - now()))
            require(now() < cleanup_deadline, 'owned CI retirement exceeded five seconds')
        except Exception as error:
            cleanup_errors.append('retirement: ' + str(error))
        finally:
            try:
                group.close()
                if child is not None:
                    child.close()
            except Exception as error:
                cleanup_errors.append('close: ' + str(error))
    retired = group.retired or child is None
    if not retired or any(s.startswith(('retirement:', 'close:')) for s in cleanup_errors):
        reason = 'runner-loss'
    exit_code = 124 if reason == 'timeout' else (code if reason in ('success', 'test-failure') else 125)
    return dict(classification=reason, exit=exit_code, observed_cargo_exit=code, retired=retired,
        cleanup_errors=cleanup_errors, bound_seconds=bound, cleanup_seconds=5)


class CapacityError(RuntimeError):
    pass


def execute():
    require(sys.platform.startswith('linux') and os.geteuid() != 0, 'nonroot local GNU only')
    require(OUT.parent.resolve(strict=True) == OUT.parent, 'capture ancestor alias')
    OUT.mkdir(exist_ok=False)
    def write(name, value):
        with (OUT / name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write('\n')
    write('intent.json', dict(launcher_sha256=sha(Path(__file__).read_bytes()), helpers=PINS,
        group_sha256=GROUP_SHA, parser_sha256=PARSER_SHA, bound_seconds=BOUND, test_threads=4,
        floor_bytes=FLOOR, child_environment=ENV, retry=False, services=False, clocks=False,
        server=False, product_load=False, qualification=False, release_admitted=False))
    try:
        check_controls(CONTROL)
        for name, digest in PINS.items():
            require(sha((Path(__file__).parent / name).read_bytes()) == digest, 'current review body drift')
        import gnu_review as g
        review = g.review()
        write('review.json', review)
        # Fresh whole-tree preflight precedes the timed compilation, as for V9.
        group_module = load(CONTROL / 'scripts/ci/watchdog_linux_group.py', 'checked_full_gnu_group', GROUP_SHA)
        parser = load(CONTROL / 'tools/xtask-package-attribution-074/review.py', 'checked_full_gnu_parser', PARSER_SHA)
        def capacity():
            if shutil.disk_usage('/mnt/c').free < FLOOR or shutil.disk_usage(g.ROOT).free < FLOOR:
                raise CapacityError('original physical/native 4 GiB floor')
        receipt = owned(group_module.LinuxGroup(), g.command(), g.ROOT, OUT, capacity)
        write('controller.json', receipt)
        # Group is closed and ECHILD established before any postflight subprocess.
        require(receipt['retired'], 'original CI child custody not retired')
        frontier = g.checked_module(CONTROL / 'tools/native-frontier-074/gnu_frontier.py',
            'checked_full_native_postflight', g.FRONTIER_SHA)
        sys.path.insert(0, str(CONTROL / 'tools/native-frontier-074'))
        import contract as c
        import native
        raw = (CONTROL / 'target/native-frontier-20261027' / frontier.SOURCE / 'request.json').read_bytes()
        require(c.sha(raw) == frontier.REQUEST_SHA, 'postflight request drift')
        request = c.request(c.decode(raw))
        native.validate_files(g.ROOT, request['tree'], capacity)
        require(native.git(g.ROOT, 'rev-parse', 'HEAD').decode().strip() == request['source'], 'postflight source drift')
        require(c.refs(native.git(g.ROOT, 'for-each-ref', '--format=%(objectname) %(refname)')) == request['refs'], 'postflight refs drift')
        configuration = subprocess.check_output(['/usr/bin/git', 'config', '--includes', '--show-origin', '--list'],
            cwd=g.ROOT, timeout=10)
        require(sha(configuration) == review['native_preflight']['actual_git_config_sha256'], 'postflight actual Git config drift')
        configs = [p / '.cargo' / name for p in (g.ROOT, *g.ROOT.parents)
                   for name in ('config', 'config.toml')]
        configs += [Path('/home/artur/.cargo') / name for name in ('config', 'config.toml')]
        require(not any(p.exists() for p in configs if p != g.ROOT / '.cargo/config.toml'), 'postflight shadow Cargo configuration')
        catalog = ['unittests src/lib.rs', 'unittests src/main.rs'] + sorted(
            'tests/' + p.name for p in (g.ROOT / 'crates/xtask/tests').glob('*.rs'))
        require(catalog == review['targets'], 'postflight target catalog drift')
        accounting = None
        if receipt['classification'] in ('success', 'test-failure', 'timeout'):
            accounting = parser.parse_transcript((OUT / 'stdout.log').read_text(encoding='utf-8'),
                (OUT / 'stderr.log').read_text(encoding='utf-8'), review['targets'], receipt['exit'],
                'outer-timeout' if receipt['classification'] == 'timeout' else None, BOUND)
            write('accounting.json', accounting)
        passed = bool(accounting and accounting['package_pass'] and receipt['classification'] == 'success')
        write('result.json', dict(package_pass=passed, accounting_assessed=accounting is not None,
            complete_native_source_and_refs_unchanged=True, native_source=request['source'],
            windows_v9_reinterpreted=False, release_admitted=False))
        print(json.dumps(dict(package_pass=passed, native_source=request['source'], release_admitted=False)))
        return int(not passed)
    except BaseException as error:
        write('refusal.json', dict(type=type(error).__name__, reason=str(error), package_pass=False, release_admitted=False))
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    if args.execute:
        raise SystemExit(execute())
    print(json.dumps(dict(bound_seconds=BOUND, test_threads=4, floor_bytes=FLOOR,
        execute=False, package_pass=False, release_admitted=False)))
