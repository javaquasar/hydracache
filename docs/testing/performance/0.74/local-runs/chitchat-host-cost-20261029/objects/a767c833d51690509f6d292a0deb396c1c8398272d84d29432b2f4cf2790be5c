"""Exact YAML successor, preserving whole Swagger/sys predecessor proofs."""
import copy
import json
import os
from pathlib import Path

import security_lock
import swagger_inputs as s

RULES_SHA = '3b03def69502d88f780c22129b719d316a033034f382ba8b0b38f664e26bfe11'
BODIES = {
    'Cargo.lock': ('9efeffabd8726a961a84d23b7a8e5f2ce74e0e313698f064eed603ea79062a7c',
                   '00efd4a2cafeb5e82d757cc4855e440c627211c1544c22dcbf8c875f85337cd8', 6),
    'crates/xtask/Cargo.toml': ('7e8ecf9044d082cabf09444d5be1499b93edba5fa6135f999fafe376d5794756',
                              '9e1b7d0249dc246e2d50bff92b14540c08f379eb98448cb80e13ed325d5c039b', 1),
}


def rules():
    path = Path(__file__).with_name('yaml-inputs.json')
    s.require(not path.is_symlink() and not path.is_junction(), 'YAML rules custody')
    raw = path.read_bytes()
    s.require(s.sha(raw) == RULES_SHA, 'exact reviewed YAML inverse edits')
    value = json.loads(raw)
    s.require(set(value) == {'schema', 'previous_revision', 'graph_review_sha256',
                            'semantic_review_sha256', 'artifacts'}, 'YAML rules schema')
    s.require(value['schema'] == 'hc074-reviewed-yaml-inputs-v1' and
              value['previous_revision'] == '6faa544c9493b4dfdb87fc31ba3fb78456a9455f', 'YAML predecessor')
    s.require(set(value['artifacts']) == set(BODIES), 'exact YAML artifact set')
    for name, (current, previous, count) in BODIES.items():
        row = value['artifacts'][name]
        s.require(set(row) == {'current_sha256', 'previous_sha256', 'patches'} and
                  (row['current_sha256'], row['previous_sha256'], len(row['patches'])) ==
                  (current, previous, count), 'YAML full body identities')
    return value['artifacts']


def predecessor(name, raw):
    s.require(name in BODIES, 'unknown YAML artifact')
    row = rules()[name]
    raw = raw.replace(b'\r\n', b'\n')  # The inherited closure's exact normalization.
    s.require(s.sha(raw) == row['current_sha256'], 'unreviewed complete YAML input: '+name)
    limit = len(raw)
    for edit in reversed(row['patches']):
        s.require(set(edit) == {'offset', 'current', 'previous'} and type(edit['offset']) is int
                  and edit['offset'] >= 0 and isinstance(edit['current'], str)
                  and isinstance(edit['previous'], str), 'YAML inverse edit schema')
        start = edit['offset']
        current, previous = edit['current'].encode(), edit['previous'].encode()
        end = start+len(current)
        s.require(end <= limit and raw[start:end] == current, 'ordered exact YAML inverse edit')
        raw = raw[:start]+previous+raw[end:]
        limit = start
    s.require(s.sha(raw) == row['previous_sha256'], 'complete pre-YAML input: '+name)
    return raw


def reviewed_inputs(root, data):
    s.require(not any(k.startswith('SWAGGER_UI_') for k in os.environ), 'unreviewed Swagger build override')
    s.require(data['auxiliary']['Cargo.lock'] == security_lock.PREVIOUS and
              data['auxiliary']['Cargo.toml'] == s.BODIES['Cargo.toml'][1] and
              data['auxiliary']['crates/xtask/Cargo.toml'] == BODIES['crates/xtask/Cargo.toml'][1],
              'original inherited YAML predecessors')
    result = copy.deepcopy(data)
    for name in ('Cargo.lock', 'Cargo.toml', 'crates/xtask/Cargo.toml'):
        path = root/name
        s.require(path.is_file() and not path.is_symlink() and not path.is_junction()
                  and path.resolve() == root.resolve()/name, 'YAML input custody')
        raw = path.read_bytes()
        if name == 'Cargo.toml':
            s.predecessor(name, raw)
            current = s.BODIES[name][0]
        else:
            previous = predecessor(name, raw)
            if name == 'Cargo.lock':
                security_lock.predecessor(s.predecessor(name, previous))
            current = BODIES[name][0]
        result['auxiliary'][name] = current
    return result
