"""Retain post-measurement offline checks and document hashes, without executing images."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

root = Path.cwd()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
checks = root / 'target/authority-runtime-balanced-checks-20261022'
names = ('balanced-offline', 'cpu-offline', 'variance-offline', 'native-offline',
    'runtime-review-offline', 'format', 'docs', 'registry', 'performance')
counts = dict(zip(names[:5], (21, 7, 11, 4, 4)))
for name in names:
    value = json.loads((checks / (name + '.json')).read_text())
    assert value['exit'] == 0
    for suffix in ('stdout', 'stderr'):
        assert sha(checks / (name + '.' + suffix + '.log')) == value[suffix + '_sha256']
    if name in counts:
        raw = (checks / (name + '.stderr.log')).read_text()
        assert 'Ran ' + str(counts[name]) + ' tests in ' in raw and raw.rstrip().endswith('OK')
assert 'doc-check: OK' in (checks / 'docs.stdout.log').read_text()
assert '186 discovered gates, all registered' in (checks / 'registry.stdout.log').read_text()
assert 'OK (local, non-promotable)' in (checks / 'performance.stdout.log').read_text()
assert sha(root / 'docs/testing/performance/0.74/qualification-manifest.toml') == '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
assert not subprocess.check_output(['git', 'diff', '--name-only', 'HEAD', '--', 'tools/long-run-supervisor-074'])
packet = root / 'docs/testing/performance/0.74/local-runs/authority-runtime-balanced-doc-followup-20261022'
assert not packet.exists()
packet.mkdir()
for name in names:
    for suffix in ('.json', '.stdout.log', '.stderr.log'):
        shutil.copyfile(checks / (name + suffix), packet / (name + suffix))
shutil.copyfile(checks / 'staged-evidence.json', packet / 'staged-evidence.json')
first = root / 'target/authority-runtime-balanced-doc-followup-first-20261022'
assert sha(first / 'inventory.json') == '95bc07e8c95d1582df8cb1f485ed0b22f8298a34ded5ea6e37b92b71c448d490'
shutil.copyfile(first / 'inventory.json', packet / 'first-collection-inventory.json')
shutil.copyfile(root / 'target/authority-runtime-balanced-retain-docs-first.py', packet / 'retain-docs-first.py')
for name in ('offline-checks', 'doc-checks', 'retain-docs'):
    shutil.copyfile(root / ('target/authority-runtime-balanced-' + name + '.py'), packet / (name + '.py'))
documents = ['docs/testing/performance/0.74/authority-runtime-balanced-20261022.md',
    'docs/testing/performance/0.74/README.md',
    'docs/testing/performance/0.74/diagnostic-server-readiness-intermediate-plan-20261012.md',
    'docs/articles/007-optimization-starts-with-measurement.md', 'tools/authority-runtime-balanced-074/README.md']
context = dict(evidence_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD']).decode().strip(),
    measurement_method='9bfb155285f08b52b7c5eb7225b70a6828a13d41',
    cohort_inventory_sha256='d2b3a73324efd7171736c7e2a389550b479baf2d3883f14ee5866051598ee999',
    offline_controls=counts, document_sha256={name: sha(root / name) for name in documents},
    first_documentation_collection=dict(inventory_sha256=sha(first / 'inventory.json'),
        retained_at=str(first), correction='Select exact nine current command receipts/logs, not fifteen unrelated pre-existing development logs. First collection remains untouched; no command or measurement rerun.'),
    new_measurements=False, server=False, clocks_changed=False, services_changed=False)
with (packet / 'context.json').open('x') as stream: json.dump(context, stream, sort_keys=True, indent=2)
rows = []
for path in sorted(packet.rglob('*'), key=lambda p:p.relative_to(packet).as_posix()):
    assert not path.is_symlink()
    if path.is_file(): rows.append(dict(path=path.relative_to(packet).as_posix(), bytes=path.stat().st_size, sha256=sha(path)))
with (packet / 'inventory.json').open('x') as stream:
    json.dump(dict(schema='authority-runtime-balanced-doc-inventory-074-v1', files=rows), stream, sort_keys=True, indent=2)
print(json.dumps(dict(members=len(rows), inventory_sha256=sha(packet/'inventory.json'), offline_controls=sum(counts.values())), sort_keys=True))
