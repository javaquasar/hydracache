"""Fixed local observer attribution inside the SAME accounting Job, never V6."""
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

from git_job_cpu import (AccountingApi, AccountingJob, BOUND, CALLS, CLEANUP, FREE_BYTES,
                         LOG_BYTES, ROOT, WindowsApi, artifact, calculate, cleanup, pins,
                         require, rows, terminal_job, validate)

BLOCKS = 4
REFERENCE, PROFILE = 'reference_cpu_child', 'accounted_cpu_child'
METRICS = ('wall_ns', 'whole_job_cpu_ns', 'whole_worker_cpu_ns', 'coordinator_cpu_ns')


def schedule():
    return [(block, slot, mode) for block in range(BLOCKS) for slot, mode in enumerate(
        (REFERENCE, PROFILE, PROFILE, REFERENCE) if block % 2 == 0 else
        (PROFILE, REFERENCE, REFERENCE, PROFILE))]


def summarize(attempts):
    require(len(attempts) == BLOCKS*4, 'incomplete cohort; never select successful subset')
    blocks = []
    for attempt, expected in zip(attempts, schedule()):
        require((attempt['block'], attempt['slot'], attempt['mode']) == expected and
                attempt['passed'] is True and attempt['first_refusal'] is None and
                attempt['cleanup_errors'] == [], 'cohort schedule/outcome drift')
        metric = attempt['metrics']
        require(set(metric) == set(METRICS) and all(type(v) is int and v >= 0 for v in metric.values())
                and metric['wall_ns'] > 0 and metric['whole_job_cpu_ns'] >= metric['whole_worker_cpu_ns'],
                'invalid cohort metric')
    for block in range(BLOCKS):
        selected = attempts[block*4:(block+1)*4]
        reference = [v for v in selected if v['mode'] == REFERENCE]
        profile = [v for v in selected if v['mode'] == PROFILE]
        metrics = {}
        for metric in METRICS:
            a, b = (sum(v['metrics'][metric] for v in values) for values in (reference, profile))
            metrics[metric] = dict(reference_sum=a, profile_sum=b, delta=b-a,
                ratio_numerator=b if a else None, ratio_denominator=a if a else None,
                unavailable_reason=None if a else 'zero reference counter')
        blocks.append(dict(block=block, calls_per_arm=8, metrics=metrics))
    return dict(blocks=blocks, workers=16, git_calls=64, statistic='four block sums; no trimming or CI',
        comparison='profiled versus output inside identical unlimited Job', job_overhead_measured=False,
        pure_counter_api_cost_isolated=False, overhead_accepted=False, speedup_established=False, package_pass=False)


def measure(api, observer, image, block, slot, mode, folder, deadline):
    result = dict(block=block, slot=slot, mode=mode, passed=False, first_refusal=None, cleanup_errors=[])
    job = child = None
    wall, cpu = time.monotonic_ns(), time.process_time_ns()
    try:
        require(time.monotonic() < deadline, 'worker frame started too late')
        job = AccountingJob(api, observer, image)
        require(api.counters(job.handle) == dict(user_ns=0, kernel_ns=0, total=0, active=0, terminated=0),
                'new accounting Job has prior charges')
        log_path = folder/f'{block}-{slot}-{mode}.log'
        with log_path.open('xb') as log:
            child = job.spawn([str(image), '--exact', mode, '--nocapture', '--test-threads=1'], ROOT, log, deadline)
            code = child.wait(timeout=max(0, deadline-time.monotonic()))
        require(time.monotonic() < deadline and code == 0, 'worker exit/deadline refused')
        pid, path = job.worker_identity
        require(observer.dll.GetProcessId(child.handle) == pid == child.pid, 'retained terminal worker PID drift')
        worker = observer.read(child.handle)
        validate(job.worker_start, worker)
        first, last = api.counters(job.handle), api.counters(job.handle)
        terminal_job(first, last)
        require(first['terminated'] == 0, 'unexpected forced Job termination')
        require(log_path.stat().st_size <= LOG_BYTES, 'worker log size refused')
        data = rows(log_path.read_bytes(), mode == PROFILE)
        result.update(worker_pid=child.pid, worker_image=path, worker_initial=job.worker_start,
                      worker=worker, job=first, job_terminal_samples=[first, last], records=data)
        if mode == PROFILE:
            result['native_inclusive'] = calculate(first, worker, [row['phases']['child_end'] for row in data])
        child.close()
        job.close()
        child = job = None
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
    return result


