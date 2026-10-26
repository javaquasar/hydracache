"""Verify raw local check ledger, including both first verifier errors."""
import hashlib
import json
from pathlib import Path

INDEX = '64b1ada902211b722908a13f62a05cdaeb8e9d976aa0318670c05c705fddf8d1'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def verify(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['checks'] == 6 and index['new_workload_calls'] == 0 and all(index[k] is False for k in
            ('native_control_executed', 'overhead_accepted', 'package_pass')) and
            sha((folder.parent/'index.json').read_bytes()) == index['model_index_sha256'] ==
            'd539ff2fa93959cf1fbb176e4994ebb5d7f83c5e141a0e1bae81f672f0cb2000', 'scope/parent differs')
    members = {}
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'raw member differs')
        members[name] = body
    require(len(members) == 31, 'member count differs')
    for name in ('verify.py', 'test_verify.py'):
        require(members[name] == (folder.parent/name).read_bytes(), 'verifier source differs')
    require(sha(members['qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification changed')
    for label in ('integrity-first', 'integrity-corrected', 'integrity-closure', 'verifier-syntax', 'doc-first', 'contract-first'):
        stem = 'checks/capture-leaves-'+label+'/'
        a, b = (json.loads(members[stem+part]) for part in ('intent.json', 'result.json'))
        require(b['exit'] == (1 if label in ('integrity-first', 'integrity-corrected') else 0) and
                b['outer_refusal'] is None and b['source_unchanged'] is b['head_unchanged'] is True and
                a['source'] == b['source_after'] and a['head'] == '8a0a10cab2a87da519e882838b11645d702fb5e9',
                'receipt/source differs')
        if label.startswith('integrity-'):
            log = members[stem+'stderr.log']
            require(b'Ran 5 tests' in log, 'integrity count differs')
            require(log.rstrip().endswith(b'OK') if label == 'integrity-closure' else b'FAILED (errors=1)' in log,
                    'first/final integrity outcome differs')
    require(b'original dependency changed' in members['checks/capture-leaves-integrity-first/stderr.log'] and
            b'owner setter AST differs' in members['checks/capture-leaves-integrity-corrected/stderr.log'], 'first errors lost')
    require(b'doc-check: OK' in members['checks/capture-leaves-doc-first/stdout.log'] and
            b'OK (local, non-promotable)' in members['checks/capture-leaves-contract-first/stdout.log'], 'gate verdict differs')
    return dict(checks=6, preserved_verifier_errors=2, final_integrity_cases=5, raw_members=31,
                new_workload_calls=0, native_control_executed=False, package_pass=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
