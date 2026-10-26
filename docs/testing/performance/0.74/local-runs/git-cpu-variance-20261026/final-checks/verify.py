"""Final receipt integrity only; never executes their recorded commands."""
import hashlib
import json
from pathlib import Path

FOLDER = Path(__file__).resolve().parent
sha = lambda raw: hashlib.sha256(raw).hexdigest()
raw = (FOLDER/'index.json').read_bytes()
assert sha(raw) == '5418f40dc1e2c469b61cbc1a4e8b67b83d559ef6bb6cb67123c81d5ab7276c8a'
index = json.loads(raw)
assert index['analysis_index_sha256'] == sha((FOLDER.parent/'index.json').read_bytes())
assert all(index[key] is False for key in ('workload_replayed', 'package_pass', 'qualification', 'server'))
members = {}
for name, entry in index['members'].items():
    body = (FOLDER/'objects'/entry['sha256']).read_bytes()
    assert len(body) == entry['bytes'] and sha(body) == entry['sha256'], name
    members[name] = body
assert len(members) == 20
for name in ('doc', 'contract', 'syntax', 'integrity'):
    before, after = (json.loads(members[name+'/'+part]) for part in ('intent.json', 'result.json'))
    assert after['exit'] == 0 and after['outer_refusal'] is None
    assert after['source_unchanged'] is True and after['head_unchanged'] is True
    assert before['source'] == after['source_after'] and before['bound_seconds'] == 1200
    assert all(before[key] is False for key in ('server', 'services_or_clocks', 'qualification', 'retry'))
assert b'doc-check: OK' in members['doc/stdout.log']
assert b'OK (local, non-promotable)' in members['contract/stdout.log']
assert b'Ran 4 tests' in members['integrity/stderr.log'] and members['integrity/stderr.log'].rstrip().endswith(b'OK')
for name in ('verify.py', 'test_verify.py'):
    assert members[name] == (FOLDER.parent/name).read_bytes()
assert sha(members['qualification-manifest.toml']) == '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
print(dict(final_checks=4, raw_members=20, workload_replayed=False, package_pass=False))
