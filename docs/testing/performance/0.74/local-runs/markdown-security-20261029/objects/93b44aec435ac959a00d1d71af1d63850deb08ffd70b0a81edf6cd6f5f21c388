"""Offline diagnostics of a fixed sealed cohort; no measurement execution route."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics as stats
import sys

ROOT = Path(__file__).resolve().parents[2]
TOOLS = Path(__file__).resolve().parent
CPU_TOOLS = ROOT / 'tools/authority-runtime-cpu-074'
sys.path.insert(0, str(CPU_TOOLS))
import review as original  # noqa: E402
import seal as predecessor  # noqa: E402

need = original.need
PACKET = ROOT / 'docs/testing/performance/0.74/local-runs/authority-runtime-cpu-20261022'
INPUT_SHA = '4b86d85c55d0f2ebe36780547fcd27103de8b5368a06e623f7fb1543030b7137'
DOCS = ROOT / 'docs/testing/performance/0.74'
PLAN = DOCS / 'authority-runtime-variance-analysis-plan-20261022.md'
METHOD = DOCS / 'authority-runtime-variance-method-20261022.md'
PHASES = ('open', 'warm', 'publication', 'refusal', 'sticky', 'drop')
STRATA = {'baseline_first': [0, 2, 4, 6], 'installed_first': [1, 3, 5],
          'early': [0, 1, 2], 'middle': [3], 'late': [4, 5, 6]}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def same_checkout_source(raw, saved):
    # Git's Windows checkout representation only; no general whitespace normalization.
    return (raw == saved or (b'\r' not in saved and raw == saved.replace(b'\n', b'\r\n'))
            or (b'\r' not in raw and saved == raw.replace(b'\n', b'\r\n')))


def series(values):
    need(len(values) > 0 and all(type(v) in (int, float) and math.isfinite(v) for v in values),
         'Finite numeric series required')
    median = stats.median(values)
    return dict(values=values, min=min(values), median=median, max=max(values),
                mad=stats.median([abs(v - median) for v in values]),
                max_over_min=max(values) / min(values) if min(values) > 0 else None)


def ranks(values):
    ordered = sorted(values)
    return [stats.mean([i + 1 for i, v in enumerate(ordered) if v == value]) for value in values]


def correlation(left, right):
    need(len(left) == len(right) >= 2, 'Matched correlation membership required')
    series(left); series(right)
    def pearson(x, y):
        dx = [v - stats.mean(x) for v in x]; dy = [v - stats.mean(y) for v in y]
        denominator = math.sqrt(sum(v * v for v in dx) * sum(v * v for v in dy))
        return sum(a * b for a, b in zip(dx, dy)) / denominator if denominator else None
    return dict(n=len(left), pearson=pearson(left, right), spearman=pearson(ranks(left), ranks(right)))


def covariance(left, right):
    need(len(left) == len(right) >= 2, 'Matched covariance membership required')
    series(left); series(right)
    return sum((a - stats.mean(left)) * (b - stats.mean(right)) for a, b in zip(left, right)) / (len(left) - 1)


def bracket(costs):
    need(len(costs) == 7, 'Every bracket requires all seven pairs')
    for cost in costs: original.costs(cost)
    vectors = {key: [cost[key] for cost in costs] for key in original.FIELDS}
    residual = [v['wall_ns'] - v['cpu_ns'] for v in costs]
    discrepancy = [v['user_ns'] + v['system_ns'] - v['cpu_ns'] for v in costs]
    wall_var = stats.variance(vectors['wall_ns']); cpu_var = stats.variance(vectors['cpu_ns'])
    residual_var = stats.variance(residual); cross = 2 * covariance(vectors['cpu_ns'], residual)
    need(math.isclose(wall_var, cpu_var + residual_var + cross, rel_tol=1e-12, abs_tol=1e-6),
         'Elapsed variance identity failed')
    return dict(costs={key: series(value) for key, value in vectors.items()},
                elapsed_minus_cpu_ns=series(residual), rusage_minus_cpu_ns=series(discrepancy),
                elapsed_variance_ns2=dict(elapsed=wall_var, cpu=cpu_var, residual=residual_var, twice_covariance=cross),
                associations=dict(cpu_elapsed=correlation(vectors['cpu_ns'], vectors['wall_ns']),
                    residual_involuntary=correlation(residual, vectors['involuntary']),
                    residual_voluntary=correlation(residual, vectors['voluntary']),
                    cpu_pair_ordinal=correlation(vectors['cpu_ns'], list(range(7)))))


def stratify(ratios):
    need(len(ratios) == 7 and all(type(v) in (int, float) and math.isfinite(v) and v > 0 for v in ratios),
         'Seven positive ratios required without selection')
    return dict(all=series(ratios), **{name: dict(indices=indices, **series([ratios[i] for i in indices]))
                                     for name, indices in STRATA.items()})


def phase_sum(rows):
    need(len(rows) == 7 and all((row['mode'], row['depth'], row['fault']) == ('phases', 127, 'external')
                              for row in rows), 'Only matched maximum-depth phased fixtures')
    vectors = {phase: [row[phase]['cpu_ns'] for row in rows] for phase in PHASES}
    totals = [sum(vectors[phase][i] for phase in PHASES) for i in range(7)]
    variance = stats.variance(totals)
    contributions = {phase: covariance(values, totals) / variance if variance else None
                     for phase, values in vectors.items()}
    need(not variance or math.isclose(sum(contributions.values()), 1, rel_tol=1e-12), 'Phase covariance identity failed')
    return dict(measured_brackets_cpu_sum_ns=series(totals),
                cpu_share={phase: series([v / total for v, total in zip(values, totals)]) for phase, values in vectors.items()},
                variance_share_via_covariance=contributions, cycle_decomposition=False)


def guard():
    need(sha(PACKET / 'inventory.json') == INPUT_SHA, 'Pinned CPU inventory changed')
    for module, name in ((original, 'review.py'), (predecessor, 'seal.py')):
        need(Path(module.__file__).resolve() == (CPU_TOOLS / name).resolve(), 'Wrong historical reader imported')
    for name in ('review.py', 'seal.py', 'run.py', 'probe.rs'):
        saved = PACKET / 'source/method/tools/authority-runtime-cpu-074' / name
        need(same_checkout_source((CPU_TOOLS / name).read_bytes(), saved.read_bytes()),
             'Frozen CPU reader/method changed: ' + name)
    value = predecessor.verify(PACKET)
    qualification = DOCS / 'qualification-manifest.toml'
    need(sha(qualification) == value['raw_qualification'], 'Original qualification changed')
    return value


def derive():
    frozen = guard()
    lanes = {}; rows_count = 0
    for lane in ('194', '188'):
        pairs = []; headers = []
        for index in range(7):
            selected = {}; calibrated = {}
            for role in ('baseline', 'installed'):
                raw = (PACKET / 'measured' / f'{lane}-pair-{index}-{role}.stdout.log').read_text()
                calibrated[role], selected[role] = original.parse(raw)
                rows_count += len(selected[role])
            pairs.append(selected); headers.append(calibrated)
        need(original.summarize(pairs) == frozen['lanes'][lane], 'Original paired facts changed')
        cells = []
        for cell, (mode, depth, fault) in enumerate(original.CELLS):
            phases = ('cycle',) if mode == 'cycle' else tuple(p for p in PHASES if pairs[0]['baseline'][cell][p] is not None)
            costs = {role: {phase: bracket([pair[role][cell][phase] for pair in pairs]) for phase in phases}
                     for role in ('baseline', 'installed')}
            cells.append(dict(mode=mode, depth=depth, fault=fault, roles=costs))
        primary = {}
        for label, name in (('warm_cpu', 'primary_warm_cpu'), ('cycle_elapsed', 'primary_cycle_elapsed')):
            primary[label] = stratify(frozen[name]['lanes'][lane]['paired_ratios'])
        overhead = {}
        for role in ('baseline', 'installed'):
            empty = [value for header in headers for value in header[role]['empty']]
            overhead[role] = {}
            for field in ('cpu_ns', 'wall_ns'):
                maximum = max(value[field] for value in empty)
                overhead[role][field] = dict(empty=series([value[field] for value in empty]),
                    observed_max_empty_over_minimum_workload=[dict(mode=mode, depth=depth, fault=fault,
                        phase=phase, ratio=maximum / data['costs'][field]['min'])
                        for row in cells for mode, depth, fault in [(row['mode'], row['depth'], row['fault'])]
                        for phase, data in row['roles'][role].items()])
        lanes[lane] = dict(cells=cells, primary_strata=primary, calibration=overhead,
            interrupted_phase_cpu={role: phase_sum([pair[role][3] for pair in pairs]) for role in ('baseline', 'installed')})
    need(rows_count == 336 and guard() == frozen, 'Input changed during offline analysis')
    receipt = json.loads((PACKET / 'measured/result.json').read_text())
    return dict(schema='authority-runtime-variance-074-v1', input_inventory_sha256=INPUT_SHA,
                method=frozen['method'], runtime_sources=frozen['runtime_sources'],
                original_primary={key: frozen[key] for key in ('primary_warm_cpu', 'primary_cycle_elapsed')},
                qualification=frozen['raw_qualification'], processes=28, rows=rows_count, lanes=lanes,
                cohort_boundary_context={key: receipt[key] for key in ('pressure_before', 'pressure_after')},
                post_hoc=True, measurement_executed=False, causal_attribution=False, server_admitted=False)


def publish(folder, checks):
    need(not folder.exists(), 'Never overwrite derived evidence')
    value = derive()
    files = {f'source/{path.name}': path for path in (TOOLS / 'analyze.py', TOOLS / 'test_analysis.py', TOOLS / 'README.md', PLAN, METHOD)}
    if checks:
        need(checks.is_dir() and any(checks.iterdir()), 'Checks directory empty')
        for path in sorted(checks.iterdir()):
            need(path.is_file() and not path.is_symlink() and path.suffix == '.log', 'Only check log files accepted')
            files['checks/' + path.name] = path
    folder.mkdir(parents=True)
    original.method.common.publish(folder / 'facts.json', value)
    for name, source in files.items():
        target = folder / name; target.parent.mkdir(exist_ok=True)
        with target.open('xb') as stream: stream.write(source.read_bytes())
    original.method.common.publish(folder / 'inventory.json', dict(schema='authority-runtime-variance-inventory-074-v1', files=predecessor.inventory(folder)))
    verify(folder)


def verify(folder):
    rows = json.loads((folder / 'inventory.json').read_text())['files']
    for row in rows: original.method.common.member(row['path'])
    need(rows == predecessor.inventory(folder), 'Derived packet membership/bytes changed')
    need(json.loads((folder / 'facts.json').read_text()) == derive(), 'Derived facts changed')
    for source in (TOOLS / 'analyze.py', TOOLS / 'test_analysis.py', TOOLS / 'README.md', PLAN, METHOD):
        need(sha(folder / 'source' / source.name) == sha(source), 'Analysis source/method changed')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--publish', type=Path); group.add_argument('--verify', type=Path)
    parser.add_argument('--checks', type=Path)
    args = parser.parse_args()
    need(not args.checks or args.publish, 'Checks require new publication')
    if args.publish: publish(args.publish, args.checks)
    else: verify(args.verify)
    print(json.dumps(dict(input_sha256=INPUT_SHA, processes=28, rows=336, post_hoc=True,
                          measurement_executed=False, original_primaries_unchanged=True), sort_keys=True))
