"""Pure parent-device comparison using the frozen four-arm process-level estimator."""
from pathlib import Path
import importlib.util
import math
import statistics

spec = importlib.util.spec_from_file_location('directory_math', Path(__file__).parents[1]/'authority-directory-cost-074/review.py')
common = importlib.util.module_from_spec(spec)
spec.loader.exec_module(common)
require = common.require
ARMS = common.ARMS
ORDERS = common.ORDERS
PAYLOADS = {'marker': (65, 65), 'record': (1024, 2048)}


def parse(raw, cohort, phase):
    require(raw.decode('ascii').count('test result: ok. 1 passed; 0 failed; 0 ignored;') == 1, 'Missing exact verdict')
    origin = common.records(raw, 'HC074-PARENT-ORIGIN ')
    require(len(origin) == 1 and origin[0] == dict(cohort=cohort, phase=phase, samples=16,
        iterations=1024, uid=1000, warmups=32, product=False, admission=False), 'Changed origin')
    require(all(type(origin[0][k]) is int for k in ('cohort','samples','iterations','uid','warmups'))
        and origin[0]['product'] is False and origin[0]['admission'] is False, 'Changed origin types')
    rows = common.records(raw, 'HC074-PARENT-COST ')
    require(len(rows) == 128, 'Incomplete or repeated work')
    for index, row in enumerate(rows):
        payload = tuple(PAYLOADS)[index//64]
        sample, slot = divmod(index%64, 4)
        require(type(row['cohort']) is int and row['cohort'] == cohort and row['phase'] == phase
            and row['payload'] == payload, 'Changed cohort/phase/payload')
        require(type(row['sample']) is int and row['sample'] == sample and type(row['slot']) is int
            and row['slot'] == slot and row['variant'] == ARMS[ORDERS[(sample+cohort)%8][slot]], 'Changed order')
        for key, expected in [('calls',1024),('metadata_observations',4),('payload_bytes',PAYLOADS[payload][0]),('budget',PAYLOADS[payload][1])]:
            require(type(row[key]) is int and row[key] == expected, 'Changed work')
        require(row['product'] is False and row['admission'] is False, 'Promoted scope')
        require(all(type(row[k]) is int and row[k]>0 for k in ('cpu_ns','wall_ns')), 'Invalid clock')
    return rows


def review(cohorts, phase):
    require(phase in ('prototype','installed') and len(cohorts) == 8, 'Changed phase/process count')
    parsed = [parse(raw,i,phase) for i,raw in enumerate(cohorts)]
    payloads = {}
    for payload in PAYLOADS:
        estimates = {name: [] for name in ARMS[1:]}
        cross, times = [], []
        for rows in parsed:
            rows = [r for r in rows if r['payload'] == payload]
            pairs = {name: [] for name in ARMS[1:]}
            checks, elapsed = [], []
            for start in range(0,64,4):
                cpu = {r['variant']: r['cpu_ns'] for r in rows[start:start+4]}
                for name in pairs: pairs[name].append(math.log(cpu[name]/cpu['reference']))
                checks.append(math.log(cpu['runtime']/cpu['prototype']))
                elapsed.append(cpu['reference']/1024)
            for name in pairs: estimates[name].append(statistics.median(pairs[name]))
            cross.append(statistics.median(checks)); times.append(statistics.median(elapsed))
        ratios = {name: common.interval(values) for name,values in estimates.items()}
        runtime_to_prototype = common.interval(cross)
        controls = common.inside(ratios['reference-control']) and common.inside(
            ratios['runtime'] if phase == 'prototype' else runtime_to_prototype)
        candidate = ratios['prototype' if phase == 'prototype' else 'runtime']
        payloads[payload] = dict(ratios=ratios,runtime_to_prototype=runtime_to_prototype,
            process_log_estimates=estimates,reference_cpu_ns_per_call=statistics.median(times),
            controls_passed=controls,cpu_screen_passed=controls and candidate['upper']<=0.98)
    return dict(phase=phase,payloads=payloads,cpu_screen_passed=all(p['cpu_screen_passed'] for p in payloads.values()),
        before_read_metadata_changed=False,full_preparation_improvement_proven=False,product=False,admission=False)