def run(build_label, label, *, coordinator_frames=False, capture_leaves=False):
    require(type(coordinator_frames) is bool, 'explicit boolean coordinator frames gate')
    require(type(capture_leaves) is bool and (not capture_leaves or coordinator_frames), 'explicit leaf gate requires coordinator frames')
    require(all(re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', value) for value in (build_label, label)), 'bounded labels')
    folder = ROOT/('target/git-cpu-capture-leaves-20261026' if capture_leaves else
                  'target/git-cpu-coordinator-frames-20261026' if coordinator_frames else
                   'target/git-cpu-overhead-20261026')/label
    folder.mkdir(parents=True, exist_ok=False)
    def write(name, value):
        with (folder/name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write('\n')
    result = dict(passed=False, first_refusal=None, attempts=[], package_pass=False,
                  overhead_accepted=False, speedup_established=False, server=False)
    intent = dict(build_label=build_label, schedule=schedule(), calls_per_worker=CALLS,
        bound_seconds=BOUND, cleanup_seconds=CLEANUP, free_bytes=FREE_BYTES, log_bytes=LOG_BYTES,
        job_overhead_measured=False, pure_counter_api_cost_isolated=False, overhead_accepted=False,
        git_suspended=False, stdin_hold=False, native_bypass=False, environment_changes=False,
        retry=False, package_pass=False, server=False)
    if coordinator_frames:
        intent['coordinator_frames'] = dict(enabled=True, schema='hc074-coordinator-frames-v1')
    if capture_leaves:
        intent['capture_leaves'] = dict(enabled=True, schema='hc074-capture-leaves-v1')
    write('intent.json', intent)
    try:
        if coordinator_frames:
            import coordinator_frames as frames
            result['coordinator_frames_contract'] = frames.contract()
            require(result['coordinator_frames_contract']['schema'] == intent['coordinator_frames']['schema'],
                    'coordinator frames schema drift')
        if capture_leaves:
            import capture_leaves as leaf_frames
            result['capture_leaves_contract'] = leaf_frames.contract()
            require(result['capture_leaves_contract']['schema'] == intent['capture_leaves']['schema'], 'capture leaves schema drift')
        result['pins_before'] = pins()
        sys.path.insert(0, str(ROOT/'tools/historical-blob-batch-074'))
        import package_effects_review_v6 as gate
        result['environment'] = gate.environment_preflight(ROOT)
        require(shutil.disk_usage(ROOT).free >= FREE_BYTES, 'disk preflight floor')
        image, build = artifact(ROOT/'target/source-continuation-20261024'/build_label)
        image_sha = hashlib.sha256(image.read_bytes()).hexdigest()
        write('build.json', dict(image=str(image), image_sha256=image_sha, source=build['source'], head=build['head']))
        deadline = time.monotonic()+BOUND
        api, observer = AccountingApi(), WindowsApi()
        for block, slot, mode in schedule():
            if capture_leaves:
                attempt = leaf_frames.measure(api, observer, image, block, slot, mode, folder, deadline,
                                              frames.Tape(), leaf_frames.Leaves())
            elif coordinator_frames:
                attempt = frames.measure(api, observer, image, block, slot, mode, folder, deadline, frames.Tape())
            else:
                attempt = measure(api, observer, image, block, slot, mode, folder, deadline)
            result['attempts'].append(attempt)
            write(f'{block}-{slot}-result.json', attempt)
            require(attempt['passed'], 'worker refused: '+str(attempt['first_refusal'])+
                    '; cleanup='+str(attempt['cleanup_errors']))
        result['analysis'] = summarize(result['attempts'])
        result['pins_after'] = pins()
        require(hashlib.sha256(image.read_bytes()).hexdigest() == image_sha and
                all(hashlib.sha256((ROOT/name).read_bytes()).hexdigest() == value for name, value in build['source'].items()),
                'postflight image/source drift')
        require(subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() == build['head'],
                'postflight HEAD drift')
        require(shutil.disk_usage(ROOT).free >= FREE_BYTES, 'disk postflight floor')
        require(time.monotonic() < deadline, 'late cohort witness')
        result['passed'] = True
    except BaseException as error:
        result['first_refusal'] = f'{type(error).__name__}: {error}'
    finally:
        write('result.json', result)
    return result


if __name__ == '__main__':
    require(len(sys.argv) == 3 or (len(sys.argv) == 4 and sys.argv[3:] == ['--coordinator-frames']) or
            (len(sys.argv) == 5 and sys.argv[3:] == ['--coordinator-frames', '--capture-leaves']),
            'usage: git_cpu_overhead.py BUILD_LABEL NEW_LABEL [--coordinator-frames [--capture-leaves]]')
    result = run(*sys.argv[1:3], coordinator_frames=len(sys.argv) >= 4, capture_leaves=len(sys.argv) == 5)
    print(json.dumps(dict(passed=result['passed'], first_refusal=result['first_refusal'], package_pass=False)))
    raise SystemExit(0 if result['passed'] else 1)
