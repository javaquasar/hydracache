"""Fixed v2 schedule, unchanged historical probe and offline input guards."""
from collections import Counter
from fractions import Fraction
import json
import math
from pathlib import Path, PureWindowsPath
import statistics
import sys

ROOT = Path(__file__).resolve().parents[2]
TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / 'tools/authority-runtime-variance-074'))
import analyze as history  # noqa: E402

cpu_review = history.original
cpu = cpu_review.method
common = cpu.common
need = common.need
BLOCKS = ('BIIB', 'IBBI', 'IBBI', 'BIIB', 'IBBI', 'BIIB', 'BIIB', 'IBBI')
SCHEDULE_SHA = '1566ab8cbfa4cbdea2897737be07ddf533142ede5a6c6dbbc4f5bf36f6f638c1'
PROCESS_SECONDS = 300
COHORT_SECONDS = 3600
FREEZE_SECONDS = 600
ROLE = {'B': 'baseline', 'I': 'installed'}
COMPILERS = {'194': '1.94.0', '188': '1.88.0'}
INPUTS = [f'tools/authority-runtime-balanced-074/{name}' for name in
          ('balanced.py', 'runner.py', 'evaluate.py', 'test_runner.py', 'README.md')] + [
    'tools/authority-runtime-cpu-074/' + name for name in ('run.py', 'probe.rs', 'review.py', 'seal.py')] + [
    'tools/authority-runtime-allocation-074/run.py', 'tools/authority-runtime-variance-074/analyze.py',
    'docs/testing/performance/0.74/authority-runtime-variance-method-20261022.md',
    'docs/testing/performance/0.74/authority-runtime-variance-method-v2-20261022.md']


def schedule():
    return [dict(block=block, compiler=lane, position=position, role=role)
            for block, word in enumerate(BLOCKS)
            for lane in (('194', '188') if block % 2 == 0 else ('188', '194'))
            for position, role in enumerate(word)]


def schedule_digest(rows):
    return common.sha_bytes(json.dumps(rows, sort_keys=True, separators=(',', ':')).encode())


def validate_schedule(rows):
    need(rows == schedule() and len(rows) == 64, 'Exact ordered 64-process schedule required')
    need(all(set(row) == {'block', 'compiler', 'position', 'role'} and
             type(row['block']) is int and type(row['position']) is int for row in rows), 'Invalid schedule record')
    need(schedule_digest(rows) == SCHEDULE_SHA, 'Prepared schedule digest changed')
    need(set(Counter((r['compiler'], r['role']) for r in rows).values()) == {16}, 'Role membership changed')
    need(set(Counter((r['compiler'], r['role'], r['position']) for r in rows).values()) == {4}, 'Unbalanced positions')
    return rows


def plan():
    return dict(schema='authority-runtime-balanced-plan-074-v2', schedule=validate_schedule(schedule()),
                schedule_sha256=SCHEDULE_SHA, processes=64, observations_per_role_per_compiler=16,
                block_ratios_per_compiler=8, rows=768, timed_warm_inspections=24576,
                process_seconds=PROCESS_SECONDS, cohort_seconds=COHORT_SECONDS, freeze_seconds=FREEZE_SECONDS,
                runtime_sources=dict(baseline=common.BASE, installed=common.CURRENT), qualification=common.QUAL,
                measurement_executed=False)


def reference():
    portable_history_guard()
    raw = (history.PACKET / 'measured/194-pair-0-baseline.stdout.log').read_text()
    return signature(cpu_review.parse(raw)[1])


