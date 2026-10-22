"""Collect and independently check native validation receipts; executes no image."""
import json
from pathlib import Path
import shutil
import struct
import sys

ROOT = Path.cwd()
sys.path.insert(0, str(ROOT / 'tools/authority-runtime-balanced-074'))
import balanced as m
import evaluate as reader

attempt = ROOT / 'target/authority-runtime-native-second'
result = json.loads((attempt / 'result.json').read_text())
assert result['failure'] is None and result['phases'] == ['cleanup', 'seed-audit', 'build-only', 'postflight']
assert result['before'] == result['after'] and result['source'] == result['before']['head']
assert result['before']['qualification'] == m.common.QUAL
assert all(result[key] is False for key in ('measurement_executed', 'runtime_images_executed', 'server', 'product', 'clocks_changed', 'services_changed'))
m.validate_images(result['images']); m.portable_history_guard()
records = {}
for path in attempt.glob('*.json'):
    value = json.loads(path.read_text())
    if not isinstance(value, dict) or 'argv' not in value: continue
    assert value['name'] == path.stem and value['exit'] == 0 and value['timeout'] is False
    for suffix in ('stdout', 'stderr'):
        assert m.common.sha(attempt / (value['name'] + '.' + suffix + '.log')) == value[suffix + '_sha256']
    records[value['name']] = value
expected = {}
for lane in m.COMPILERS: expected.update(reader.build_commands(lane, result['owned_root']))
assert set(records) == set(expected) and len(records) == 14
for name, (argv, cwd) in expected.items():
    assert records[name]['argv'] == argv and (cwd is None or records[name]['cwd'] == cwd)
for lane in m.COMPILERS:
    before = json.loads((attempt / (lane + '-seed-before.json')).read_text())
    assert before == json.loads((attempt / (lane + '-seed-after.json')).read_text())
    for role in m.ROLE.values():
        name = lane + '-' + role
        seed = json.loads((attempt / (name + '-dependency-seed.json')).read_text())
        assert seed['origin'] == before['origin'] and seed['files'] == before['files'] and seed['package_artifacts_copied'] is False
        assert m.common.image((attempt / (name + '-build.stdout.log')).read_bytes()) == result['images'][lane][role]['path']
owned = Path('//wsl.localhost/Ubuntu' + result['owned_root'])
cleanup = owned / 'cleanup'
control = json.loads((cleanup / 'control.json').read_text())
command = json.loads((cleanup / '000-profile.json').read_text())
assert control['passed'] is True and control['pidfds_observed_exited'] == 2 and control['measurement_executed'] is False
assert command['timeout'] is True and command['exit'] == -9 and command['error'] is None
assert command['pid'] == control['ready']['leader']['pid'] and command['argv'][3] == result['owned_root'] + '/cleanup/synthetic-timeout'
for suffix in ('stdout', 'stderr'):
    assert m.common.sha(cleanup / ('000-profile.' + suffix + '.log')) == command[suffix + '_sha256']
assert m.common.sha(cleanup / 'synthetic-timeout') == result['input_hashes']['tools/authority-runtime-native-074/cleanup_child.py']
image_facts = []
for lane, roles in result['images'].items():
    for role, image in roles.items():
        path = Path('//wsl.localhost/Ubuntu' + image['path'])
        assert m.common.sha(path) == image['sha256']
        with path.open('rb') as stream: header = stream.read(20)
        assert header[:6] == b'\x7fELF\x02\x01' and struct.unpack('<H', header[18:20])[0] == 62
        image_facts.append(dict(compiler=lane, role=role, sha256=image['sha256'], elf64_x86_64=True, executed=False))
for name, digest in result['input_hashes'].items(): assert m.common.sha(ROOT / name) == digest
packet = ROOT / 'docs/testing/performance/0.74/local-runs/authority-runtime-native-validation-20261022'
assert not packet.exists()
packet.mkdir()
shutil.copytree(attempt, packet / 'native')
shutil.copytree(cleanup, packet / 'cleanup')
checks = packet / 'checks'; checks.mkdir()
for path in sorted((ROOT / 'target/authority-runtime-native-checks-20261022').glob('*.log')):
    shutil.copyfile(path, checks / path.name)
shutil.copyfile(__file__, checks / 'audit.py')
for name in result['input_hashes']:
    target = packet / 'source' / name; target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / name, target)
facts = dict(schema='authority-runtime-native-audit-074-v1', source=result['source'], successful_build_commands=14,
    images=image_facts, seed_members_per_compiler={lane: len(json.loads((attempt / (lane + '-seed-before.json')).read_text())['files']) for lane in m.COMPILERS},
    cleanup_pidfds_exited=2, native_runtime_images_executed=False, measurement_executed=False,
    unchanged_metadata=True, first_preflight_refusal='checks/native-first.log', portable_inventory_order='case-sensitive POSIX member strings')
m.common.publish(packet / 'audit.json', facts)
members = sorted(m.history.predecessor.inventory(packet), key=lambda row: row['path'])
m.common.publish(packet / 'inventory.json', dict(schema='authority-runtime-native-inventory-074-v1', files=members))
print(json.dumps(dict(members=len(members), inventory_sha256=m.common.sha(packet / 'inventory.json'), **facts), sort_keys=True))
