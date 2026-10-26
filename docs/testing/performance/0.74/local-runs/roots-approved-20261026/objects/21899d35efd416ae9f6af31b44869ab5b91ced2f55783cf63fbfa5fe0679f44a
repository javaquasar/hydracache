"""Exact public-catalog audit. No Git, network, code generation or enrollment."""
import argparse
import ast
from collections import Counter
import copy
import hashlib
import json
from pathlib import Path
import re
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
PACKET = 'docs/testing/performance/0.74/local-runs/roots-security-20261026'
INDEX_SHA = '5a66272e834fd917f87f7063f1415d1e50beba5b18f776104a7b8a19c4a6f73d'
POLICY = Path(__file__).with_name('approved-catalog.json')
LITERAL = r'b"(?:[^"\\]|\\.)*"'
ANCHOR = (r'TrustAnchor\s*\{\s*subject:\s*Der::from_slice\((' + LITERAL + r')\),\s*'
          r'subject_public_key_info:\s*Der::from_slice\((' + LITERAL + r')\),\s*'
          r'name_constraints:\s*(None|Some\(Der::from_slice\((' + LITERAL + r')\)\))')


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def packet_reader(root=ROOT):
    folder = root / PACKET
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == INDEX_SHA, 'historical packet identity differs')
    index = json.loads(raw)

    def read(name):
        row = index['members'][name]
        require(re.fullmatch('[a-f0-9]{64}', row['sha256']), 'object pointer')
        raw = (folder / 'objects' / row['sha256']).read_bytes()
        require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'historical object differs')
        return raw
    return read


def parse_anchors(raw):
    text = raw.decode()
    found = re.findall(ANCHOR, text)
    require(found and len(found) == text.count('subject_public_key_info: Der::from_slice('),
            'incomplete anchor census')
    return Counter((ast.literal_eval(s), ast.literal_eval(k), None if c == 'None' else ast.literal_eval(v))
                   for s, k, c, v in found)


def parse_certificates(raw):
    text = raw.decode()
    found = re.findall(r'CertificateDer::from_slice\((' + LITERAL + r')\)', text)
    require(found and len(found) == text.count('CertificateDer::from_slice('), 'incomplete DER census')
    return Counter(ast.literal_eval(value) for value in found)


def fields(raw):
    """Bounded definite-length DER TLVs; reject truncation/nonminimal lengths."""
    offset, result = 0, []
    while offset < len(raw):
        start = offset
        require(offset + 2 <= len(raw), 'truncated DER header')
        tag, size = raw[offset:offset + 2]
        offset += 2
        if size & 128:
            count = size & 127
            require(0 < count <= 4 and offset + count <= len(raw) and raw[offset] != 0, 'DER length')
            size = int.from_bytes(raw[offset:offset + count], 'big')
            require(size >= 128, 'nonminimal DER length')
            offset += count
        end = offset + size
        require(end <= len(raw), 'truncated DER body')
        result.append((tag, raw[offset:end], raw[start:end]))
        offset = end
    return result


def certificate_identity(raw):
    outer = fields(raw)
    require(len(outer) == 1 and outer[0][0] == 0x30, 'certificate envelope')
    parts = fields(outer[0][1])
    require(len(parts) == 3 and parts[0][0] == 0x30, 'certificate parts')
    body = fields(parts[0][1])
    shift = 1 if body and body[0][0] == 0xa0 else 0
    require(len(body) >= shift + 6 and body[shift + 4][0] == body[shift + 5][0] == 0x30,
            'certificate subject/SPKI')
    extensions = {}
    for tag, value, _ in body[shift + 6:]:
        if tag != 0xa3:
            continue
        sequence = fields(value)
        require(len(sequence) == 1 and sequence[0][0] == 0x30, 'extension sequence')
        for tag, value, _ in fields(sequence[0][1]):
            require(tag == 0x30, 'extension tag')
            row = fields(value)
            require(len(row) in (2, 3) and row[0][0] == 6 and row[-1][0] == 4, 'extension fields')
            oid = row[0][1].hex()
            require(oid not in extensions, 'duplicate extension')
            require(len(row) != 3 or row[1][:2] == (1, b'\xff'), 'extension critical flag')
            extensions[oid] = dict(critical=len(row) == 3, value_hex=row[-1][1].hex())
    return body[shift + 4][1], body[shift + 5][1], body[shift + 5][2], extensions


