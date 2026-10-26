"""Check retained final receipts only, with no collector execution."""
import hashlib
import json
from pathlib import Path

INDEX = 'd5bd23a54f15415f2cc1ea5f3ca39d71e24fe2e9691e0e6f4f7c81d5f71f9e8b'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def verify(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['checks'] == 7 and index['new_native_attempts'] == 0 and
            index['overhead_accepted'] is index['package_pass'] is False and
            sha((folder.parent/'index.json').read_bytes()) == index['native_index_sha256'] ==
            '55b802621e147681f9478a164053a04c055ec28d8d35227d68d49ad0b4de380f', 'scope/parent differs')
    members = {}
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'member differs: '+name)
        members[name] = body
    require(len(members) == 32, 'member count differs')
    for name in ('verify.py', 'test_verify.py'):
        require(members[name] == (folder.parent/name).read_bytes(), 'verifier source differs')
    require(sha(members['qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification changed')
    for name in members:
        if not name.startswith('checks/') or not name.endswith('/intent.json'):
            continue
        stem = name.removesuffix('intent.json')
        a, b = (json.loads(members[stem+part]) for part in ('intent.json', 'result.json'))
        require(b['exit'] == 0 and b['outer_refusal'] is None and b['source_unchanged'] is b['head_unchanged'] is True and
                a['source'] == b['source_after'] and a['head'] == '1030acb263e96d1e023f852dceb021de458ce6f4',
                'receipt/source differs')
        require(a['bound_seconds'] == 1200 and a['disk_floor_bytes'] == 4*1024**3 and all(a[k] is False for k in
                ('dependency_changes', 'observation_frequency_changed', 'services_or_clocks', 'server',
                 'qualification', 'retry')), 'check scope differs')
    for label, count in (('integrity', 6), ('models', 56)):
        log = members[f'checks/coordinator-native-{label}-first/stderr.log']
        require(f'Ran {count} tests'.encode() in log and log.rstrip().endswith(b'OK'), 'test verdict differs')
    require(b'doc-check: OK' in members['checks/coordinator-native-doc-first/stdout.log'] and
            b'OK (local, non-promotable)' in members['checks/coordinator-native-contract-first/stdout.log'], 'gate verdict differs')
    return dict(checks=7, raw_members=32, new_native_attempts=0, overhead_accepted=False, package_pass=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
