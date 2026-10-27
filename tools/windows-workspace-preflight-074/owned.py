"""Private Windows CI envelope prototype. No Cargo dispatcher or production authority."""
import importlib.util
import math
import os
from pathlib import Path
import time

import prepare

JOB_SHA = '8b392811e106eb7e692e21ee5b55cd04ea8ac421cad015edb66ce736b08c2ab1'


class CapacityError(RuntimeError):
    pass


def specification(mode):
    contract = prepare.contract()
    if mode == 'package':
        value = contract['package']
        return dict(argv=value['argv'], bound_seconds=780, start_free_bytes=prepare.FLOOR,
            floor_bytes=prepare.FLOOR, capacity_poll_seconds=5, cleanup_seconds=5,
            environment={'CARGO_INCREMENTAL': '0', 'CARGO_BUILD_JOBS': '2'}, retry=False)
    if mode == 'workspace':
        value = contract['workspace']
        return dict(argv=value['argv'], bound_seconds=4800, start_free_bytes=16*1024**3,
            floor_bytes=prepare.FLOOR, capacity_poll_seconds=5, cleanup_seconds=5,
            environment={'CARGO_INCREMENTAL': '0'}, retry=False)
    raise ValueError('fixed package/workspace modes only')


def load_job(root=prepare.ROOT):
    raw = prepare.checked(root, 'scripts/ci/watchdog_windows_job.py', JOB_SHA)
    prepare.require(raw and os.name == 'nt', 'original Windows Job only')
    path = root / 'scripts/ci/watchdog_windows_job.py'
    spec = importlib.util.spec_from_file_location('hc074_original_windows_job', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _owned(group, argv, cwd, capacity, *, bound, now=time.monotonic, pause=time.sleep):
    """Fixture-tested mechanism only; fixed gate dispatch still needs independent review.

    Its single-threaded worker must inherit already-separated stdout/stderr from
    the parent. No SetStdHandle/dup2, temporary global env, PID reopen or taskkill.
    A synthetic small test bound is not a package/workspace deadline override.
    """
    if not math.isfinite(bound) or bound <= 0:
        raise ValueError('finite positive private envelope bound')
    started = now()
    deadline = started + bound
    child, code, reason = None, None, 'runner-loss'
    errors = []
    retired = False
    try:
        capacity()
        child = group.spawn(argv, cwd, None, deadline)
        next_capacity = started + 5
        while True:
            at = now()
            if at >= deadline:
                reason = 'timeout'
                break
            if at >= next_capacity:
                capacity()
                next_capacity = now() + 5
            code = child.poll()
            if code is not None and (code != 0 or group.is_empty()):
                reason = ('success' if code == 0 else 'test-failure') if now() < deadline else 'timeout'
                break
            pause(min(0.2, max(0.01, deadline-now())))
    except KeyboardInterrupt:
        reason = 'cancelled'
    except BaseException as error:
        reason = 'resource-floor' if isinstance(error, CapacityError) else 'runner-loss'
        errors.append(type(error).__name__ + ': ' + str(error))
    finally:
        # The original helper retains a suspended child before later setup can fail.
        child = child if child is not None else group.child
        cleanup_deadline = now() + 5
        try:
            if not group.is_empty():
                group.terminate()
            while not group.is_empty():
                prepare.require(now() < cleanup_deadline, 'original CI Job retirement exceeded five seconds')
                pause(0.01)
            retired = True
            if child is not None:
                child.wait(timeout=max(0, cleanup_deadline-now()))
            prepare.require(now() < cleanup_deadline, 'original CI Job retirement exceeded five seconds')
        except BaseException as error:
            errors.append('retirement: ' + str(error))
        finally:
            for name, close in [('job', group.close), ('child', child.close if child is not None else None)]:
                if close is not None:
                    try:
                        close()
                    except BaseException as error:
                        errors.append('close ' + name + ': ' + str(error))
            if now() >= cleanup_deadline:
                errors.append('retirement: five-second bound exceeded during close')
    if not retired or any(e.startswith(('retirement:', 'close ')) for e in errors):
        reason = 'runner-loss'
    exit_code = {'timeout': 124, 'cancelled': 130}.get(reason,
        code if reason in ('success', 'test-failure') else 125)
    return dict(classification=reason, exit=exit_code, observed_child_exit=code,
        retired=retired, errors=errors, bound_seconds=bound, cleanup_seconds=5,
        capacity_poll_seconds=5, production_writer_authority=False)
