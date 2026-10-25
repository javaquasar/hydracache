"""Offline verification of historical source data; no Git/code execution or network."""
import hashlib
import json
from pathlib import Path

INDEX_SHA256 = 'cf14d72fa0637ef5aeab50ca96ff2cfa69a9e331da3e358c779ff0f830d807d7'
LEDGER_SHA256 = 'fec2195d38bd7784e9c584704b90d5851f5a5c228a900513aa11cf91b42fad8f'
OLD = 'c91068bfa8b297bb075b63457da235aaa9c6232e'
NEW = '958b3135a15c00b9f6aad41522cef9d1f6608d3a'


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def verify(folder):
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == INDEX_SHA256, 'fixed provenance index differs')
    index = json.loads(raw)
    bodies = {}
    for name, row in index['members'].items():
        digest = row['sha256']
        require(len(digest) == 64 and all(c in '0123456789abcdef' for c in digest), 'invalid object digest')
        body = (folder / 'objects' / digest).read_bytes()
        require(sha(body) == digest and len(body) == row['bytes'], 'raw member differs')
        bodies[name] = body
    require(len(bodies) == 47, 'fixed member inventory differs')
    require(sha(bodies['phase-ledger.json']) == LEDGER_SHA256 == index['ledger_sha256'], 'fixed phase ledger differs')
    require(sha(bodies['previous-ledger.json']) == 'cfd5e7f1c153f5214dd71da196dfdbac2d1c05806fa14865605e9ce0776bc9c0',
            'original ledger differs')
    ledger = json.loads(bodies['phase-ledger.json'])
    previous = json.loads(bodies['previous-ledger.json'])
    require(ledger['schema'] == 'hc074-inventory544-phase-v1' and
            ledger['previous_revision'] == OLD and ledger['revision'] == NEW, 'phase identity differs')

    def tree(revision):
        result = {}
        for line in bodies['tree-' + revision + '.raw'].decode().splitlines():
            metadata, path = line.split('\t')
            mode, kind, blob = metadata.split()
            require(mode == '100644' and kind == 'blob' and path not in result, 'invalid source tree row')
            result[path] = dict(mode=mode, blob=blob)
        return result

    old, current = tree(OLD), tree(NEW)
    require(len(old) == 530 and len(current) == 544 and not old.keys() - current.keys(), 'source counts/removal differ')
    require(previous['pinned_current_source'] == OLD and
            {name:dict(mode=row['mode'], blob=row['blob']) for name,row in previous['current'].items()} == old,
            'original description/tree differ')
    added, changed = current.keys() - old.keys(), {name for name in old if old[name] != current[name]}
    require(len(added) == 14 and len(changed) == 3 and set(ledger['added']) == added and
            set(ledger['changed']) == changed and ledger['removed'] == [], 'exact source delta differs')
    for name in sorted(added | changed):
        row = ledger['added'][name] if name in added else ledger['changed'][name]['current']
        body = bodies['source/' + NEW + '/' + name]
        require(row == dict(**current[name], canonical_sha256=sha(body.decode('utf-8').replace('\r\n','\n').encode())),
                'current source identity differs')
        # Git object IDs describe the original raw blob, not normalized bytes.
        blob = hashlib.sha1(b'blob ' + str(len(body)).encode() + b'\0' + body).hexdigest()
        require(blob == row['blob'], 'raw source Git blob differs')
        if name in changed:
            prior = ledger['changed'][name]['previous']
            old_body = bodies['source/' + OLD + '/' + name]
            require(prior == dict(**old[name], canonical_sha256=sha(old_body.decode('utf-8').replace('\r\n','\n').encode())),
                    'old source identity differs')
            require(all(prior[key] == previous['current'][name][key] for key in prior), 'old ledger row differs')
    expected = ''.join(f"{row['mode']} {row['blob']} 0\t{name}\n" for name,row in sorted(current.items()))
    require(bodies['observed-index.raw'].decode() == expected, 'observed index differs from independent producer')
    require(index['current_is_fixed_historical_tree'] is True and index['original_ledgers_changed'] is False and
            index['product_or_qualification_admission'] is False, 'historical scope changed')
    return dict(previous=530, current=544, added=14, changed=3, unchanged=527,
                ledger_sha256=LEDGER_SHA256, code_executed=False, admission=False)


if __name__ == '__main__':
    print(json.dumps(verify(Path(__file__).resolve().parent), indent=2, sort_keys=True))