def validate_catalogs(old, new, old_der, new_der, policy):
    require((sum(old.values()), sum(new.values()), sum(old_der.values()), sum(new_der.values()),
             sum((old & new).values()), sum((old_der & new_der).values())) == (118, 120, 118, 120, 116, 116),
            'catalog size or surviving policy differs')
    changes = {}
    for label, anchors, certs in (('removed', old - new, old_der - new_der),
                                  ('added', new - old, new_der - old_der)):
        identities = [(c, certificate_identity(c)) for c in certs.elements()]
        rows = []
        for subject, key, constraints in anchors.elements():
            matching = [(c, identity) for c, identity in identities if identity[:2] == (subject, key)]
            require(len(matching) == 1, 'changed anchor/DER mapping differs')
            cert, (_, _, spki, extensions) = matching[0]
            rows.append(dict(spki_sha256=sha(spki), certificate_sha256=sha(cert),
                             name_constraints_hex=None if constraints is None else constraints.hex()))
            if label == 'added':
                require(constraints is None and '551d1e' not in extensions, 'new root name constraints differ')
                basic = extensions.get('551d13', {})
                require(basic.get('critical') is True and basic.get('value_hex') == '30030101ff',
                        'new root CA constraints differ')
        expected = [{k: row[k] for k in ('spki_sha256', 'certificate_sha256', 'name_constraints_hex')}
                    for row in policy[label]]
        require(len(identities) == len(rows) and sorted(rows, key=lambda r: r['spki_sha256']) ==
                sorted(expected, key=lambda r: r['spki_sha256']), 'unapproved ' + label + ' keys/constraints')
        changes[label] = rows
    return dict(old_entries=118, new_entries=120, unchanged_complete_anchors=116,
                unchanged_der_certificates=116, **changes, exact_der_mapping=True,
                new_roots_critical_ca_true=True, new_name_constraints_absent=True)


def audit(read=None, policy=None):
    read = read or packet_reader()
    policy = policy or json.loads(POLICY.read_bytes())
    require(policy['schema'] == 'hc074-public-roots-approval-v1' and
            policy['versions'] == {'webpki-roots': '1.0.10', 'webpki-root-certs': '1.0.10'} and
            (policy['old_entries'], policy['new_entries'], policy['unchanged_complete_anchors']) == (118, 120, 116),
            'approved scope differs')
    for key in ('production_issuer_or_enrollment_pins_changed', 'tenant_authorization_changed', 'release_admitted'):
        require(policy[key] is False, 'production scope expansion')
    def source(name, version):
        return read('upstream/' + name + '-' + version + '/src/lib.rs')
    return dict(validate_catalogs(parse_anchors(source('webpki-roots', '1.0.8')),
                parse_anchors(source('webpki-roots', '1.0.10')),
                parse_certificates(source('webpki-root-certs', '1.0.8')),
                parse_certificates(source('webpki-root-certs', '1.0.10')), policy),
                approved_exact_public_catalog=True, production_enrollment_admitted=False,
                policy_pass=False, full_package_pass=False, release_admitted=False)


def registry_audit(registry, read=None):
    read = read or packet_reader()
    graph = tomllib.loads(read('source/Cargo.lock').decode())
    for name in ('webpki-roots', 'webpki-root-certs'):
        node = next(p for p in graph['package'] if p['name'] == name and p['version'] == '1.0.10')
        folder = name + '-1.0.10'
        crate = registry.parent.parent / 'cache' / registry.name / (folder + '.crate')
        require(sha(crate.read_bytes()) == node['checksum'], 'published crate checksum differs')
        with tarfile.open(crate, mode='r:gz') as archive:
            for relative in ('src/lib.rs', 'Cargo.toml', '.cargo_vcs_info.json', 'LICENSE'):
                expected = read('upstream/' + folder + '/' + relative)
                require((registry / folder / relative).read_bytes() == expected, 'registry source differs')
                require(archive.extractfile(folder + '/' + relative).read() == expected, 'published source differs')
    return dict(exact_published_crates_verified=True, registry_source_matches_archived_audit=True)


def validate_graph(before, after):
    expected = copy.deepcopy(before)
    published = tomllib.loads(packet_reader()('source/Cargo.lock').decode())
    for name in ('webpki-roots', 'webpki-root-certs'):
        rows = [p for p in expected['package'] if p['name'] == name and p['version'] == '1.0.8']
        require(len(rows) == 1, 'baseline root membership')
        node = next(p for p in published['package'] if p['name'] == name and p['version'] == '1.0.10')
        rows[0].update(version='1.0.10', checksum=node['checksum'])
    for package in expected['package']:
        package['dependencies'] = ['webpki-roots 1.0.10' if edge == 'webpki-roots 1.0.8' else edge
                                   for edge in package.get('dependencies', [])] if 'dependencies' in package else []
        if not package['dependencies']:
            package.pop('dependencies')
        if package['name'] == 'hydracache-redis-compat':
            package['dependencies'] = sorted(package['dependencies'] +
                ['webpki-root-certs', 'webpki-roots 0.26.11', 'webpki-roots 1.0.10'])
    require(expected == after, 'unrelated lock change')
    return dict(exact_two_catalog_updates_and_three_test_edges=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--registry', type=Path)
    args = parser.parse_args()
    result = audit()
    if args.registry is not None:
        result.update(registry_audit(args.registry))
    print(json.dumps(result, indent=2, sort_keys=True))
