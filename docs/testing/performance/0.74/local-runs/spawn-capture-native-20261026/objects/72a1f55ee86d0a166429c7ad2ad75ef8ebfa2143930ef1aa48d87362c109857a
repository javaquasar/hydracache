"""Explicit separate spawn diagnostic gate; never a V6 or product launcher."""
import hashlib
import json
import re
import shutil
import subprocess
import sys
import time

from git_cpu_overhead import (ROOT, BOUND, CLEANUP, FREE_BYTES, LOG_BYTES, CALLS,
    WindowsApi, artifact, pins, require, schedule, summarize)


def run(build_label, label, *, spawn_frames=False):
    require(type(spawn_frames) is bool and spawn_frames, 'explicit boolean spawn frames gate required')
    require(all(re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', value) for value in (build_label, label)), 'bounded labels')
    folder = ROOT/'target/git-cpu-spawn-capture-20261026'/label
    folder.mkdir(parents=True, exist_ok=False)
    def write(name, value):
        with (folder/name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write('\n')
    result = dict(passed=False, first_refusal=None, attempts=[], package_pass=False,
                  overhead_accepted=False, speedup_established=False, server=False)
    write('intent.json', dict(build_label=build_label, schedule=schedule(), calls_per_worker=CALLS,
        bound_seconds=BOUND, cleanup_seconds=CLEANUP, free_bytes=FREE_BYTES, log_bytes=LOG_BYTES,
        coordinator_frames=dict(enabled=True, schema='hc074-coordinator-frames-v1'),
        capture_leaves=dict(enabled=True, schema='hc074-capture-leaves-v1'),
        spawn_frames=dict(enabled=True, schema='hc074-spawn-capture-v1'),
        job_overhead_measured=False, pure_counter_api_cost_isolated=False, overhead_accepted=False,
        git_suspended=False, stdin_hold=False, native_bypass=False, environment_changes=False,
        retry=False, package_pass=False, server=False))
    try:
        import coordinator_frames as frames
        import capture_leaves as leaf_frames
        import spawn_capture as capture
        result['coordinator_frames_contract'] = frames.contract()
        result['capture_leaves_contract'] = leaf_frames.contract()
        result['spawn_frames_contract'] = capture.contract()
        require(result['coordinator_frames_contract']['schema'] == 'hc074-coordinator-frames-v1' and
                result['capture_leaves_contract']['schema'] == 'hc074-capture-leaves-v1' and
                result['spawn_frames_contract']['schema'] == 'hc074-spawn-capture-v1', 'spawn diagnostic schema drift')
        scope = result['spawn_frames_contract']
        require(scope['model_schema'] == 'hc074-spawn-frames-model-v1' and
                scope['phases'] == ['preparation', 'create_to_retention', 'membership_release'] and
                all(type(scope[k]) is int and scope[k] == v for k, v in
                    (('max_additional_probes', 6), ('max_additional_clock_reads', 18),
                     ('max_recorder_clock_reads_with_outer_and_leaves', 102))) and
                all(scope[k] is True for k in ('successful_create_to_retention_has_no_added_probe',
                    'failed_create_last_error_read_unchanged', 'complete_frame_does_not_prove_resume', 'caller_connected')) and
                all(scope[k] is False for k in ('cleanup_instrumented', 'pure_create_cost_isolated',
                    'existing_predicates_removed_or_reordered', 'window_renewed', 'overhead_accepted', 'package_pass')),
                'spawn diagnostic scope drift')
        result['pins_before'] = pins()
        sys.path.insert(0, str(ROOT/'tools/historical-blob-batch-074'))
        import package_effects_review_v6 as gate
        result['environment'] = gate.environment_preflight(ROOT)
        require(shutil.disk_usage(ROOT).free >= FREE_BYTES, 'disk preflight floor')
        image, build = artifact(ROOT/'target/source-continuation-20261024'/build_label)
        image_sha = hashlib.sha256(image.read_bytes()).hexdigest()
        write('build.json', dict(image=str(image), image_sha256=image_sha, source=build['source'], head=build['head']))
        deadline = time.monotonic()+BOUND
        api, observer = capture.SpawnAccountingApi(), WindowsApi()
        for block, slot, mode in schedule():
            attempt = capture.measure(api, observer, image, block, slot, mode, folder, deadline,
                                      frames.Tape(), leaf_frames.Leaves(), capture.SpawnTape())
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
    require(len(sys.argv) == 4 and sys.argv[3] == '--spawn-frames',
            'usage: git_cpu_spawn.py BUILD_LABEL NEW_LABEL --spawn-frames')
    result = run(*sys.argv[1:3], spawn_frames=True)
    print(json.dumps(dict(passed=result['passed'], first_refusal=result['first_refusal'], package_pass=False)))
    raise SystemExit(0 if result['passed'] else 1)
