"""Retain only the exact final check receipts; no bus or image execution."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

root = Path.cwd(); sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = root / 'target/held-manager-compat-final-checks-20261022'
packet = root / 'docs/testing/performance/0.74/local-runs/held-manager-compat-doc-followup-20261022'
assert not packet.exists()
for name in ('format', 'docs', 'registry', 'performance'):
    row = json.loads((source / (name + '.json')).read_text())
    assert row['exit'] == 0
    for suffix in ('stdout', 'stderr'):
        assert sha(source / (name + '.' + suffix + '.log')) == row[suffix + '_sha256']
assert '186 discovered gates, all registered' in (source / 'registry.stdout.log').read_text()
assert 'OK (local, non-promotable)' in (source / 'performance.stdout.log').read_text()
assert sha(root / 'docs/testing/performance/0.74/qualification-manifest.toml') == '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
assert not subprocess.check_output(['git', 'diff', 'c237f580', '--name-only', '--', 'tools/long-run-supervisor-074', 'docs/plans/releases.toml'])
packet.mkdir()
for name in ('format', 'docs', 'registry', 'performance'):
    for suffix in ('.json', '.stdout.log', '.stderr.log'):
        shutil.copyfile(source / (name + suffix), packet / (name + suffix))
shutil.copyfile(__file__, packet / 'retain-docs.py')
shutil.copyfile(root / 'target/held-manager-compat-final-checks.py', packet / 'check-driver.py')
documents = ['docs/testing/performance/0.74/held-manager-compat-20261022.md', 'docs/testing/performance/0.74/README.md',
    'docs/testing/performance/0.74/diagnostic-server-readiness-intermediate-plan-20261012.md',
    'docs/articles/007-optimization-starts-with-measurement.md', 'tools/held-manager-compat-074/README.md']
with (packet / 'context.json').open('x') as stream:
    json.dump(dict(evidence_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD']).decode().strip(),
        evidence_inventory_sha256='de30658692b85efa826e5af7af320abdda3768fe7f22453f1bb9f2efd9f6c4b8',
        checkout_documents_sha256={name: sha(root / name) for name in documents},
        original_delivery=False, writers_cessation=False, server=False, new_native_calls=False), stream, sort_keys=True, indent=2)
rows = []
for path in sorted(packet.rglob('*'), key=lambda p: p.relative_to(packet).as_posix()):
    assert not path.is_symlink()
    if path.is_file(): rows.append(dict(path=path.relative_to(packet).as_posix(), bytes=path.stat().st_size, sha256=sha(path)))
with (packet / 'inventory.json').open('x') as stream:
    json.dump(dict(schema='held-manager-compat-doc-inventory-074-v1', files=rows), stream, sort_keys=True, indent=2)
print(json.dumps(dict(members=len(rows), inventory_sha256=sha(packet/'inventory.json')), sort_keys=True))
