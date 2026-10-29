"""One distinct repaired GNU attempt per fixed mode, never release admission."""
import argparse
import json
import os
from pathlib import Path
import sys

import review as r

BASE = r.CONTROL/'target/gnu-frontier-local-20261027'
CONTROL_PHASE = 'controls-native'
OWNED_SHA = '2f43410ab4c579f7119bdbd4746fed3710694627caf373fde56fd070bba21f2f'
GROUP_SHA = 'e5ff52835fc2486171dbdbbb1b6df47177f583b4539f561779992c2c5065a691'
PARSER_SHA = '0229d4067fffd0d0b5acee7d864efc64aadfb201de1ea3f17944a2f8dddf3c4d'
FOCUSED = sorted(['pid_stage_join_074','pid_path_cost_074','fresh_kernel_cost_074',
    'remaining_readers_074','package_closure_inputs_074','pid_path_reader_074',
    'fresh_status_reader_074','memory_compat_071','memory_profile_071',
    'package_frontier_inputs_074','frontier_current_view_074','authority_rust_history_074','current_tree_input_audit_074',
    'historical_source_cache_074','role_census_reader_074','validation_leaf_reader_074'])
CONTROL_SOURCES = ['tools/package-frontier-gate-074/'+n for n in ('review.py','runner.py','test_runner.py','inputs.json')]

def command(mode):
    task = 'test' if mode in ('focused','full') else mode
    argv = ['/home/artur/.cargo/bin/cargo','+1.94.0',task,'--offline','--locked']
    if task=='test':
        argv += ['--no-fail-fast']
    argv += ['--target-dir',str(r.TARGET),'-p','xtask']
    if mode=='full':
        argv += ['--lib','--tests']
    elif mode=='focused':
        for name in FOCUSED:
            argv += ['--test',name]
    else:
        argv += ['--all-targets']
        if mode=='clippy':
            argv += ['--all-features','--','-D','warnings']
    return argv+(['--','--test-threads=4'] if task=='test' else [])

def check_controls(root):
    folder=root/'target/gnu-frontier-local-20261027'/CONTROL_PHASE
    intent=json.loads((folder/'intent.json').read_bytes())
    result=json.loads((folder/'result.json').read_bytes())
    r.require(result['exit']==0 and result['outer_refusal'] is None and result['source_unchanged'] and result['commit_unchanged'],'controls not green')
    r.require(intent['argv']==['wsl.exe','-d','Ubuntu','-u','artur','--exec','/usr/bin/python3','-m','unittest','discover','-s',str(r.CONTROL/'tools/package-frontier-gate-074'),'-p','test_runner.py','-v'],'control command drift')
    for name in CONTROL_SOURCES:
        digest=r.sha((root/name).read_bytes())
        r.require(intent['source'][name]==result['source_after'][name]==digest,'controls source drift')

def check_focused():
    folder=BASE/'focused'
    result=json.loads((folder/'result.json').read_bytes())
    receipt=json.loads((folder/'controller.json').read_bytes())
    accounting=json.loads((folder/'accounting.json').read_bytes())
    r.require(result['mode']=='focused' and result['source']==r.SOURCE and result['cohort_pass'] and not result['package_pass'],'focused gate not green')
    r.require(result['request_sha256']==r.REQUEST_SHA and receipt['classification']=='success' and receipt['retired'] and not receipt['cleanup_errors'],'focused receipt drift')
    r.require(accounting['complete'] and accounting['expected_targets']==16 and accounting['passed']==67 and accounting['failed']==accounting['ignored']==0,'focused coverage drift')
    intent=json.loads((folder/'intent.json').read_bytes())
    r.require(intent['argv']==command('focused'),'focused command drift')
    r.require(intent['helpers']==helper_pins(),'focused helpers stale')

def helper_pins():
    return {name:r.sha((r.CONTROL/name).read_bytes()) for name in CONTROL_SOURCES}

def execute(mode):
    out=BASE/mode
    out.mkdir(exist_ok=False)
    def write(name,value):
        with (out/name).open('x',encoding='utf-8',newline='\n') as stream:
            json.dump(value,stream,indent=2,sort_keys=True); stream.write('\n')
    write('intent.json',dict(source=r.SOURCE,mode=mode,argv=command(mode),helpers=helper_pins(),
        bound_seconds=780,test_threads=4,floor_bytes=4*1024**3,retry=False,
        server=False,services=False,clocks=False,product_load=False,qualification=False))
    try:
        check_controls(r.CONTROL)
        if mode=='full':
            check_focused()
        before=r.review()
        write('review-before.json',before)
        sys.path.insert(0,str(r.CONTROL/'tools/package-closure-offline-074'))
        owned=r.load(r.CONTROL/'tools/package-closure-offline-074/gnu_package.py','checked_repaired_owned',OWNED_SHA)
        group=r.load(r.CONTROL/'scripts/ci/watchdog_linux_group.py','checked_repaired_group',GROUP_SHA)
        parser=r.load(r.CONTROL/'tools/xtask-package-attribution-074/review.py','checked_repaired_parser',PARSER_SHA)
        low=[before['physical_free'],before['native_free']]
        def capacity():
            import shutil
            free=[shutil.disk_usage('/mnt/c').free,shutil.disk_usage(r.ROOT).free]
            low[:]=[min(a,b) for a,b in zip(low,free)]
            if min(free)<4*1024**3:
                raise owned.CapacityError('unchanged physical/native 4 GiB floor')
        receipt=owned.owned(group.LinuxGroup(),command(mode),r.ROOT,out,capacity)
        write('controller.json',receipt)
        r.require(receipt['retired'],'original CI children not retired')
        after=r.review()
        write('review-after.json',after)
        write('capacity.json',dict(low_physical_free_bytes=low[0],low_native_free_bytes=low[1],sampled_floor_bytes=4*1024**3,hard_quota=False))
        accounting=None
        if mode in ('full','focused') and receipt['classification'] in ('success','test-failure','timeout'):
            expected=after['targets'] if mode=='full' else ['tests/'+n+'.rs' for n in FOCUSED]
            accounting=parser.parse_transcript((out/'stdout.log').read_text(),(out/'stderr.log').read_text(),expected,
                receipt['exit'],'outer-timeout' if receipt['classification']=='timeout' else None,780)
            write('accounting.json',accounting)
        passed=receipt['classification']=='success' and not receipt['cleanup_errors']
        if mode in ('full','focused'):
            passed=passed and bool(accounting and accounting['package_pass'])
        write('result.json',dict(source=r.SOURCE,request_sha256=r.REQUEST_SHA,mode=mode,
            source_and_refs_and_environment_unchanged=True,cohort_pass=passed if mode=='focused' else False,
            check_pass=passed if mode in ('check','clippy') else False,
            package_pass=passed if mode=='full' else False,release_admitted=False))
        print((out/'result.json').read_text())
        return int(not passed)
    except BaseException as error:
        write('refusal.json',dict(type=type(error).__name__,reason=str(error),package_pass=False,release_admitted=False))
        raise

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mode',choices=('focused','full','check','clippy'),default='full')
    parser.add_argument('--execute',action='store_true')
    args=parser.parse_args()
    if args.execute:
        raise SystemExit(execute(args.mode))
    print(json.dumps(dict(mode=args.mode,argv=command(args.mode),bound_seconds=780,floor_bytes=4*1024**3,execute=False,package_pass=False)))
