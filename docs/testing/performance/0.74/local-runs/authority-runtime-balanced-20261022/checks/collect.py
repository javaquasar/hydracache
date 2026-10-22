"""Retain the single completed v2 cohort; offline verification only, no image execution."""
import json
from pathlib import Path
import shutil
import statistics
import struct
import sys

ROOT = Path.cwd()
sys.path.insert(0, str(ROOT / 'tools/authority-runtime-balanced-074'))
import balanced as m
import evaluate as reader

attempt = ROOT / 'target/authority-runtime-balanced-first'
facts = reader.verify(attempt)
assert facts == json.loads((attempt / 'facts.json').read_text())
result = json.loads((attempt / 'result.json').read_text())
terminal = json.loads((attempt / 'cohort/result.json').read_text())
assert result['method'] == '9bfb155285f08b52b7c5eb7225b70a6828a13d41'
assert m.common.git('status', '--porcelain').strip() == b''
assert result['build_before'] == result['measure_before'] == result['measure_after']
assert json.loads((attempt / 'method.json').read_text()) == dict(source=result['method'], schedule_sha256=m.SCHEDULE_SHA)
images = []
seeds = {}
for lane, roles in result['images'].items():
    for role, image in roles.items():
        path = Path(image['path'])
        assert not path.is_symlink() and m.common.sha(path) == image['sha256']
        with path.open('rb') as stream: header = stream.read(20)
        assert header[:6] == b'\x7fELF\x02\x01' and struct.unpack('<H', header[18:20])[0] == 62
        images.append(dict(compiler=lane, role=role, sha256=image['sha256'], elf64_x86_64=True))
        seed = json.loads((attempt / (lane + '-' + role + '-dependency-seed.json')).read_text())
        assert seed['package_artifacts_copied'] is False
        assert len(seed['files']) == 1869
        origin = Path(seed['origin'])
        for row in seed['files']:
            m.common.member(row['path'])
            assert not (origin / row['path']).is_symlink()
            assert set(row) == {'path', 'sha256'}
            assert m.common.sha(origin / row['path']) == row['sha256']
        normalized = {key: value for key, value in seed.items() if key != 'target'}
        if lane in seeds: assert seeds[lane] == normalized
        seeds[lane] = normalized
for name, digest in result['tool_hashes'].items(): assert m.common.sha(ROOT / name) == digest

support = {}
calibration = {}
for lane, cells in facts['lanes'].items():
    support[lane] = []
    for mode, metric in (('phases', 'open_cpu_ns'), ('phases', 'publication_cpu_ns'),
            ('phases', 'publication_wall_ns'), ('phases', 'refusal_cpu_ns'),
            ('phases', 'drop_cpu_ns'), ('cycle', 'cycle_cpu_ns'),
            ('phases', 'warm_cpu_ns_per_inspect'), ('cycle', 'cycle_wall_ns')):
        value = next(cell for cell in cells if (cell['mode'], cell['depth'], cell['fault']) == (mode, 127, 'external'))['metrics'][metric]
        support[lane].append(dict(metric=metric, baseline_marginal_median=statistics.median(v for block in value['blocks'] for v in block['baseline']),
            installed_marginal_median=statistics.median(v for block in value['blocks'] for v in block['installed']),
            block_median=value['median'], block_range=value['interval']))
    headers = [json.loads(path.read_text())['clock_metadata'] for path in sorted((attempt / 'cohort').glob('*-step.json'))
        if json.loads(path.read_text())['selection']['compiler'] == lane]
    calibration[lane] = dict(brackets=len(headers) * 8,
        resolution_pairs=sorted(set((h['wall_resolution_ns'], h['cpu_resolution_ns']) for h in headers)),
        ranges={key: [min(v[key] for h in headers for v in h['empty']), max(v[key] for h in headers for v in h['empty'])]
            for key in ('cpu_ns', 'wall_ns')})

packet = ROOT / 'docs/testing/performance/0.74/local-runs/authority-runtime-balanced-20261022'
assert not packet.exists()
packet.mkdir()
shutil.copytree(attempt, packet / 'native')
checks = packet / 'checks'; checks.mkdir()
shutil.copyfile(__file__, checks / 'collect.py')
shutil.copyfile(ROOT / 'target/authority-runtime-balanced-collect-first.py', checks / 'collect-first.py')
shutil.copyfile(ROOT / 'target/authority-runtime-balanced-collect-first-refusal.log', checks / 'collect-first-refusal.log')
shutil.copyfile(ROOT / 'target/authority-runtime-balanced-first-freeze-refusal.log', checks / 'operator-freeze-first-refusal.log')
summary = dict(schema='authority-runtime-balanced-audit-074-v1', method=result['method'],
    runtime_sources=result['runtime_sources'], qualification=result['build_before']['qualification'],
    boot=result['build_before']['boot'], cpu=result['cpu'], images=images,
    successful_preliminary_commands=22, clock_semantic_controls=8, measured_processes=64, rows=768,
    timed_warm_inspections=24576, cohort_seconds=terminal['finished_monotonic'] - terminal['started_monotonic'],
    seed_members_per_compiler={lane: len(v['files']) for lane, v in seeds.items()},
    support=support, calibration=calibration, primary_dispositions={key: v['disposition'] for key, v in facts['primary'].items()},
    server=False, product=False, live=False, clocks_changed=False, services_changed=False,
    first_operator_refusal='checks/operator-freeze-first-refusal.log', control_retries=0, measured_retries=0,
    inventory_order='case-sensitive POSIX member string')
m.common.publish(packet / 'audit.json', summary)
assert reader.verify(packet / 'native') == facts
members = sorted(m.history.predecessor.inventory(packet), key=lambda row: row['path'])
m.common.publish(packet / 'inventory.json', dict(schema='authority-runtime-balanced-inventory-074-v1', files=members))
print(json.dumps(dict(members=len(members), inventory_sha256=m.common.sha(packet / 'inventory.json'), **summary), sort_keys=True))
