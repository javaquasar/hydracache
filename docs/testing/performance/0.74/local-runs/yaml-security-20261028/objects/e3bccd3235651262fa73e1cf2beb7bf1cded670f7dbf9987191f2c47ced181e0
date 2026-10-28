"""Exact reviewed Swagger source inputs; no broad catalog or dependency refresh."""
import copy
import hashlib
import json
import os
from pathlib import Path

import security_lock

RULES_SHA = '868507587efb67519060d05854dcc44dd2de4395e4d9d478f40cfc442580c517'
BODIES = {
    'Cargo.lock': ('00efd4a2cafeb5e82d757cc4855e440c627211c1544c22dcbf8c875f85337cd8',
                   '50ec8e7e20a8caebeefdbdde7bbafb36b96257daf1204dcd6dfa8f6c5b7c925a', 37),
    'Cargo.toml': ('fd57accedc5029f0024c45e72f3c3ef66501d670eaa8776cf293553ff8d5a1ad',
                   '7877b4051e4952e7a809896cf46c03a80c6ae3bae3fd13ed95ed0b47d377f472', 1),
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def rules():
    path = Path(__file__).with_name('swagger-inputs.json')
    require(not path.is_symlink() and not path.is_junction(), 'Swagger rules custody')
    raw = path.read_bytes()
    require(sha(raw) == RULES_SHA, 'exact reviewed Swagger inverse edits')
    value = json.loads(raw)
    require(set(value) == {'schema', 'previous_revision', 'graph_review_sha256', 'openapi_sha256', 'artifacts'}, 'Swagger rules schema')
    require(value['schema'] == 'hc074-reviewed-swagger-inputs-v1' and
            value['previous_revision'] == '58a994a4c79fd5e8dd2fa622bcd57bbf640ff2df' and
            value['openapi_sha256'] == '989c92b97e9c4475792864fd073664a6ac81cf2f246f38492412636fa2768414', 'Swagger predecessor and document')
    require(set(value['artifacts']) == set(BODIES), 'exact Swagger artifact set')
    for name, (current, previous, count) in BODIES.items():
        row = value['artifacts'][name]
        require(set(row) == {'current_sha256', 'previous_sha256', 'patches'} and
                (row['current_sha256'], row['previous_sha256'], len(row['patches'])) ==
                (current, previous, count), 'Swagger full body identities')
    return value['artifacts']


def predecessor(name, raw):
    require(name in BODIES, 'unknown Swagger artifact')
    row = rules()[name]
    raw = raw.replace(b'\r\n', b'\n')  # The inherited review's original normalization.
    require(sha(raw) == row['current_sha256'], 'unreviewed complete Swagger input: '+name)
    limit = len(raw)
    for edit in reversed(row['patches']):
        require(set(edit) == {'offset', 'current', 'previous'} and type(edit['offset']) is int
                and edit['offset'] >= 0 and isinstance(edit['current'], str)
                and isinstance(edit['previous'], str), 'Swagger inverse edit schema')
        start = edit['offset']
        current, previous = edit['current'].encode(), edit['previous'].encode()
        end = start+len(current)
        require(end <= limit and raw[start:end] == current, 'ordered exact Swagger inverse edit')
        raw = raw[:start]+previous+raw[end:]
        limit = start
    require(sha(raw) == row['previous_sha256'], 'complete pre-Swagger input: '+name)
    return raw


def reviewed_inputs(root, data):
    require(not any(k.startswith('SWAGGER_UI_') for k in os.environ), 'unreviewed Swagger build override')
    require(data['auxiliary']['Cargo.lock'] == security_lock.PREVIOUS and
            data['auxiliary']['Cargo.toml'] == BODIES['Cargo.toml'][1], 'original inherited Swagger predecessor')
    result = copy.deepcopy(data)
    for name in ('Cargo.lock', 'Cargo.toml'):
        path = root/name
        require(path.is_file() and not path.is_symlink() and not path.is_junction()
                and path.resolve().parent == root.resolve(), 'Swagger input custody')
        previous = predecessor(name, path.read_bytes())
        if name == 'Cargo.lock':
            # Preserve the earlier complete sys security predecessor proof too.
            security_lock.predecessor(previous)
        result['auxiliary'][name] = BODIES[name][0]
    return result
