"""Offline preregistered leaf comparison; this never grants runtime admission."""
import math
import random
import statistics

LABELS = ('reference', 'reference-control', 'fstat-prototype')
ORDERS = ((0, 1, 2), (1, 2, 0), (2, 0, 1), (0, 2, 1), (2, 1, 0), (1, 0, 2))


def require(value, why):
    if not value:
        raise ValueError(why)


def fields(line):
    return dict(token.split('=', 1) for token in line.split()[1:])


def parse(raw, cohort):
    lines = raw.decode('ascii').splitlines()
    require(len(lines) == 37 and lines[0].startswith('HC074-FD-IDENTITY-ORIGIN '), 'Incomplete cohort')
    origin = fields(lines[0])
    require(origin == dict(cohort=str(cohort), files='7', iterations='8192', samples='12',
                           product='false', admission='false'), 'Changed origin/work')
    rows = []
    for index, line in enumerate(lines[1:]):
        require(line.startswith('HC074-FD-IDENTITY '), 'Unexpected record')
        row = fields(line)
        sample, slot = divmod(index, 3)
        require(row['cohort'] == str(cohort) and row['sample'] == str(sample)
                and row['slot'] == str(slot), 'Reordered or duplicate sample')
        require(row['variant'] == LABELS[ORDERS[(sample + cohort) % 6][slot]], 'Changed order')
        require(row['calls'] == str(8192 * 7), 'Unequal work')
        for field in ('cpu_ns', 'wall_ns'):
            require(row[field].isdigit() and int(row[field]) > 0, 'Invalid clock')
        rows.append(row)
    return rows


def percentile(values, fraction):
    values = sorted(values)
    position = (len(values) - 1) * fraction
    lower = int(position)
    upper = min(lower + 1, len(values) - 1)
    return values[lower] + (values[upper] - values[lower]) * (position - lower)


def interval(estimates):
    rng = random.Random(0x074FD105)
    boot = [statistics.median(rng.choices(estimates, k=8)) for _ in range(20000)]
    return dict(ratio=math.exp(statistics.median(estimates)),
                lower=math.exp(percentile(boot, 0.025)), upper=math.exp(percentile(boot, 0.975)))


def review(cohorts):
    require(len(cohorts) == 8, 'Not eight independent processes')
    estimates = {label: [] for label in LABELS[1:]}
    for cohort, raw in enumerate(cohorts):
        rows = parse(raw, cohort)
        groups = [rows[index:index + 3] for index in range(0, 36, 3)]
        for label in estimates:
            ratios = []
            for group in groups:
                cpu = {row['variant']: int(row['cpu_ns']) for row in group}
                ratios.append(math.log(cpu[label] / cpu['reference']))
            estimates[label].append(statistics.median(ratios))
    control = interval(estimates['reference-control'])
    candidate = interval(estimates['fstat-prototype'])
    control_ok = control['lower'] >= 0.97 and control['upper'] <= 1.03
    return dict(control=control, candidate=candidate, process_log_estimates=estimates,
                same_function_control_passed=control_ok,
                leaf_screen_passed=control_ok and candidate['upper'] <= 0.95,
                runtime_integrated=False, runtime_improvement_proven=False,
                release_gate=False, admission=False)


def review_origin(cohorts):
    require(len(cohorts) == 8, 'Not eight independent origin processes')
    estimates = {'reference-control': [], 'fstat-installed': []}
    labels = ('reference', 'reference-control', 'fstat-installed')
    for cohort, raw in enumerate(cohorts):
        text = raw.decode('ascii')
        require(text.count('test result: ok. 1 passed; 0 failed; 0 ignored;') == 1,
                'Missing exact original test verdict')
        rows = []
        for line in text.splitlines():
            marker = 'HC074-ORIGIN-IDENTITY '
            if marker in line:
                require(line.count(marker) == 1, 'Duplicate marker')
                rows.append(fields(marker + line.split(marker, 1)[1]))
        require(len(rows) == 36, 'Incomplete origin comparison')
        for index, row in enumerate(rows):
            sample, slot = divmod(index, 3)
            require(row['cohort'] == str(cohort) and row['sample'] == str(sample)
                    and row['slot'] == str(slot) and row['calls'] == '1024', 'Changed origin work/order')
            require(row['variant'] == labels[ORDERS[(sample + cohort) % 6][slot]], 'Changed origin variant')
            require(all(row[k].isdigit() and int(row[k]) > 0 for k in ('cpu_ns', 'wall_ns')), 'Invalid origin clock')
        for label in estimates:
            ratios = []
            for index in range(0, 36, 3):
                cpu = {row['variant']: int(row['cpu_ns']) for row in rows[index:index + 3]}
                ratios.append(math.log(cpu[label] / cpu['reference']))
            estimates[label].append(statistics.median(ratios))
    control, installed = interval(estimates['reference-control']), interval(estimates['fstat-installed'])
    control_ok = control['lower'] >= 0.97 and control['upper'] <= 1.03
    return dict(control=control, installed=installed, process_log_estimates=estimates,
                same_function_control_passed=control_ok,
                origin_cpu_screen_passed=control_ok and installed['upper'] <= 0.98,
                full_preparation_proven=False, product_improvement_proven=False,
                release_gate=False, admission=False)
