"""Opt-in bounded coordinator telemetry. No native execution entry point."""
import time

from git_cpu_overhead import (AccountingJob, CLEANUP, LOG_BYTES, PROFILE, ROOT,
                              calculate, cleanup, require, rows, terminal_job, validate)

PHASES = ('setup', 'spawn', 'wait', 'terminal_reads', 'decode', 'close')


def contract():
    return dict(schema='hc074-coordinator-frames-v1', phases=list(PHASES), max_probes=12,
        max_clock_reads=36, wall_clock='time.monotonic_ns', cpu_clock='time.process_time_ns',
        units='ns', cpu_scope='coordinator process-wide, all threads',
        wall_bounds_scope='elapsed between bracketed CPU-read instants, not effect-only duration',
        clock_failure_policy='freeze telemetry, drain original owner, refuse diagnostic verdict',
        existing_predicates_removed_or_reordered=False, window_renewed=False,
        complete_frame_partition=False, cpu_is_native_git=False, overhead_accepted=False)


class Tape:
    def __init__(self, *, wall=None, cpu=None, clock_domain='python-coordinator'):
        require((wall is None and cpu is None and clock_domain == 'python-coordinator') or
                (callable(wall) and callable(cpu) and clock_domain == 'synthetic-control'), 'explicit paired clock domain')
        self.wall = time.monotonic_ns if wall is None else wall
        self.cpu = time.process_time_ns if cpu is None else cpu
        self.clock_domain = clock_domain
        self.probes, self.spans = [], []
        self.first_refusal = self.active = self.previous = None
        self.cursor, self.sealed = 0, False

    def fault(self, reason):
        if self.first_refusal is None:
            self.first_refusal = str(reason)[:512]

    def _probe(self, phase, kind):
        require(len(self.probes) < 12, 'probe budget exhausted')
        event = dict(phase=phase, kind=kind, raw={})
        self.probes.append(event)
        raw = event['raw']
        for key, clock in (('wall_before_ns', self.wall), ('process_cpu_ns', self.cpu), ('wall_after_ns', self.wall)):
            raw[key] = clock()
            require(type(raw[key]) is int and raw[key] >= 0, 'invalid '+key)
            if self.previous is not None and key == 'wall_before_ns':
                require(raw[key] >= self.previous['wall_after_ns'], 'wall sequence regressed')
            if self.previous is not None and key == 'process_cpu_ns':
                require(raw[key] >= self.previous['process_cpu_ns'], 'process CPU regressed')
            if key == 'wall_after_ns':
                require(raw[key] >= raw['wall_before_ns'], 'wall probe bracket regressed')
        self.previous = raw
        return len(self.probes)-1

    def begin(self, phase):
        if self.first_refusal is not None:
            return
        try:
            require(not self.sealed and self.active is None and self.cursor < len(PHASES) and
                    PHASES[self.cursor] == phase and len(self.probes) < 12, 'unexpected phase begin')
            span = dict(phase=phase, status='open', start_probe=len(self.probes), end_probe=None, elapsed=None)
            self.spans.append(span)
            self.active = span
            self._probe(phase, 'begin')
        except Exception as error:
            self.fault(f'{phase}/begin: {type(error).__name__}: {error}')

    def end(self, phase):
        if self.first_refusal is not None:
            return
        try:
            require(not self.sealed and self.active is not None and self.active['phase'] == phase and
                    len(self.probes) < 12, 'unexpected phase end')
            index = self._probe(phase, 'end')
            first, last = (self.probes[i]['raw'] for i in (self.active['start_probe'], index))
            self.active.update(end_probe=index, status='complete', elapsed=dict(
                wall_lower_ns=last['wall_before_ns']-first['wall_after_ns'],
                wall_upper_ns=last['wall_after_ns']-first['wall_before_ns'],
                process_cpu_ns=last['process_cpu_ns']-first['process_cpu_ns']))
            self.active = None
            self.cursor += 1
        except Exception as error:
            self.fault(f'{phase}/end: {type(error).__name__}: {error}')

    def finish(self, operation_passed):
        if self.sealed:
            self.fault('capture already sealed')
        if self.active is not None:
            self.active['status'] = 'operation_interrupted' if not operation_passed else 'incomplete'
        if any(set(p['raw']) != {'wall_before_ns', 'process_cpu_ns', 'wall_after_ns'} for p in self.probes):
            self.fault('interrupted clock probe')
        if operation_passed and self.cursor != len(PHASES):
            self.fault('incomplete phase schedule')
        self.sealed = True
        return dict(contract=contract(), clock_domain=self.clock_domain,
            complete=self.cursor == len(PHASES) and self.first_refusal is None,
            capture_valid=self.first_refusal is None, first_refusal=self.first_refusal,
            not_observed=list(PHASES[len(self.spans):]),
            probes=[dict(phase=p['phase'], kind=p['kind'], raw=dict(p['raw'])) for p in self.probes],
            spans=[dict(s, elapsed=dict(s['elapsed']) if s['elapsed'] is not None else None) for s in self.spans])


