"""Offline inverse-view and first-outcome verification; never execute stored code."""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
INDEX_SHA256 = 'db231b09668cc784fca1b8464cbcf37119dd45f41b870b8ac14c1ddab2f21431'
QUAL = 'docs/testing/performance/0.74/qualification-manifest.toml'
PATHS = ['Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/src/diagnostic_manager_channel.rs',
         'crates/xtask/tests/performance_contract_074.rs']
PREFIX = '#[path = "../../../tools/authority-runtime-074/gate.rs"]\nmod runtime;\n#[path = "../../../tools/authority-runtime-074/io.rs"]\nmod runtime_io;\n#[path = "../../../tools/authority-storage-source-074/contract.rs"]\nmod storage_contract;\n\n'

def require(value, reason):
    if not value:
        raise ValueError(reason)

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def canonical(raw):
    return raw.replace(b'\r\n',b'\n')

def check(load=None):
    load = load or (lambda name:(HERE/name).read_bytes())
    raw = load('index.json')
    require(sha(raw) == INDEX_SHA256,'index differs')
    index = json.loads(raw)
    for row in index['members'].values():
        raw = load('objects/'+row['sha256'])
        require(len(raw) == row['bytes'] and sha(raw) == row['sha256'],'object differs')
    def read(name):
        return load('objects/'+index['members'][name]['sha256'])
    def data(name):
        return json.loads(read(name))
    def text(name):
        return canonical(read(name)).decode()
    for field in ('historical_source_is_executed','original_checks_or_windows_changed',
            'frozen_qualification_changed','package_dispatched','package_pass',
            'roots_candidate_accepted','server','services','clocks','qualification','release_admitted'):
        require(index[field] is False,'packet promoted')
    require(index['security_input_paths'] == 4 and index['current_root_executing_edits'] == 1,
            'integration scope differs')
    require(len(index['phases']) == 20,'phase membership differs')
    for phase,code in index['phases'].items():
        result = data('checks/'+phase+'/result.json')
        require(result['exit'] == code and result['outer_refusal'] is None and
                result['source_unchanged'] is True and result['commit_unchanged'] is True,
                'first verdict differs')
    for phase in ('focused-terminal-root','msrv-focused','gnu-focused-explicit-git'):
        require(index['counts'][phase] == dict(targets=3,totals=[15,0,0,0,0]),'focused counts differ')
    for phase,counts in (('manager-guards',[9,5,0,0,307]),
            ('manager-diagnostic-successor',[13,1,0,0,307]),
            ('manager-terminal-root',[14,0,0,0,307]),('gnu-focused',[0,3,0,0,0])):
        require(index['counts'][phase]['totals'] == counts,'first cohort lost')
    require(index['phases']['focused'] == 101 and index['phases']['gnu-git-diagnosis'] == 128,
            'initial refusal lost')
    require(b'LowerHex' in read('checks/focused/stderr.log') and
            b'not a git repository' in read('checks/gnu-git-diagnosis/stderr.log'),'first cause lost')
    argv = data('checks/gnu-focused-explicit-git/intent.json')['argv']
    require('GIT_DIR=/mnt/c/Workspace/prj/jq/cashe/hydracache/.git/worktrees/hydracache-074' in argv and
            'GIT_WORK_TREE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074' in argv,'explicit Linux context differs')
    require(index['independent_graph_controls'] == 22 and
            b'Ran 22 tests' in read('checks/graph-controls/stderr.log'),'graph controls differ')
    rules_raw = read('current/tools/security-input-view-074/rules.json')
    require(sha(rules_raw) == '279a9a8c88039fd2224618c0f406d0d4c48eb3135c76560c1981b615c3dd9bc2',
            'rule descriptor differs')
    rules = json.loads(rules_raw)
    require([row['path'] for row in rules['rows']] == PATHS,'path allowlist differs')
    for number,row in enumerate(rules['rows']):
        path = row['path']
        current = canonical(read('current/'+path))
        require(sha(current) == row['current_sha256'],'current body differs')
        require(len(row['patches']) == [2,6,3,6][number],'inverse membership differs')
        limit = len(current)
        for edit in reversed(row['patches']):
            at,chunk = edit['offset'],edit['current'].encode()
            end = at+len(chunk)
            require(0 <= at <= end <= limit and current[at:end] == chunk,'inverse boundary differs')
            current = current[:at]+edit['previous'].encode()+current[end:]
            limit = at
        require(sha(current) == row['previous_sha256'],'whole predecessor differs')
        if number < 3:
            require(canonical(read('current/'+path)) == canonical(read('producer/'+path)),
                    'independent producer differs')
            require(current == canonical(read('baseline/'+path)),'independent predecessor differs')
        else:
            require(current == PREFIX.encode()+canonical(read('root-historical/root.rs')),
                    'existing historical root differs')
    before = text('root-baseline/root.rs')
    old = 'let raw_lock = std::fs::read_to_string(root().join("Cargo.lock")).unwrap();'
    new = 'let raw_lock = runtime_io::read_to_string(root().join("Cargo.lock")).unwrap();'
    require(before.count(old) == 1 and text('current/'+PATHS[-1]) == before.replace(old,new,1),
            'executing root edit differs')
    doc = text('current/docs/COMPAT.md')
    fragment = text('current/tools/source-successor-view-074/diagnostic-additions.txt')
    header = '# HydraCache Compatibility Register\n\n'
    previous = text('doc-previous/COMPAT.md')
    require(len(fragment.encode()) == 2395 and doc == text('doc-producer/COMPAT.md') and
            doc == header+fragment+previous[len(header):],'diagnostic document differs')
    require(sha(read('current/'+QUAL)) == '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc',
            'qualification differs')
    require(sha(read('current/tools/source-successor-view-074/rules.json')) ==
            '61d3e1941cdae497600592a49af0487e4cf545573ba3f091add173110b560cda','old rules differ')
    return dict(members=len(index['members']),focused_cases_per_platform=15,
                manager_cases=14,graph_controls=22,package_pass=False,release_admitted=False)

if __name__ == '__main__':
    print(json.dumps(check(),sort_keys=True))
