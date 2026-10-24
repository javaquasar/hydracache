"""Two existing census-fixture trust anchors; never mint pins from a batch result."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
SOURCE = 'tools/long-run-supervisor-074/src/diagnostic_artifacts_linux.rs'
BASE = '25faa7bb762aa229631e6f6cfc25cae33e16ff43'
REVIEWED = 'c91068bfa8b297bb075b63457da235aaa9c6232e'
BASE_SHA = 'ede23782d4ff6a5ae052ca542998c3ebd8968af2ad678f67b179af4fdb05d828'
PACKET = 'docs/testing/performance/0.74/local-runs/role-census-inventory-20261024'
PACKET_SHA = '86946490836001a8f887220767b3d3677a050bbf94664370b09fd548ff8a1251'
AUDIT_SHA = 'cfd5e7f1c153f5214dd71da196dfdbac2d1c05806fa14865605e9ce0776bc9c0'
CALLER = 'crates/xtask/tests/support/census_provider_input.rs'
CONTRACT = 'tools/material-lock-fixture-074/contract.rs'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def decode(raw):
    def unique(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError('duplicate reference field')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=unique)


def member(folder, index, name):
    entry = index['members'][name]
    digest = entry['sha256']
    if not re.fullmatch('[0-9a-f]{64}', digest):
        raise ValueError('reference object name')
    body = (folder / 'objects' / digest).read_bytes()
    if len(body) != entry['bytes'] or sha(body) != digest:
        raise ValueError('reference member drift')
    return body


def verify_body(body, digest, size=None):
    # The old BASE anchor is normalized; require canonical LF bytes as an extra
    # condition before treating that existing hash as a raw byte pin.
    if b'\r' in body or sha(body) != digest or (size is not None and len(body) != size):
        raise ValueError('independent source anchor mismatch')
    body.decode('utf-8', errors='strict')
    return dict(kind='blob', oid=hashlib.sha1(b'blob ' + str(len(body)).encode() + b'\0' + body).hexdigest(),
                bytes=len(body), sha256=digest)


def prepare(root=ROOT):
    root = Path(root).resolve(strict=True)
    folder = root / PACKET
    raw = (folder / 'index.json').read_bytes()
    if sha(raw) != PACKET_SHA:
        raise ValueError('reviewed reference packet drift')
    index = decode(raw)
    audit_raw = member(folder, index, 'audit/inventory.json')
    if sha(audit_raw) != AUDIT_SHA:
        raise ValueError('independent inventory drift')
    audit = decode(audit_raw)
    if audit['pinned_current_source'] != REVIEWED:
        raise ValueError('reviewed revision drift')
    caller = (root / CALLER).read_bytes().replace(b'\r\n', b'\n')
    archived = member(folder, index, 'source/current/' + CALLER).replace(b'\r\n', b'\n')
    if caller != archived:
        raise ValueError('selected caller changed')
    # The literal BASE hash is authoritative only inside the exact contract
    # whose whole-file identity was retained before this candidate existed.
    intent = decode(member(folder, index, 'observation/census-exact-candidate/intent.json'))
    contract_raw = (root / CONTRACT).read_bytes()
    if sha(contract_raw) != intent['source'][CONTRACT]:
        raise ValueError('sealed base contract changed')
    contract = contract_raw.decode('utf-8', errors='strict')
    for literal in (SOURCE, BASE, BASE_SHA):
        if contract.count('"' + literal + '"') != 1:
            raise ValueError('existing base authority changed')
    reviewed = audit['current'][SOURCE]
    if reviewed['mode'] != '100644':
        raise ValueError('reviewed mode drift')
    cases = []
    # These references are fixed before invoking Git. No mutable ref, query
    # output hash or native stream is allowed to refresh either anchor.
    for revision, digest, size, oid in ((BASE, BASE_SHA, None, None),
            (REVIEWED, reviewed['canonical_sha256'], reviewed['bytes'], reviewed['blob'])):
        spec = revision + ':' + SOURCE
        result = subprocess.run(['git', '--no-replace-objects', 'show', spec], cwd=root,
                                capture_output=True, timeout=10)
        if result.returncode != 0:
            raise ValueError('historical reference unavailable')
        pin = verify_body(result.stdout, digest, size)
        if oid is not None and pin['oid'] != oid:
            raise ValueError('independent blob identity drift')
        cases.append(dict(spec=spec, pin=pin, stdout_hex=result.stdout.hex(),
                          authority=('existing-contract-lf-byte-pin' if revision == BASE
                                     else 'sealed-reviewed-inventory'), stderr_hex=result.stderr.hex()))
    return dict(schema='hc074-census-caller-pins-v1', caller=CALLER, cases=cases,
        reference_packet_sha256=PACKET_SHA, inventory_sha256=AUDIT_SHA,
        test_fixture_pin_integrity=True, production_pin_authority=False,
        caller_integrated=False, caller_admitted=False, live_checks_or_deadlines_changed=False,
        server=False, services_or_clocks=False, measurement=False)


if __name__ == '__main__':
    print(json.dumps(prepare(), sort_keys=True))
