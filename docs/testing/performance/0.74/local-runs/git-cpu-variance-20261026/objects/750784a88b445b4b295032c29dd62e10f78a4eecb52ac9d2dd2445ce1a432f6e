"""Birth-to-terminal CPU accounting for ordinary short Git calls, not V6.

An unlimited private Job contains the diagnostic worker before it executes.
Its Git calls use the original program/argv/env and original std pipe helper.
Residual CPU is non-launcher descendants, not authenticated single-image CPU.
"""
import ctypes
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

from git_process_boundary import pins, ROOT
from owned_process_observer import Refusal, WindowsApi, validate

sys.path.insert(0, str(ROOT / 'scripts/ci'))
from watchdog_windows_job import Accounting, Api, WindowsJob

BOUND, CLEANUP, CALLS = 120, 5, 4
FREE_BYTES, LOG_BYTES = 4*1024**3, 16*1024**2
SPEC = '3e9bcc06ebce3439495200162cb0c1900b8eda53:crates/xtask/tests/diagnostic_artifact_fixture_074.rs'


def require(value, reason):
    if not value:
        raise Refusal(reason)


class AccountingApi(Api):
    def configure(self, handle):
        # Default new job: no CPU/memory/time limit, kill-on-close or breakaway.
        # Do NOT reuse CI's configure(), which changes process lifetime policy.
        require(self.empty(handle), 'new accounting Job is not empty')

    def counters(self, handle):
        value = Accounting()
        self.require(self.dll.QueryInformationJobObject(handle, 1, ctypes.byref(value),
                     ctypes.sizeof(value), None), 'QueryInformationJobObject(accounting)')
        return dict(user_ns=value.user_time*100, kernel_ns=value.kernel_time*100,
                    total=value.total, active=value.active, terminated=value.terminated)


class AccountingJob(WindowsJob):
    """Use CI's retained-child publication BEFORE membership check and resume."""
    def __init__(self, api, observer, image):
        self.observer, self.image = observer, image
        self.worker_identity = self.worker_start = None
        super().__init__(api)

    @property
    def child(self):
        return self._child

    @child.setter
    def child(self, value):
        require(getattr(self, '_child', None) is None, 'original worker cannot be replaced')
        self._child = value  # Retain even if a pre-resume observation refuses.
        if value is not None:
            pid, path = self.observer.identity(value.handle)
            require(pid == value.pid and Path(path).resolve() == self.image, 'original worker identity drift')
            self.worker_identity = (pid, path)
            first = self.observer.read(value.handle)
            validate(None, first)
            require(first['terminal'] is False, 'original worker exited before release')
            self.worker_start = first


def calculate(job, worker, launcher_rows, calls=CALLS):
    require(type(calls) is int and calls == CALLS, 'fixed call count')
    require(type(job['total']) is int and job['total'] == 1+2*calls and
            type(job['active']) is int and job['active'] == 0,
            'incomplete/unexpected Job membership')
    require(worker['terminal'] is True and worker['exit_100ns'] >= worker['creation_100ns'] > 0,
            'worker is not terminal')
    require(len(launcher_rows) == calls, 'missing direct launcher lifetime counters')
    require(len({(row['pid'], row['created_100ns']) for row in launcher_rows}) == calls,
            'duplicate launcher lifetime')
    result = {}
    for metric, field in (('user_ns', 'user_100ns'), ('kernel_ns', 'kernel_100ns')):
        require(type(job[metric]) is int and job[metric] >= 0 and type(worker[field]) is int
                and worker[field] >= 0, 'invalid CPU counter')
        direct = []
        for row in launcher_rows:
            require(type(row[metric]) is int and row[metric] >= 0 and row['exited_100ns'] >= row['created_100ns'] > 0,
                    'nonterminal/invalid launcher counter')
            direct.append(row[metric])
        value = job[metric] - worker[field]*100 - sum(direct)
        require(value >= 0, 'CPU accounting underflow; never clamp')
        result[metric] = value
    return dict(non_launcher_descendant_cpu=result, cpu_includes_native=True,
                exact_native_image_attested=False, per_call_native_cpu=False,
                initial_sample_subtracted=False, overhead_accepted=False, speedup_established=False,
                package_pass=False)


