"""Separate integrated spawn diagnostic; old collectors remain unchanged."""
import time

from capture_leaves import (attach_leaves, leaf_rows, PROFILE, ROOT,
    CLEANUP, LOG_BYTES, calculate, cleanup, require, terminal_job, validate)
from coordinator_frames import attach
from spawn_frames import SpawnAccountingApi, SpawnAccountingJob, SpawnTape, contract as model_contract, PHASES


def contract():
    return dict(schema='hc074-spawn-capture-v1', model_schema='hc074-spawn-frames-model-v1',
        phases=list(PHASES), max_additional_probes=6, max_additional_clock_reads=18,
        max_recorder_clock_reads_with_outer_and_leaves=102,
        cpu_scope='coordinator process-wide, all threads; not child or native Git CPU',
        wall_bounds_scope='elapsed between bracketed CPU-read instants, not effect-only duration',
        successful_create_to_retention_has_no_added_probe=True, failed_create_last_error_read_unchanged=True,
        complete_frame_does_not_prove_resume=True, caller_connected=True, cleanup_instrumented=False,
        pure_create_cost_isolated=False, existing_predicates_removed_or_reordered=False,
        window_renewed=False, overhead_accepted=False, package_pass=False)


def attach_spawn(result, trace):
    # All original effects/owned cleanup have already completed. The original
    # verdict survives independently of outer/leaf/spawn diagnostic validity.
    raw = trace.finish(result['operation_verdict']['passed'])
    require(raw.pop('contract') == model_contract(), 'spawn model export contract differs')
    result['spawn_frames'] = dict(contract=contract(), **raw)
    if not raw['capture_valid']:
        if result['first_refusal'] is None:
            result['first_refusal'] = 'Refusal: spawn telemetry: '+str(raw['first_refusal'])
        result['passed'] = False
    return result


def measure(api, observer, image, block, slot, mode, folder, deadline, trace, leaves, spawn):
    result = dict(block=block, slot=slot, mode=mode, passed=False, first_refusal=None, cleanup_errors=[])
    job = child = None
    wall, cpu = time.monotonic_ns(), time.process_time_ns()
    try:
        trace.begin('setup')
        require(time.monotonic() < deadline, 'worker frame started too late')
        job = SpawnAccountingJob(api, observer, image, leaves, spawn)
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
        leaves.begin('decode', 'log_stat')
        require(log_path.stat().st_size <= LOG_BYTES, 'worker log size refused')
        leaves.end('decode', 'log_stat')
        leaves.begin('decode', 'log_read')
        data = leaf_rows(log_path.read_bytes(), mode == PROFILE, leaves)
        leaves.begin('decode', 'result_join')
        result.update(worker_pid=child.pid, worker_image=path, worker_initial=job.worker_start,
                      worker=worker, job=first, job_terminal_samples=[first, last], records=data)
        if mode == PROFILE:
            result['native_inclusive'] = calculate(first, worker, [row['phases']['child_end'] for row in data])
        leaves.end('decode', 'result_join')
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
    return attach_spawn(attach_leaves(attach(result, trace), leaves), spawn)
