"""Pure balanced directory-method CPU review. Never admission or a guard."""
import json
import math
import random
import statistics

ARMS = ('reference', 'reference-control', 'runtime', 'prototype')
LAYOUTS = ('fixture', 'fixed')
ORDERS = ((0, 1, 2, 3), (1, 2, 3, 0), (2, 3, 0, 1), (3, 0, 1, 2),
          (0, 3, 2, 1), (1, 0, 3, 2), (2, 1, 0, 3), (3, 2, 1, 0))


def require(value, why):
    if not value:
        raise ValueError(why)


def records(raw, marker):
    rows = []
    for line in raw.decode('ascii').splitlines():
        if marker in line:
            require(line.count(marker) == 1, 'Duplicate marker')
            rows.append(json.loads(line.split(marker, 1)[1]))
    return rows


def parse(raw, cohort, phase):
    text = raw.decode('ascii')
    require(text.count('test result: ok. 1 passed; 0 failed; 0 ignored;') == 1, 'Missing exact verdict')
    origins = records(raw, 'HC074-DIR-ORIGIN ')
    require(len(origins) == 1 and origins[0] == dict(cohort=cohort, phase=phase, samples=16,
            iterations=1024, uid=1000, warmups=32, product=False, admission=False), 'Changed origin')
    require(all(type(origins[0][key]) is int for key in ('cohort', 'samples', 'iterations', 'uid', 'warmups'))
            and origins[0]['product'] is False and origins[0]['admission'] is False, 'Changed origin types')
    rows = records(raw, 'HC074-DIR-COST ')
    require(len(rows) == 128, 'Incomplete or repeated work')
    for index, row in enumerate(rows):
        layout = LAYOUTS[index // 64]
        sample, slot = divmod(index % 64, 4)
        require(type(row['cohort']) is int and row['cohort'] == cohort and row['phase'] == phase and row['layout'] == layout,
                'Changed cohort/phase/layout')
        require(type(row['sample']) is int and row['sample'] == sample
                and type(row['slot']) is int and row['slot'] == slot, 'Changed sample order')
        require(row['variant'] == ARMS[ORDERS[(sample + cohort) % 8][slot]], 'Changed variant order')
        require(type(row['calls']) is int and row['calls'] == 1024
                and row['metadata_queries_per_call'] == (4 if layout == 'fixture' else 11), 'Changed work')
        require(row['product'] is False and row['admission'] is False, 'Promoted scope')
        require(all(type(row[k]) is int and row[k] > 0 for k in ('cpu_ns', 'wall_ns')), 'Invalid clock')
    return rows


def percentile(values, fraction):
    values = sorted(values)
    position = (len(values) - 1) * fraction
    low = int(position)
    high = min(low + 1, len(values) - 1)
    return values[low] + (values[high] - values[low]) * (position - low)


def interval(values):
    rng = random.Random(0x074D1A05)
    boot = [statistics.median(rng.choices(values, k=8)) for _ in range(20000)]
    return dict(ratio=math.exp(statistics.median(values)),
                lower=math.exp(percentile(boot, 0.025)), upper=math.exp(percentile(boot, 0.975)))


def inside(value):
    return value['lower'] >= 0.97 and value['upper'] <= 1.03


def review(cohorts, phase):
    require(phase in ('prototype', 'installed') and len(cohorts) == 8, 'Changed phase/process count')
    parsed = [parse(raw, i, phase) for i, raw in enumerate(cohorts)]
    layouts = {}
    for layout in LAYOUTS:
        estimates = {name: [] for name in ARMS[1:]}
        runtime_to_prototype = []
        reference_cpu = []
        for rows in parsed:
            rows = [row for row in rows if row['layout'] == layout]
            pairs = {name: [] for name in ARMS[1:]}
            check = []
            times = []
            for start in range(0, 64, 4):
                cpu = {r['variant']: r['cpu_ns'] for r in rows[start:start+4]}
                for name in pairs:
                    pairs[name].append(math.log(cpu[name] / cpu['reference']))
                check.append(math.log(cpu['runtime'] / cpu['prototype']))
                times.append(cpu['reference'] / 1024)
            for name in pairs:
                estimates[name].append(statistics.median(pairs[name]))
            runtime_to_prototype.append(statistics.median(check))
            reference_cpu.append(statistics.median(times))
        ratios = {name: interval(v) for name, v in estimates.items()}
        cross = interval(runtime_to_prototype)
        controls = inside(ratios['reference-control'])
        if phase == 'prototype':
            controls = controls and inside(ratios['runtime'])
        else:
            controls = controls and inside(cross)
        candidate = ratios['prototype' if phase == 'prototype' else 'runtime']
        layouts[layout] = dict(ratios=ratios, runtime_to_prototype=cross,
            reference_cpu_ns_per_call=statistics.median(reference_cpu),
            process_log_estimates=estimates, controls_passed=controls,
            cpu_screen_passed=controls and candidate['upper'] <= 0.95)
    return dict(phase=phase, layouts=layouts,
                cpu_screen_passed=all(r['cpu_screen_passed'] for r in layouts.values()),
                full_preparation_improvement_proven=False, product=False, admission=False)