def terminal_job(first, last):
    require(first == last and type(first['active']) is int and first['active'] == 0,
            'Job charges are not terminal/stable')
    require(type(first['total']) is int and first['total'] == 1+2*CALLS,
            'incomplete/unexpected Job membership')


def cleanup(job, child, deadline):
    """Best-effort release of only owned objects; retain every cleanup failure."""
    errors = []
    child = child if child is not None else job.child
    try:
        if not job.is_empty():
            job.terminate()
        while not job.is_empty():
            require(time.monotonic() < deadline, 'cleanup deadline')
            time.sleep(0.01)
        if child is not None:
            child.wait(timeout=max(0, deadline-time.monotonic()))
    except Exception as error:
        errors.append(str(error))
    # A wait/terminate failure must not bypass release of either held object.
    for owned in (child, job):
        if owned is not None:
            try:
                owned.close()
            except Exception as error:
                errors.append(str(error))
    if time.monotonic() >= deadline:
        errors.append('late cleanup witness')
    return errors


def rows(raw, accounted):
    text = raw.decode()
    records = [json.loads(line.removeprefix('HC074_GIT_CPU ')) for line in text.splitlines()
               if line.startswith('HC074_GIT_CPU ')]
    done = [json.loads(line.removeprefix('HC074_GIT_CPU_DONE ')) for line in text.splitlines()
            if line.startswith('HC074_GIT_CPU_DONE ')]
    require(len(records) == CALLS and done == [dict(calls=CALLS, accounted=accounted,
        git_argv=['git', 'show', SPEC], stdin_hold=False, git_suspended=False, native_bypass=False,
        environment_changes=False, package_pass=False)], 'complete fixed worker protocol')
    require(re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out;', text),
            'one selected worker verdict missing')
    for call, row in enumerate(records):
        require(row['call'] == call and row['accounted'] is accounted and row['spec'] == SPEC and
                row['stdout_bytes'] == 23589 and row['stderr_bytes'] == 0 and row['code'] == 0,
                'Git semantics/schedule drift')
        if accounted:
            phase = row['phases']
            first, last = phase['child_start'], phase['child_end']
            require(first['pid'] == last['pid'] and first['created_100ns'] == last['created_100ns']
                    and last['exited_100ns'] >= last['created_100ns'] > 0 and
                    last['user_ns'] >= first['user_ns'] >= 0 and last['kernel_ns'] >= first['kernel_ns'] >= 0,
                    'original retained launcher identity/counter drift')
        else:
            require(row['phases'] is None, 'reference command must remain original output()')
    return records


def artifact(build):
    intent, result = (json.loads((build / name).read_bytes()) for name in ('intent.json', 'result.json'))
    require(result['exit'] == 0 and result['source_unchanged'] and result['head_unchanged']
            and result['outer_refusal'] is None and intent['source'] == result['source_after'], 'build receipt refused')
    require(intent['argv'] == ['cargo', '+1.94.0', 'test', '--offline', '--locked', '-p', 'xtask',
                              '--test', 'git_descendant_cpu_074', '--no-run', '--message-format=json'], 'fixed build command')
    require(all(hashlib.sha256((ROOT/name).read_bytes()).hexdigest() == digest
                for name, digest in intent['source'].items()), 'compiled source changed')
    require(subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() == intent['head'], 'build HEAD changed')
    messages = [json.loads(line) for line in (build/'stdout.log').read_text().splitlines() if line.startswith('{')]
    found = [row for row in messages if row.get('reason') == 'compiler-artifact'
             and row.get('target', {}).get('name') == 'git_descendant_cpu_074']
    require(len(found) == 1, 'ambiguous worker artifact')
    image = Path(found[0]['executable']).resolve()
    require(image.parent == (ROOT/'target/debug/deps').resolve() and found[0]['profile']['test'], 'worker image path/profile')
    return image, intent


