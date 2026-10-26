"""Offline source-closure replay; no retained Rust, compiler, fixture, Git or Cargo."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile

INDEX = 'be8eb6bc81a7ddfb2dc118827f2946ebab0850e9d6aff1171657898e9a795dc0'
OLD_INDEX = 'e46b334397fcb7ef6ffe36335fbe86a908be481dc12832109bf1da99b93ea6fb'
RESOLVER = 'c4bbc2d6b69e416ac3fcb30a1a64fb3bb54473b70977a1875ff90ca264b0cd44'
ROOT = Path(__file__).resolve().parents[6]
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def verify(folder, root=ROOT):
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == INDEX, 'packet index differs')
    index = json.loads(raw)
    require(index['sources'] == 365 and index['syntax_sites'] == 25774 and index['controls'] == 23,
            'packet scope differs')
    require(all(index[key] is False for key in ('package_dispatched', 'execution_effects_review_complete',
                                               'server', 'qualification')), 'not admission')
    members = {}
    for name, row in index['members'].items():
        body = (folder / 'objects' / row['sha256']).read_bytes()
        require(len(body) == row['bytes'] and sha(body) == row['sha256'], 'raw member differs: ' + name)
        members[name] = body
    require(len(members) == 103, 'member count differs')
    old = root / index['old_packet']
    old_raw = (old / 'index.json').read_bytes()
    require(sha(old_raw) == index['old_index_sha256'] == OLD_INDEX, 'old index differs')
    old_index = json.loads(old_raw)

    def previous(name):
        row = old_index['members'][name]
        body = (old / 'objects' / row['sha256']).read_bytes()
        require(len(body) == row['bytes'] and sha(body) == row['sha256'], 'old member differs')
        return body

    document = json.loads(members['syntax/contextual-closure/result.json'])
    require(document['executed_input_code'] is False and document['compiler_expansion'] is False,
            'source-only scope differs')
    rows = {row['source']: row for row in document['sources']}
    require(len(rows) == len(document['sources']) == 365 and
            sum(len(row['sites']) for row in rows.values()) == 25774, 'syntax count differs')
    preceding = json.loads(previous('syntax/result.json'))
    old_rows = {row['source']: row for row in preceding['sources']}
    delta = json.loads(members['derived/delta.json'])
    require(len(old_rows) == 344 and not old_rows.keys() - rows.keys(), 'old sources removed')
    require(sorted(name for name in old_rows if old_rows[name] != rows[name]) == delta['changed'] and
            sorted(rows.keys() - old_rows.keys()) == delta['added'] and
            len(delta['changed']) == len(delta['added']) == 21 and delta['removed'] == [], 'source delta differs')
    require(delta['execution_effects_review_complete'] is False and delta['package_dispatched'] is False,
            'delta not effect clearance')
    labels = ['package-v5-syntax-build-first', 'package-v5-syntax-first', 'module-context-v5-controls-first',
              'module-context-v5-controls-final', 'package-v5-syntax-context-first',
              'module-context-v5-check', 'module-context-v5-clippy', 'module-context-v5-fmt']
    intents = {}
    for label in labels:
        prefix = 'checks/' + label + '/'
        intent = json.loads(members[prefix + 'intent.json'])
        result = json.loads(members[prefix + 'result.json'])
        intents[label] = intent
        require(result['exit'] == (1 if label == 'package-v5-syntax-first' else 0) and
                result['outer_refusal'] is None and result['source_unchanged'] and result['head_unchanged'] and
                intent['source'] == result['source_after'], 'check outcome differs')
        require(intent['head'] == '6897daaed9aef668f86d77e3ce5076346c20a8f3' and
                intent['bound_seconds'] == 1200 and intent['disk_floor_bytes'] == 4 * 1024**3,
                'check identity/bounds differ')
        require(all(intent[key] is False for key in ('dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'original_owner', 'qualification', 'retry')), 'check scope differs')
    controls = members['checks/module-context-v5-controls-final/stderr.log'].decode()
    require('Ran 23 tests' in controls and controls.rstrip().endswith('OK'), 'control cohort differs')
    first = json.loads(members['syntax/first-refusal/refusal.json'])
    edges = json.loads(members['syntax/first-refusal/edges.json'])
    require(first == dict(type='AssertionError', reason='unresolved actual module edge') and
            len(edges['unresolved']) == 2 and {row['scope'] for row in edges['unresolved']} == {'additions', 'contract'} and
            all(row['source'] == 'tools/current-frontier-074/reader.rs' for row in edges['unresolved']),
            'first refusal differs')
    require(json.loads(members['syntax/contextual-closure/edges-1.json']) == dict(additions={}, unresolved=[]),
            'closure not complete')
    receipt = json.loads(members['syntax/contextual-closure/resolver.json'])
    require(receipt['resolver_sha256'] == RESOLVER and receipt['fallback_search'] is False and
            receipt['package_dispatched'] is False and receipt['execution_effects_cleared'] is False,
            'resolver receipt differs')
    error = json.loads(members['derived/first-control-error.json'])
    require(error['cases'] == 8 and error['passed'] == 7 and error['errors'] == 1 and
            error['raw_streams_retained'] is False, 'derived first error differs')
    name = 'tools/historical-blob-batch-074/module_context_v5.py'
    source = root / name
    require(sha(source.read_bytes()) == sha(members['coordinator/' + name]) == RESOLVER,
            'reviewed local resolver differs')
    spec = importlib.util.spec_from_file_location('checked_local_module_context_v5', source)
    resolver = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(resolver)
    with tempfile.TemporaryDirectory() as directory:
        fixture = Path(directory).resolve()
        for name, row in rows.items():
            body = members.get('delta-source/' + name)
            if body is None:
                body = previous('fixture/' + name)
            require(sha(body) == row['sha256'] and all(intent['source'].get(name) == row['sha256']
                    for intent in intents.values()), 'source/build/check binding differs')
            path = (fixture / name).resolve()
            require(path.is_relative_to(fixture) and path.suffix == '.rs', 'fixture source path differs')
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)
        require(resolver.module_edges(fixture, document) == ({}, []), 'offline module closure differs')
    return dict(sources=365, syntax_sites=25774, controls=23, changed=21, added=21,
                package_dispatched=False, execution_effects_review_complete=False, retained_rust_executed=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
