"""Exact root D-Bus successor; every older YAML/Swagger/sys proof still runs."""
import copy
import json
import os
from pathlib import Path

import security_lock
import swagger_inputs as s
import yaml_inputs as y

RULES_SHA = 'd9cb550f86f9d71090a75ce1df6a405db4edb73a977e66546e952d6cef28202e'
BODIES = {
    'Cargo.lock': ('76d4ab3b38c1b259c0784e66dd620a5f61d45cde1760a9533c4d6c7db061b207',
                   y.BODIES['Cargo.lock'][0], 38),
    'Cargo.toml': ('29118852fb21a5b3d9fc091bea2bb9e4240f47fee75ea8a538f29c0dae56858d',
                   s.BODIES['Cargo.toml'][0], 2),
}


def rules():
    path = Path(__file__).with_name('dbus-inputs.json')
    s.require(not path.is_symlink() and not path.is_junction(), 'D-Bus rules custody')
    raw = path.read_bytes()
    s.require(s.sha(raw) == RULES_SHA, 'exact reviewed D-Bus inverse edits')
    value = json.loads(raw)
    s.require(set(value) == {'schema','previous_revision','source_review_sha256',
                            'package_review_sha256','artifacts'}, 'D-Bus rules schema')
    s.require(value['schema'] == 'hc074-reviewed-dbus-inputs-v1'
              and value['previous_revision'] == '75ca41e25b71d09cd5fbee6271680b5035027c9c'
              and value['source_review_sha256'] == '149698f228bba92ece4c5454db982977093899bfc8da6cf5e50c9405c2708ca9'
              and value['package_review_sha256'] == '33cfe7ec5061690fa0f03ab5d81df4a598d7a8a810ffc4ae7457fb4bc97d24d9',
              'D-Bus source/effect/package predecessors')
    s.require(set(value['artifacts']) == set(BODIES), 'exact D-Bus artifact set')
    for name, (current, previous, count) in BODIES.items():
        row = value['artifacts'][name]
        s.require(set(row) == {'current_sha256','previous_sha256','patches'}
                  and (row['current_sha256'], row['previous_sha256'], len(row['patches'])) ==
                  (current, previous, count), 'D-Bus whole body identities')
    return value['artifacts']


def predecessor(name, raw):
    s.require(name in BODIES, 'unknown D-Bus artifact')
    row = rules()[name]
    raw = raw.replace(b'\r\n', b'\n')  # The unchanged inherited normalization.
    s.require(s.sha(raw) == row['current_sha256'], 'unreviewed complete D-Bus input: '+name)
    limit = len(raw)
    for edit in reversed(row['patches']):
        s.require(set(edit) == {'offset','current','previous'} and type(edit['offset']) is int
                  and edit['offset'] >= 0 and isinstance(edit['current'], str)
                  and isinstance(edit['previous'], str), 'D-Bus inverse edit schema')
        start = edit['offset']
        current, previous = edit['current'].encode(), edit['previous'].encode()
        end = start+len(current)
        s.require(end <= limit and raw[start:end] == current, 'ordered exact D-Bus inverse edit')
        raw = raw[:start]+previous+raw[end:]
        limit = start
    s.require(s.sha(raw) == row['previous_sha256'], 'complete pre-D-Bus input: '+name)
    return raw


def reviewed_inputs(root, data):
    s.require(not any(k.startswith('SWAGGER_UI_') for k in os.environ), 'unreviewed Swagger build override')
    s.require(data['auxiliary']['Cargo.lock'] == security_lock.PREVIOUS
              and data['auxiliary']['Cargo.toml'] == s.BODIES['Cargo.toml'][1]
              and data['auxiliary']['crates/xtask/Cargo.toml'] == y.BODIES['crates/xtask/Cargo.toml'][1],
              'original inherited D-Bus predecessors')
    result = copy.deepcopy(data)
    for name in ('Cargo.lock','Cargo.toml','crates/xtask/Cargo.toml'):
        path = root/name
        s.require(path.is_file() and not path.is_symlink() and not path.is_junction()
                  and path.resolve() == root.resolve()/name, 'D-Bus input custody')
        raw = path.read_bytes()
        if name == 'Cargo.lock':
            previous = y.predecessor(name, predecessor(name, raw))
            security_lock.predecessor(s.predecessor(name, previous))
            current = BODIES[name][0]
        elif name == 'Cargo.toml':
            s.predecessor(name, predecessor(name, raw))
            current = BODIES[name][0]
        else:
            y.predecessor(name, raw)
            current = y.BODIES[name][0]
        result['auxiliary'][name] = current
    return result
