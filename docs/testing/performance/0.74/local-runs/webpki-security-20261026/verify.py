"""Offline paired-regression and exact lock-delta verification, no admission."""
import hashlib
import json
from pathlib import Path
import re
import tomllib

HERE = Path(__file__).resolve().parent


def check(load=None):
    load = load or (lambda name: (HERE/name).read_bytes())
    raw = load('index.json')
    if hashlib.sha256(raw).hexdigest() != 'edcd229bcf2afac2a6fc1e099e94777e9af2946140d8df2e3cda55b588164c80':
        raise ValueError('index differs')
    index = json.loads(raw)
    for row in index['members'].values():
        raw = load('objects/'+row['sha256'])
        if len(raw) != row['bytes'] or hashlib.sha256(raw).hexdigest() != row['sha256']:
            raise ValueError('object differs')
    def read(name):
        return load('objects/'+index['members'][name]['sha256'])
    if any(index[k] is not False for k in ('unrelated_lock_delta', 'policy_pass', 'full_package_pass',
            'production_original_delivery', 'server', 'release_admitted')):
        raise ValueError('packet promoted')
    old = tomllib.loads(read('baseline/Cargo.lock').decode())
    new = tomllib.loads(read('source/Cargo.lock').decode())
    before = next(p for p in old['package'] if p['name'] == 'rustls-webpki')
    after = next(p for p in new['package'] if p['name'] == 'rustls-webpki')
    if (before['version'], after['version']) != ('0.103.15', '0.103.16'):
        raise ValueError('version differs')
    before.update(version=after['version'], checksum=after['checksum'])
    if old != new:
        raise ValueError('unrelated lock delta')
    for name in ('Cargo.toml', 'deny.toml'):
        if read('baseline/'+name).replace(b'\r\n', b'\n') != read('source/'+name).replace(b'\r\n', b'\n'):
            raise ValueError('manifest or policy differs')
    source = read('source/crates/hydracache-redis-compat/tests/tls_spki_security_074.rs')
    intent = json.loads(read('checks/webpki-baseline/intent.json'))
    if intent['source']['crates/hydracache-redis-compat/tests/tls_spki_security_074.rs'] != hashlib.sha256(source).hexdigest():
        raise ValueError('paired oracle differs')
    embedded = re.findall(r'\("([A-Z0-9]+\.spki\.der)", "([0-9a-f]+)"\)', source.decode())
    vectors = json.loads(read('upstream/vectors.json'))['vectors']
    if len(embedded) != 8 or dict(embedded) != {row['name']: row['hex'] for row in vectors}:
        raise ValueError('public vectors differ')
    for row in vectors:
        if hashlib.sha256(bytes.fromhex(row['hex'])).hexdigest() != row['sha256']:
            raise ValueError('upstream vector hash differs')
    for phase in ('webpki-baseline', 'webpki-update', 'webpki-candidate', 'webpki-msrv',
            'webpki-regression', 'webpki-linux', 'webpki-policy', 'webpki-check', 'webpki-clippy',
            'webpki-format', 'webpki-resp-all', 'webpki-doc', 'webpki-contract'):
        result = json.loads(read('checks/'+phase+'/result.json'))
        expected = 101 if phase == 'webpki-baseline' else 1 if phase == 'webpki-policy' else 0
        if (result['exit'] != expected or result['outer_refusal'] is not None or
                result['source_contract'] is not True or result['commit_unchanged'] is not True):
            raise ValueError('focused verdict differs')
    totals = {'webpki-candidate': [5,0,0,0,0], 'webpki-msrv': [5,0,0,0,0],
        'webpki-linux': [5,0,0,0,0], 'webpki-regression': [20,0,0,0,0],
        'webpki-resp-all': [183,0,23,0,0]}
    if {name: row['totals'] for name, row in index['counts'].items()} != totals:
        raise ValueError('focused membership differs')
    return dict(members=len(index['members']), integrity=True, policy_pass=False, release_admitted=False)


if __name__ == '__main__':
    print(check())
