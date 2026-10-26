"""Offline syntax receipt verification; no package, Git or retained code execution."""
import hashlib
import json
from pathlib import Path

INDEX = '4788224085aadbfa874f16d13826fc34d2c32ac880116d7e959c8e27df53a062'
AST = '2552f3a6d96786433ea27b111a3f9f3b7274e82446634d68ca7d49090e24aa54'
RAW = 'e8c6a3141d7381fb024e712e0287a980bc0e9d062597bac437558dbce3bd6996'
SOURCE = '6e4913efbd89a81f32baef8f32a1646284caab5a'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def verify(folder):
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and all(index[key] is False for key in
            ('execution_effects_review_complete', 'package_dispatched', 'package_pass')), 'scope differs')
    members = {}
    for name, row in index['members'].items():
        body = (folder / 'objects' / row['sha256']).read_bytes()
        require(len(body) == row['bytes'] and sha(body) == row['sha256'], 'raw member differs: ' + name)
        members[name] = body
    require(len(members) == 436, 'member count differs')
    document = json.loads(members['syntax/result.json'])
    require(sha(members['syntax/result.json']) == RAW and
            sha(json.dumps(document, sort_keys=True, separators=(',', ':')).encode()) == AST,
            'syntax differs')
    require(document['executed_input_code'] is False and document['compiler_expansion'] is False and
            len(document['sources']) == 368 and sum(len(row['sites']) for row in document['sources']) == 25868,
            'syntax scope differs')
    rows = {row['source']: row for row in document['sources']}
    require(len(rows) == 368, 'duplicate source')
    for name, row in rows.items():
        require(sha(members['fixture/' + name]) == row['sha256'], 'source fixture differs')
    previous = json.loads(members['previous/v5-syntax.json'])
    old = {row['source']: row for row in previous['sources']}
    require(len(old) == 365 and not old.keys() - rows.keys() and
            sum(old[name] != rows[name] for name in old) == 3 and len(rows.keys() - old.keys()) == 3,
            'delta differs')
    edge_names = [name for name in members if name.startswith('syntax/edges') and name.endswith('.json')]
    edge_names.sort(key=lambda name: 0 if name == 'syntax/edges.json' else int(Path(name).stem.split('-')[1]))
    edges = [json.loads(members[name]) for name in edge_names]
    require(bool(edges) and edges[-1] == dict(additions={}, unresolved=[]) and
            all(not row['unresolved'] for row in edges), 'closure differs')
    for label in ('package-v6-syntax-build-first', 'package-v6-syntax-first'):
        prefix = 'checks/' + label + '/'
        intent = json.loads(members[prefix + 'intent.json'])
        result = json.loads(members[prefix + 'result.json'])
        require(intent['head'] == SOURCE and result['exit'] == 0 and result['outer_refusal'] is None and
                result['source_unchanged'] is True and result['head_unchanged'] is True and
                intent['source'] == result['source_after'], 'first check receipt differs')
        require(all(intent['source'][name] == row['sha256'] for name, row in rows.items()),
                'build/scan source identity differs')
    return dict(sources=368, sites=25868, changed=3, added=3, removed=0,
                syntax_only=True, package_dispatched=False, package_pass=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
