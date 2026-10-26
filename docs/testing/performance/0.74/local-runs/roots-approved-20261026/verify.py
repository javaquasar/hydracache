"""Offline approval/evidence checks; never execute archived sources or commands."""
import copy
import hashlib
import json
from pathlib import Path
import re
import tomllib

HERE = Path(__file__).resolve().parent
INDEX_SHA = '284eb6ced97d68fe7ff7711c2611a723265fe4a08a2894de293ed4a14f6018d9'


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def check(load=None):
    load = load or (lambda name: (HERE / name).read_bytes())
    raw = load('index.json')
    require(sha(raw) == INDEX_SHA, 'index differs')
    index = json.loads(raw)
    require(len(index['members']) == 144 and len(index['phases']) == 20
            and index['approved_exact_public_catalog'] is True, 'approval scope differs')
    for key in ('old_packet_promoted', 'original_checks_or_deadlines_changed', 'runtime_source_changed',
                'production_enrollment_changed', 'tenant_authorization_changed', 'full_package_pass',
                'policy_pass', 'server', 'release_admitted'):
        require(index[key] is False, 'packet scope promoted')
    for row in index['members'].values():
        require(re.fullmatch('[a-f0-9]{64}', row['sha256']), 'object pointer')
        body = load('objects/' + row['sha256'])
        require(len(body) == row['bytes'] and sha(body) == row['sha256'], 'object differs')

    def read(name):
        return load('objects/' + index['members'][name]['sha256'])

    def data(name):
        return json.loads(read(name))

    old = tomllib.loads(read('baseline/Cargo.lock').decode())
    current = tomllib.loads(read('source/Cargo.lock').decode())
    for name, checksum in (('webpki-root-certs', '58ec80e73fd8baf57bf0d9615554c75348bda990d6d5bbeab964dd068319fecb'),
                           ('webpki-roots', '1c1c8e20c8a38fc7d65abfe74eb7280e4aa667e8eec8bc84996d2874810748d9')):
        row = next(p for p in old['package'] if p['name'] == name and p['version'] == '1.0.8')
        row.update(version='1.0.10', checksum=checksum)
    for row in old['package']:
        if 'dependencies' in row:
            row['dependencies'] = ['webpki-roots 1.0.10' if edge == 'webpki-roots 1.0.8' else edge
                                   for edge in row['dependencies']]
        if row['name'] == 'hydracache-redis-compat':
            row['dependencies'] = sorted(row['dependencies'] +
                ['webpki-root-certs', 'webpki-roots 0.26.11', 'webpki-roots 1.0.10'])
    require(old == current, 'unrelated graph change')
    manifest = 'crates/hydracache-redis-compat/Cargo.toml'
    old = tomllib.loads(read('baseline/' + manifest).decode())
    current = tomllib.loads(read('source/' + manifest).decode())
    require(current['dev-dependencies'].pop('webpki-roots') == '=1.0.10'
            and current['dev-dependencies'].pop('webpki-root-certs') == '=1.0.10'
            and current['dev-dependencies'].pop('webpki-roots-legacy') == dict(package='webpki-roots', version='=0.26.11')
            and current == old, 'manifest scope differs')
    for name in ('Cargo.toml', 'deny.toml', 'docs/testing/performance/0.74/qualification-manifest.toml'):
        require(read('baseline/' + name) == read('source/' + name), 'protected input differs')
    require(sha(read('source/crates/hydracache-redis-compat/tests/tls_roots_security_074.rs')) ==
            '655728cfa43e04ef1e26eabbfba8b382d5ddab8ce6232c1ef8d3603327c83f73', 'paired oracle differs')
    policy = data('implementation/approved-catalog.json')
    require(policy['versions'] == {'webpki-roots': '1.0.10', 'webpki-root-certs': '1.0.10'}, 'future catalog approval')
    inventory = data('historical/coordinator/roots-trust-inventory-20261026.json')
    require(inventory['trust_scope_approved'] is False, 'historical refusal promoted')
    actual = data('checks/trust-audit-first/stdout.log')
    for label in ('added', 'removed'):
        expected = sorted([{k: row[k] for k in ('spki_sha256', 'certificate_sha256', 'name_constraints_hex')}
                           for row in inventory[label]], key=lambda r: r['spki_sha256'])
        approved = sorted([{k: row[k] for k in ('spki_sha256', 'certificate_sha256', 'name_constraints_hex')}
                           for row in policy[label]], key=lambda r: r['spki_sha256'])
        require(expected == approved == sorted(actual[label], key=lambda r: r['spki_sha256']), 'key or constraints differ')
    require(actual['unchanged_complete_anchors'] == 116 and actual['exact_der_mapping'] is True
            and actual['new_roots_critical_ca_true'] is True and actual['new_name_constraints_absent'] is True
            and actual['exact_published_crates_verified'] is True, 'audit incomplete')
    for phase in index['phases']:
        result = data('checks/' + phase + '/result.json')
        require(result['exit'] == (1 if phase in ('dependency-policy-first', 'v7-pin-refusal-first') else 0)
                and result['outer_refusal'] is None and result['source_contract'] is True
                and result['commit_unchanged'] is True and result['source_unchanged'] == (phase != 'resolve-approved'),
                'phase outcome differs')
    for phase, targets, passed, ignored in (('focused-windows-first', 3, 10, 0), ('msrv-first', 3, 10, 0),
            ('gnu-first', 3, 10, 0), ('server-first', 2, 15, 0), ('resp-all-first', 17, 188, 23)):
        rows = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',
                          read('checks/' + phase + '/stdout.log').decode())
        require(len(rows) == targets and [sum(int(row[n]) for row in rows) for n in range(5)] ==
                [passed, 0, ignored, 0, 0], 'focused coverage differs')
    ids = sorted(set(re.findall(r'ID: (RUSTSEC-\d{4}-\d{4})', read('checks/dependency-policy-first/stderr.log').decode())))
    require(ids == index['remaining_advisories'] == ['RUSTSEC-2024-0449', 'RUSTSEC-2026-0367', 'RUSTSEC-2026-0368',
            'RUSTSEC-2026-0369', 'RUSTSEC-2026-0370', 'RUSTSEC-2026-0371', 'RUSTSEC-2026-0372'], 'policy result differs')
    require(data('checks/db-metadata-first/stdout.log')['head'] == '84d943a6deeb8cc6c6f632531aaf4f1c62275239', 'DB identity')
    refusal = data('v7-refusal/preflight-refusal.json')
    require(refusal['package_dispatched'] is False and refusal['reason'] == 'reviewed auxiliary input drift', 'V7 bypass')
    return dict(verified=True, approved_exact_public_catalog=True, archived_code_executed=False,
                historical_refusal_promoted=False, package_pass=False, policy_pass=False, release_admitted=False)


if __name__ == '__main__':
    print(json.dumps(check(), sort_keys=True))