def attach(result, trace):
    result['operation_verdict'] = dict(passed=result['passed'], first_refusal=result['first_refusal'],
                                     cleanup_errors=list(result['cleanup_errors']))
    result['coordinator_frames'] = trace.finish(result['passed'])
    if not result['coordinator_frames']['capture_valid']:
        if result['first_refusal'] is None:
            result['first_refusal'] = 'Refusal: coordinator telemetry: '+str(trace.first_refusal)
        result['passed'] = False
    return result


def measure(api, observer, image, block, slot, mode, folder, deadline, trace):
    result = dict(block=block, slot=slot, mode=mode, passed=False, first_refusal=None, cleanup_errors=[])
    job = child = None
    wall, cpu = time.monotonic_ns(), time.process_time_ns()
    try:
        trace.begin('setup')
        require(time.monotonic() < deadline, 'worker frame started too late')
        job = AccountingJob(api, observer, image)
        require(api.counters(job.handle) == dict(user_ns=0, kernel_ns=0, total=0, active=0, terminated=0),
                'new accounting Job has prior charges')
        log_path = folder/f'{block}-{slot}-{mode}.log'
        with log_path.open('xb') as log:
            trace.end('setup')
            trace.begin('spawn')
            child = job.spawn([str(image), '--exact', mode, '--nocapture', '--test-threads=1'], ROOT, log, deadline)
            trace.end('spawn')
            trace.begin('wait')
            code = child.wait(timeout=max(0, deadline-time.monotonic()))
        require(time.monotonic() < deadline and code == 0, 'worker exit/deadline refused')
        trace.end('wait')
        trace.begin('terminal_reads')
        pid, path = job.worker_identity
        require(observer.dll.GetProcessId(child.handle) == pid == child.pid, 'retained terminal worker PID drift')
        worker = observer.read(child.handle)
        validate(job.worker_start, worker)
        first, last = api.counters(job.handle), api.counters(job.handle)
        terminal_job(first, last)
        require(first['terminated'] == 0, 'unexpected forced Job termination')
        trace.end('terminal_reads')
        trace.begin('decode')
        require(log_path.stat().st_size <= LOG_BYTES, 'worker log size refused')
        data = rows(log_path.read_bytes(), mode == PROFILE)
        result.update(worker_pid=child.pid, worker_image=path, worker_initial=job.worker_start,
                      worker=worker, job=first, job_terminal_samples=[first, last], records=data)
        if mode == PROFILE:
            result['native_inclusive'] = calculate(first, worker, [row['phases']['child_end'] for row in data])
        trace.end('decode')
        trace.begin('close')
        child.close()
        job.close()
        child = job = None
        trace.end('close')
        result['metrics'] = dict(wall_ns=time.monotonic_ns()-wall,
            coordinator_cpu_ns=time.process_time_ns()-cpu,
            whole_job_cpu_ns=first['user_ns']+first['kernel_ns'],
            whole_worker_cpu_ns=(worker['user_100ns']+worker['kernel_100ns'])*100)
        require(time.monotonic() < deadline, 'late worker frame witness')
        result['passed'] = True
    except BaseException as error:
        result['first_refusal'] = f'{type(error).__name__}: {error}'
    finally:
        if job is not None:
            result['cleanup_errors'] = cleanup(job, child, time.monotonic()+CLEANUP)
            if result['cleanup_errors']:
                result['passed'] = False
    return attach(result, trace)