def run(build_label, label):
    require(all(re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', value) for value in (build_label, label)), 'bounded labels')
    out = ROOT/'target/git-job-cpu-20261026'/label
    out.mkdir(parents=True, exist_ok=False)
    def write(name, value):
        with (out/name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, sort_keys=True, indent=2)
            stream.write('\n')
    result = dict(passed=False, first_refusal=None, cleanup_errors=[], modes=[], package_pass=False,
                  speedup_established=False, overhead_accepted=False, server=False)
    write('intent.json', dict(build_label=build_label, calls_per_mode=CALLS,
        modes=['reference_cpu_child', 'accounted_cpu_child'], bound_seconds=BOUND, cleanup_seconds=CLEANUP,
        free_bytes=FREE_BYTES, log_bytes=LOG_BYTES,
        retry=False, git_suspended=False, stdin_hold=False, native_bypass=False, environment_changes=False,
        job_limits=False, job_kill_on_close=False, package_pass=False, server=False))
    job = child = None
    try:
        result['pins_before'] = pins()
        sys.path.insert(0, str(ROOT/'tools/historical-blob-batch-074'))
        import package_effects_review_v6 as gate
        result['environment'] = gate.environment_preflight(ROOT)
        require(shutil.disk_usage(ROOT).free >= FREE_BYTES, 'disk preflight floor')
        image, build = artifact(ROOT/'target/source-continuation-20261024'/build_label)
        image_sha = hashlib.sha256(image.read_bytes()).hexdigest()
        write('build.json', dict(image=str(image), image_sha256=image_sha, source=build['source'], head=build['head']))
        origin = time.monotonic()
        api, process_api = AccountingApi(), WindowsApi()
        for name in ('reference_cpu_child', 'accounted_cpu_child'):
            job = AccountingJob(api, process_api, image)
            require(api.counters(job.handle) == dict(user_ns=0, kernel_ns=0, total=0, active=0, terminated=0),
                    'new accounting job has prior charges')
            with (out/(name+'.log')).open('xb') as log:
                child = job.spawn([str(image), '--exact', name, '--nocapture', '--test-threads=1'], ROOT, log, origin+BOUND)
                code = child.wait(timeout=max(0, origin+BOUND-time.monotonic()))
            require(time.monotonic() < origin+BOUND and code == 0, 'worker exit/deadline refused')
            pid, path = job.worker_identity
            require(process_api.dll.GetProcessId(child.handle) == pid == child.pid, 'retained terminal worker PID drift')
            worker = process_api.read(child.handle)
            validate(job.worker_start, worker)
            first, last = api.counters(job.handle), api.counters(job.handle)
            terminal_job(first, last)
            log_path = out/(name+'.log')
            require(log_path.stat().st_size <= LOG_BYTES, 'worker log size refused')
            data = rows(log_path.read_bytes(), name == 'accounted_cpu_child')
            row = dict(mode=name, worker_pid=child.pid, worker_image=path, worker_initial=job.worker_start,
                       worker=worker, job=first,
                       job_terminal_samples=[first, last], records=data)
            if name == 'accounted_cpu_child':
                row['analysis'] = calculate(first, worker, [value['phases']['child_end'] for value in data])
            result['modes'].append(row)
            child.close()
            job.close()
            child = job = None
        result['pins_after'] = pins()
        require(hashlib.sha256(image.read_bytes()).hexdigest() == image_sha and
                all(hashlib.sha256((ROOT/name).read_bytes()).hexdigest() == digest
                    for name, digest in build['source'].items()), 'postflight executable/source drift')
        require(subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() == build['head'], 'postflight HEAD drift')
        require(shutil.disk_usage(ROOT).free >= FREE_BYTES, 'disk postflight floor')
        require(time.monotonic() < origin+BOUND, 'late final accounting witness')
        result['passed'] = True
    except BaseException as error:
        result['first_refusal'] = f'{type(error).__name__}: {error}'
    finally:
        deadline = time.monotonic()+CLEANUP
        if job is not None:
            result['cleanup_errors'] = cleanup(job, child, deadline)
            if result['cleanup_errors']:
                result['passed'] = False
        write('result.json', result)
    return result


if __name__ == '__main__':
    require(len(sys.argv) == 3, 'usage: git_job_cpu.py BUILD_LABEL NEW_LABEL')
    result = run(*sys.argv[1:])
    print(json.dumps(dict(passed=result['passed'], first_refusal=result['first_refusal'], package_pass=False)))
    raise SystemExit(0 if result['passed'] else 1)
