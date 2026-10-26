"""Verify final receipts offline without invoking their recorded commands."""
import hashlib
import json
from pathlib import Path

FOLDER = Path(__file__).resolve().parent
sha = lambda raw: hashlib.sha256(raw).hexdigest()
raw = (FOLDER/'index.json').read_bytes()
assert sha(raw) == '6820a56fe3795a3eda3092b59cca9ca62f4483b51961ca5ff51cfdad88038209'
index = json.loads(raw)
assert sha((FOLDER.parent/'index.json').read_bytes()) == index['native_index_sha256']
assert all(index[key] is False for key in ('package_pass', 'qualification', 'server'))
members = {}
for name, row in index['members'].items():
    body = (FOLDER/'objects'/row['sha256']).read_bytes()
    assert len(body) == row['bytes'] and sha(body) == row['sha256'], name
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
print(dict(final_checks=4, raw_members=20, package_pass=False, workload_replayed=False))
