"""Offline validation of a retained, NOT admitted public trust-catalog candidate."""
import copy
import hashlib
import json
from pathlib import Path
import re
import tomllib

HERE = Path(__file__).resolve().parent
INDEX_SHA256 = '5a66272e834fd917f87f7063f1415d1e50beba5b18f776104a7b8a19c4a6f73d'


def validate(index, read):
    """Check semantic boundaries after the caller verifies the sealed bytes."""
    for field in ('candidate_accepted', 'trust_scope_approved', 'full_package_pass',
                  'original_checks_or_deadlines_changed', 'original_execution_proved',
                  'policy_pass', 'release_admitted', 'runtime_source_changed', 'server'):
        if index[field] is not False:
            raise ValueError('packet promoted: '+field)
    for field in ('same_corrected_oracle', 'preserved_initial_oracle_refusals',
                  'initial_root_certs_1011_selected_but_not_tested'):
        if index[field] is not True:
            raise ValueError('first result boundary differs')
    old = tomllib.loads(read('original/Cargo.lock').decode())
    baseline = tomllib.loads(read('baseline/Cargo.lock').decode())
    candidate = tomllib.loads(read('source/Cargo.lock').decode())
    without_edges = copy.deepcopy(baseline)
    package = next(p for p in without_edges['package'] if p['name'] == 'hydracache-redis-compat')
    for edge in ('webpki-root-certs', 'webpki-roots 0.26.11', 'webpki-roots 1.0.8'):
        package['dependencies'].remove(edge)
    if without_edges != old or index['test_only_edges'] != 3:
        raise ValueError('baseline lock differs')
    for name in ('webpki-root-certs', 'webpki-roots'):
        before = next(p for p in baseline['package'] if p['name'] == name and p['version'] == '1.0.8')
        after = next(p for p in candidate['package'] if p['name'] == name and p['version'] == '1.0.10')
        before.update(version=after['version'], checksum=after['checksum'])
    # The only qualified parent references are in the legacy shim and test node.
    for package in baseline['package']:
        if 'webpki-roots 1.0.8' in package.get('dependencies', []):
            if package['name'] not in ('webpki-roots', 'hydracache-redis-compat'):
                raise ValueError('unexpected parent reference')
            package['dependencies'] = [
                'webpki-roots 1.0.10' if v == 'webpki-roots 1.0.8' else v
                for v in package['dependencies']]
    if baseline != candidate:
        raise ValueError('unrelated lock delta')
    manifest = 'crates/hydracache-redis-compat/Cargo.toml'
    source_manifest = tomllib.loads(read('source/'+manifest).decode())
    for name in ('webpki-root-certs', 'webpki-roots'):
        if source_manifest['dev-dependencies'].pop(name) != '1.0.10':
            raise ValueError('test floor differs')
    if source_manifest['dev-dependencies'].pop('webpki-roots-legacy') != {
            'package': 'webpki-roots', 'version': '=0.26.11'}:
        raise ValueError('legacy shim differs')
    if source_manifest != tomllib.loads(read('original/'+manifest).decode()):
        raise ValueError('runtime manifest differs')
    test = 'crates/hydracache-redis-compat/tests/tls_roots_security_074.rs'
    source = read('source/'+test)
    if source != read('baseline/'+test):
        raise ValueError('paired oracle differs')
    intent = json.loads(read('checks/roots-baseline-policy-corrected/intent.json'))
    if intent['source'][test] != hashlib.sha256(source).hexdigest():
        raise ValueError('baseline oracle receipt differs')
    for name in ('Cargo.toml', 'deny.toml', 'docs/testing/performance/0.74/qualification-manifest.toml'):
        if read('baseline/'+name) != read('source/'+name):
            raise ValueError('protected input differs')
    qualification = read('source/docs/testing/performance/0.74/qualification-manifest.toml')
    if hashlib.sha256(qualification).hexdigest() != '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc':
        raise ValueError('qualification differs')
    phases = {name.split('/')[1] for name in index['members'] if name.startswith('checks/')}
    if len(phases) != 20:
        raise ValueError('phase membership differs')
    failures = {'roots-baseline': 101, 'roots-baseline-order-corrected': 101,
                'roots-baseline-policy-corrected': 101, 'roots-policy': 1,
                'roots-catalog-delta': 1, 'roots-catalog-delta-observed': 1}
    counts = {}
    for phase in phases:
        result = json.loads(read('checks/'+phase+'/result.json'))
        if (result['exit'] != failures.get(phase, 0) or result['outer_refusal'] is not None
                or result['source_contract'] is not True or result['commit_unchanged'] is not True):
            raise ValueError('first phase verdict differs: '+phase)
        log = read('checks/'+phase+'/stdout.log').decode()
        rows = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', log)
        if rows:
            counts[phase] = dict(targets=len(rows), totals=[sum(int(r[i]) for r in rows) for i in range(5)])
    if counts != index['counts']:
        raise ValueError('test counts differ')
    expected = {'roots-candidate': (3, 10, 0), 'roots-msrv': (3, 10, 0),
                'roots-linux': (3, 10, 0), 'roots-server': (2, 15, 0),
                'roots-resp-all': (17, 188, 23)}
    for phase, (targets, passed, ignored) in expected.items():
        if counts[phase] != dict(targets=targets, totals=[passed, 0, ignored, 0, 0]):
            raise ValueError('focused cohort differs')
    if b'3 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out' not in read('checks/roots-baseline-policy-corrected/stdout.log'):
        raise ValueError('corrected baseline differs')
    inventory = json.loads(read('coordinator/roots-trust-inventory-20261026.json'))
    if ((inventory['old_anchors'], inventory['new_anchors'], inventory['old_certs'],
         inventory['new_certs'], inventory['unchanged_complete_anchors'], inventory['new_authorities'])
            != (118, 120, 118, 120, 116, 4) or len(inventory['removed']) != 2
            or len(inventory['added']) != 4 or inventory['trust_scope_approved'] is not False
            or inventory['release_admitted'] is not False):
        raise ValueError('unreviewed trust delta differs')
    for row in inventory['removed']+inventory['added']:
        contents = bytes.fromhex(row['spki_contents_hex'])
        size = len(contents)
        length = bytes([size]) if size < 128 else bytes([0x80+(size.bit_length()+7)//8])+size.to_bytes((size.bit_length()+7)//8, 'big')
        if hashlib.sha256(b'\x30'+length+contents).hexdigest() != row['spki_sha256']:
            raise ValueError('public SPKI hash differs')
    ids = sorted(set(re.findall(r'ID: (RUSTSEC-\d{4}-\d{4})', read('checks/roots-policy/stderr.log').decode())))
    if ids != index['remaining_advisories'] or len(ids) != 8:
        raise ValueError('candidate policy differs')
    return dict(members=len(index['members']), integrity=True, candidate_accepted=False,
                trust_scope_approved=False, policy_pass=False, release_admitted=False)


def check(load=None):
    load = load or (lambda name: (HERE/name).read_bytes())
    raw = load('index.json')
    if hashlib.sha256(raw).hexdigest() != INDEX_SHA256:
        raise ValueError('index differs')
    index = json.loads(raw)
    for row in index['members'].values():
        raw = load('objects/'+row['sha256'])
        if len(raw) != row['bytes'] or hashlib.sha256(raw).hexdigest() != row['sha256']:
            raise ValueError('object differs')
    return validate(index, lambda name: load('objects/'+index['members'][name]['sha256']))


if __name__ == '__main__':
    print(check())
