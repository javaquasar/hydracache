"""Independent offline integrity, dependency delta and focused-verdict checks."""
import hashlib
import json
from pathlib import Path
import tomllib

HERE = Path(__file__).resolve().parent


def check(load=None):
    load = load or (lambda name: (HERE/name).read_bytes())
    raw = load('index.json')
    if hashlib.sha256(raw).hexdigest() != '5406e2d000d98e77b0d40e0bfa0772f8b54c0955454af4e12a8821136ba81b14':
        raise ValueError('index differs')
    index = json.loads(raw)
    for item in index['members'].values():
        raw = load('objects/'+item['sha256'])
        if len(raw) != item['bytes'] or hashlib.sha256(raw).hexdigest() != item['sha256']:
            raise ValueError('object differs')
    def read(name):
        return load('objects/'+index['members'][name]['sha256'])
    for k in ('unrelated_lock_delta', 'full_policy_pass', 'full_package_pass',
              'full_verify_pass', 'server_admitted', 'product_performance_claim', 'qualification_allowed'):
        if index[k] is not False:
            raise ValueError('packet promoted')
    old = tomllib.loads(read('baseline/Cargo.lock').decode())
    new = tomllib.loads(read('source/Cargo.lock').decode())
    original = next(p for p in old['package'] if p['name'] == 'rustls')
    fixed = next(p for p in new['package'] if p['name'] == 'rustls')
    if (original['version'], fixed['version']) != ('0.23.45', '0.23.46'):
        raise ValueError('version differs')
    original.update(version=fixed['version'], checksum=fixed['checksum'])
    if old != new:
        raise ValueError('unrelated lock delta')
    before = read('baseline/Cargo.toml').replace(b'\r\n', b'\n')
    after = read('source/Cargo.toml').replace(b'\r\n', b'\n')
    if before.replace(b'rustls = "0.23.41"', b'rustls = "0.23.46"') != after:
        raise ValueError('manifest delta differs')
    if tomllib.loads(after.decode())['workspace']['package']['rust-version'] != '1.88':
        raise ValueError('MSRV differs')
    if read('baseline/deny.toml').replace(b'\r\n', b'\n') != read('source/deny.toml').replace(b'\r\n', b'\n'):
        raise ValueError('policy differs')
    for phase in ('rustls-update', 'rustls-candidate', 'rustls-msrv', 'rustls-resp-regression',
                  'rustls-linux-gnu194', 'rustls-policy', 'rustls-server-mtls', 'rustls-strict-check',
                  'rustls-strict-clippy', 'rustls-format', 'rustls-doc', 'rustls-contract'):
        result = json.loads(read('checks/'+phase+'/result.json'))
        if (result['exit'] != (1 if phase == 'rustls-policy' else 0)
                or result['outer_refusal'] is not None or result['source_contract'] is not True
                or result['commit_unchanged'] is not True):
            raise ValueError('verdict differs')
    expected = {'rustls-candidate': [3,0,0,0,0], 'rustls-msrv': [3,0,0,0,0],
        'rustls-linux-gnu194': [3,0,0,0,0], 'rustls-resp-regression': [181,0,23,0,0],
        'rustls-server-mtls': [15,0,0,0,0]}
    if {name: row['totals'] for name, row in index['counts'].items()} != expected:
        raise ValueError('focused membership differs')
    return dict(members=len(index['members']), integrity=True, policy_pass=False, release_admitted=False)


if __name__ == '__main__':
    print(check())
