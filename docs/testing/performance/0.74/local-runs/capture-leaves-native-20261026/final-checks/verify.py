"""Replay final local receipts only; no native collector or analyzer execution."""
import hashlib
import json
from pathlib import Path

INDEX = '7ec1c4af34c9bfe99890d6febbe7d54ad9e2ec704bedd5c44f35cd40b19d8cab'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def verify(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['checks'] == 8 and index['new_native_attempts'] == index['new_workload_calls'] == 0 and
            index['overhead_accepted'] is index['package_pass'] is False and
            sha((folder.parent/'index.json').read_bytes()) == index['native_index_sha256'] ==
            '926f30f0eca6ae708b3706976f256a9ca52a3a5d7e9a9c447a8f45cec59a0855', 'scope/parent differs')
    members = {}
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'member differs: '+name)
        members[name] = body
    require(len(members) == 37, 'member count differs')
    for name in ('verify.py', 'test_verify.py'):
        require(members[name] == (folder.parent/name).read_bytes(), 'verifier source differs')
    require(sha(members['qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification changed')
    for label in ('integrity-first', 'integrity-final', 'models', 'syntax', 'check', 'clippy', 'doc', 'contract'):
        stem = 'checks/capture-leaves-native-'+label+'/'
        a, b = (json.loads(members[stem+part]) for part in ('intent.json', 'result.json'))
        require(b['exit'] == 0 and b['outer_refusal'] is None and b['source_unchanged'] is b['head_unchanged'] is True and
                a['source'] == b['source_after'] and a['head'] == 'ef3d0cadd70e436b75be94e579a605447aaf7e2b' and
                a['bound_seconds'] == 1200 and a['disk_floor_bytes'] == 4*1024**3 and
                all(a[k] is False for k in ('dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'qualification', 'retry')), 'check verdict/source differs')
    for label, count in (('integrity-first', 6), ('integrity-final', 6), ('models', 74)):
        log = members['checks/capture-leaves-native-'+label+'/stderr.log']
        require(f'Ran {count} tests'.encode() in log and log.rstrip().endswith(b'OK'), 'test verdict differs')
    require(b'doc-check: OK' in members['checks/capture-leaves-native-doc/stdout.log'] and
            b'OK (local, non-promotable)' in members['checks/capture-leaves-native-contract/stdout.log'], 'gate verdict differs')
    return dict(checks=8, raw_members=37, new_native_attempts=0, new_workload_calls=0,
                overhead_accepted=False, package_pass=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
