"""Read-only packet review, independent of its writer; never runs packet code."""
from pathlib import Path
import hashlib
import json
import re
import sys


def need(value, reason):
    if not value:
        raise ValueError(reason)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def main(packet, expected):
    need(packet.is_dir() and not packet.is_symlink() and not packet.is_junction(), 'regular packet')
    need(not (packet/'objects').is_symlink() and not (packet/'objects').is_junction(), 'regular object store')
    raw = (packet/'index.json').read_bytes()
    need(digest(raw) == expected, 'independently supplied index pin')
    index = json.loads(raw)
    objects = set()
    for name, row in index['files'].items():
        need(not name.startswith('/') and '\\' not in name and ':' not in name
             and all(p not in ('', '.', '..') for p in name.split('/')), 'regular logical member')
        value = row['sha256']
        need(re.fullmatch('[0-9a-f]{64}', value), 'canonical object name')
        path = packet/'objects'/value
        need(path.is_file() and not path.is_symlink(), 'regular referenced object')
        body = path.read_bytes()
        need(len(body) == row['bytes'] and digest(body) == value, 'complete object bytes: '+name)
        objects.add(value)
    need(objects == {p.name for p in (packet/'objects').iterdir()}, 'no missing or orphan object')

    def body(name):
        row = index['files'][name]
        value = (packet/'objects'/row['sha256']).read_bytes()
        need(digest(value) == row['sha256'] and len(value) == row['bytes'], 'fresh member read')
        return value

    def doc(name):
        return json.loads(body(name))

    def owned(value):
        need(value['exit'] == 0 and value['retired'] and not value['errors']
             and value['bound_seconds'] == 1200 and value['cleanup_seconds'] == 5
             and value['capacity_poll_seconds'] == 5, 'unchanged successful CI owner')

    scope = 'formatted8/'
    freeze = doc(scope+'freeze/runtime-reviewed.json')
    need(len(freeze['source']) == 2246 and not freeze['complete_gate_reuse']
         and not freeze['partial_tests_reused'], 'whole final source and no reused test counts')
    for name, value in freeze['source'].items():
        need(digest(body('tested-source/'+name)) == value, 'self-contained actual source')
    protected = {
        'Cargo.lock': '76d4ab3b38c1b259c0784e66dd620a5f61d45cde1760a9533c4d6c7db061b207',
        'Cargo.toml': '84b1986d8a2014be985460b3ecc4a809f3ae3e8f9822ef6c7134b3ecbe9af114',
        'deny.toml': '16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930',
        'docs/testing/performance/0.74/qualification-manifest.toml':
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc',
    }
    need(all(freeze['source'][n] == v for n, v in protected.items()), 'exact accepted graph and unchanged protected pins')
    parent = Path(__file__).resolve().parents[2]/'docs/testing/performance/0.74/local-runs/dbus-root-integration-20261028'
    parent_raw = (parent/'index.json').read_bytes()
    need(digest(parent_raw) == '149698f228bba92ece4c5454db982977093899bfc8da6cf5e50c9405c2708ca9', 'original approved root packet')
    row = json.loads(parent_raw)['files']['candidate/fileset.json']
    recipe = (parent/'objects'/row['sha256']).read_bytes()
    need(digest(recipe) == row['sha256'] and len(recipe) == row['bytes'], 'original whole recipe')
    vendor = [v for v in json.loads(recipe)['operations'] if v['path'].startswith('third_party/')]
    need(len(vendor) == 266 and all(freeze['source'][v['path']] == v['after_sha256'] for v in vendor),
         'all canonical reviewed vendor bodies remain exact')
    for phase in ('format','checks','focus-194','focus-188','spool-194','spool-188','tests-194','tests-188'):
        path = scope+phase+'/'
        need(doc(path+'result.json')['passed'], 'complete phase')
        value = doc(path+'native-result.json')
        need(value['passed'] and value['source_unchanged'] and value['boot_unchanged']
             and value['boot_before'] == value['boot_after'], 'native final postflight')
        owned(doc(path+'execution.json'))
        intent = doc(path+'intent.json')
        need(intent['bound'] == 1200 and intent['session_bound'] == 1100
             and intent['floor'] == 4*1024**3 and intent['poll'] == intent['retirement'] == 5
             and not intent['retry'], 'unchanged phase limits')
        if phase.startswith(('focus-', 'spool-')):
            size = 17 if phase.startswith('focus-') else 7
            need(value['tests']['passed'] == value['tests']['catalog'] == size
                 and value['tests']['ignored'] == value['tests']['filtered'] == 0,
                 'unfiltered full focused harness')
    results = []
    catalogs = []
    for compiler in ('194', '188'):
        base = scope+'tests-'+compiler+'/'
        images = doc(base+'images.json')
        catalog = doc(base+'catalogs.json')
        need(len(images) == len(catalog) == 90, 'every original target')
        total = dict(catalog=0, passed=0, ignored=0, targets=90)
        for number, image in enumerate(images):
            prefix = base+f'target-{number:03d}'
            names = catalog[image['kind'][0]+'/'+image['name']]
            listed = sorted(s[:-6] for s in body(prefix+'-list/stdout.log').decode().splitlines()
                            if s.endswith(': test'))
            need(names == listed and len(names) == len(set(names)), 'actual full unique listing')
            rows = []
            for position, name in enumerate(names) if image['kind'] == ['lib'] else [(None, None)]:
                part = prefix+f'-cases/case-{position:04d}/' if position is not None else prefix+'-test/'
                raw = body(part+'stdout.log').decode()
                parsed = re.findall(r'^test (.+?) \.\.\. (ok|ignored(?:, [^\n]*)?)$', raw, re.M)
                summary = re.findall(r'^test result: ok\. (\d+) passed; 0 failed; (\d+) ignored; 0 measured; (\d+) filtered out;', raw, re.M)
                need(len(summary) == 1, 'one complete successful test summary')
                passed, ignored, filtered = map(int, summary[0])
                need(len(parsed) == passed+ignored and sum(s == 'ok' for _, s in parsed) == passed,
                     'raw rows agree with summary')
                need(filtered == (len(names)-1 if position is not None else 0), 'no unintended filtering')
                if position is not None:
                    need([n for n, _ in parsed] == [name], 'one intended library case')
                    intent = doc(part+'intent.json')
                    need(intent['argv'] == [image['path'], '--exact', name, '--test-threads=1', '--color=never'],
                         'original owned-case outer frame remains selected')
                else:
                    need(doc(part+'intent.json')['argv'] == [image['path'], '--test-threads=1'],
                         'original integration harness remains unfiltered')
                owned(doc(part+'execution.json'))
                rows.extend(parsed)
            need(sorted(n for n, _ in rows) == names, 'every full target case exactly once')
            total['catalog'] += len(rows)
            total['passed'] += sum(s == 'ok' for _, s in rows)
            total['ignored'] += sum(s.startswith('ignored') for _, s in rows)
        need(total == doc(base+'totals.json') == dict(catalog=2241, passed=2105, ignored=136, targets=90),
             'independent complete totals')
        private = body(base+'private-bus/stdout.log').decode()
        need(re.findall(r'^HC074-NEGOTIATED case=([a-z-]+) result=ok ', private, re.M) ==
             ['bare-receiver','healthy','atomic-adapter','fragmented','lost-ack','foreign-sender',
              'wrong-serial','disconnect','manager-error','no-capability'], 'all ten actual transport cases')
        owned(doc(base+'private-bus/execution.json'))
        results.append(total)
        catalogs.append(catalog)
    need(results[0] == results[1] and catalogs[0] == catalogs[1], 'actual GNU/MSRV catalogs equal')
    need('1 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out;' in
         body('resource-first/tests-194/target-055-test/stdout.log').decode(), 'first stale fixture refusal retained')
    need(not doc('caller-first/held-startup-attribution/native-result.json')['acceptance_pass'], 'diagnostic never promoted')
    need(doc('hash-leaf-first/tests-194/execution.json')['classification'] == 'resource-floor', 'first capacity stop retained')
    need(index['original_windows_seconds'] == [300,60,10]
         and not index['runtime_live_checks_changed'] and not index['performance_claim']
         and not index['release_admitted'] and not index['workspace_gate_complete']
         and not index['security_gate_complete'] and not index['partial_case_counts_reused'], 'honest gate boundary')
    return dict(passed=True, index_sha256=expected, members=len(index['files']), objects=len(objects),
                independent_raw_test_accounting=True, totals_per_compiler=results[0],
                full_catalogs_equal=True, source_files=2246, canonical_vendor_files=266, first_refusals_retained=True,
                server=False, release_admitted=False, performance_claim=False)


if __name__ == '__main__':
    need(len(sys.argv) == 3, 'packet path and independently supplied index digest')
    print(json.dumps(main(Path(sys.argv[1]), sys.argv[2]), sort_keys=True))