def portable_history_guard():
    """Replay every historical guard using its sealed Windows inventory order."""
    packet = history.PACKET
    need(history.sha(packet / 'inventory.json') == history.INPUT_SHA, 'Pinned CPU inventory changed')
    for module, name in ((cpu_review, 'review.py'), (history.predecessor, 'seal.py')):
        need(Path(module.__file__).resolve() == (history.CPU_TOOLS / name).resolve(), 'Wrong historical reader imported')
    for name in ('review.py', 'seal.py', 'run.py', 'probe.rs'):
        saved = packet / 'source/method/tools/authority-runtime-cpu-074' / name
        need(history.same_checkout_source((history.CPU_TOOLS / name).read_bytes(), saved.read_bytes()),
             'Frozen CPU reader/method changed: ' + name)
    rows = json.loads((packet / 'inventory.json').read_text())['files']
    for row in rows: common.member(row['path'])
    actual = history.predecessor.inventory(packet)
    need(len({PureWindowsPath(row['path']) for row in actual}) == len(actual), 'Case-colliding historical members refused')
    need(rows == sorted(actual, key=lambda row: PureWindowsPath(row['path'])), 'CPU packet membership/bytes changed')
    value = cpu_review.facts(packet / 'measured')
    need(value == json.loads((packet / 'facts.json').read_text()), 'CPU facts changed')
    history.predecessor.verify_predecessor()
    need(history.sha(history.DOCS / 'qualification-manifest.toml') == value['raw_qualification'], 'Original qualification changed')
    return value


def signature(rows):
    keys = ('mode', 'depth', 'fault', 'seed', 'iterations', 'corpus', 'root_name_bytes')
    return [{key: row[key] for key in keys} for row in rows]


def parse(raw, expected):
    common.passed(raw, cpu.MEASURE)
    header, rows = cpu_review.parse(raw.decode())
    need(signature(rows) == expected, 'Workload/corpus differs from frozen first cohort')
    return header, rows


def validate_images(images):
    need(set(images) == set(COMPILERS), 'Both compilers required')
    paths = []; digests = []
    for lane, roles in images.items():
        need(set(roles) == set(ROLE.values()), 'Both runtime roles required')
        for image in roles.values():
            need(set(image) == {'path', 'sha256', 'compiler'} and image['compiler'] == COMPILERS[lane], 'Wrong compiler image')
            need(isinstance(image['path'], str) and image['path'].startswith('/') and
                 isinstance(image['sha256'], str) and len(image['sha256']) == 64 and
                 all(c in '0123456789abcdef' for c in image['sha256']), 'Invalid image identity')
            paths.append(image['path']); digests.append(image['sha256'])
    need(len(set(paths)) == len(set(digests)) == 4, 'Four distinct fresh images required')


def inputs():
    return {name: common.sha(ROOT / name) for name in INPUTS}


def block_comparison(baseline, installed):
    need(len(baseline) == len(installed) == 2 and all(type(v) in (int, float) and math.isfinite(v) and v >= 0
         for v in baseline + installed), 'Two finite observations per role required')
    ratio = (math.exp(statistics.mean(map(math.log, installed)) - statistics.mean(map(math.log, baseline)))
             if min(baseline + installed) > 0 else None)
    # Zero-valued counters retain their observations/difference, without log(0).
    return dict(baseline=baseline, installed=installed,
                difference_of_means=statistics.mean(installed) - statistics.mean(baseline), ratio=ratio)


def interval(ratios):
    need(len(ratios) == 8, 'Eight complete blocks required')
    if any(value is None for value in ratios):
        return dict(block_ratios=ratios, median=None, interval=None, nominal_coverage=None)
    need(all(type(v) in (int, float) and math.isfinite(v) and v > 0 for v in ratios), 'Invalid block ratio')
    return dict(block_ratios=ratios, median=statistics.median(ratios), interval=[min(ratios), max(ratios)],
                nominal_coverage=1 - 2 / 2 ** 8)


def beyond_boundary(block, side):
    """Compare the same two-by-two geometric ratio exactly; never promote a rounded tie."""
    left = block['baseline']; right = block['installed']
    block_comparison(left, right)
    need(side in ('benefit', 'cost'), 'Unknown practical boundary')
    if min(left + right) <= 0: return False
    squared = Fraction(right[0]) * Fraction(right[1]) / (Fraction(left[0]) * Fraction(left[1]))
    return squared < Fraction(95, 100) ** 2 if side == 'benefit' else squared > Fraction(105, 100) ** 2
