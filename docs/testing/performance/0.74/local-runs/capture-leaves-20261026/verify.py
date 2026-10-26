"""Independent source/receipt checks, not execution of retained telemetry."""
import ast
import hashlib
import json
from pathlib import Path
import tomllib

INDEX = 'd539ff2fa93959cf1fbb176e4994ebb5d7f83c5e141a0e1bae81f672f0cb2000'
SOURCE = '8a0a10cab2a87da519e882838b11645d702fb5e9'
GROUPS = dict(owner=('owner_identity', 'owner_initial'),
              decode=('log_stat', 'log_read', 'text_decode', 'record_parse', 'protocol_validation', 'result_join'))
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


class Strip(ast.NodeTransformer):
    def visit_Expr(self, node):
        call = node.value
        if isinstance(call, ast.Call) and isinstance(call.func, ast.Attribute):
            v = call.func.value
            leaf = isinstance(v, ast.Name) and v.id == 'leaves'
            leaf |= isinstance(v, ast.Attribute) and isinstance(v.value, ast.Name) and v.value.id == 'self' and v.attr == 'leaves'
            if leaf:
                require(call.func.attr in ('begin', 'end') and len(call.args) == 2 and not call.keywords and
                    all(isinstance(a, ast.Constant) for a in call.args), 'unsafe leaf mark strip')
                require(call.args[0].value in GROUPS and call.args[1].value in GROUPS[call.args[0].value], 'unknown leaf strip')
                return None
        return self.generic_visit(node)

    def visit_Call(self, node):
        if isinstance(node.func, ast.Name) and node.func.id in ('LeafAccountingJob', 'leaf_rows', 'attach_leaves'):
            name = node.func.id
            require(not node.keywords and len(node.args) == {'LeafAccountingJob': 4, 'leaf_rows': 3, 'attach_leaves': 2}[name]
                    and isinstance(node.args[-1], ast.Name) and node.args[-1].id == 'leaves', 'unsafe leaf call strip')
            node.args.pop()
            if name == 'attach_leaves':
                return self.visit(node.args[0])
            node.func.id = {'LeafAccountingJob': 'AccountingJob', 'leaf_rows': 'rows'}[name]
        return self.generic_visit(node)


def function(raw, name, cls=None, *, setter=False):
    body = ast.parse(raw).body
    if cls is not None:
        body = next(n for n in body if isinstance(n, ast.ClassDef) and n.name == cls).body
    return next(n for n in body if isinstance(n, ast.FunctionDef) and n.name == name and
                (not setter or any(isinstance(d, ast.Attribute) and d.attr == 'setter' for d in n.decorator_list)))


def verify(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and (index['models'], index['regressions'], index['first_model_tests'],
        index['first_model_errors'], index['new_workload_calls']) == (18, 56, 12, 1, 0) and
        all(index[k] is False for k in ('native_control_executed', 'overhead_accepted', 'package_pass', 'server')), 'scope differs')
    require(len(index['members']) == 63, 'member count differs')
    members = {}
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'raw member differs: '+name)
        members[name] = body
    def obj(name):
        return json.loads(members[name])
    labels = ('capture-leaves-model-first', 'capture-leaves-model-precommit', 'capture-leaves-model-final',
              'capture-leaves-model-closure', 'capture-leaves-regressions-first', 'capture-leaves-syntax-first',
              'capture-leaves-check-first', 'capture-leaves-clippy-first', 'capture-leaves-postcommit')
    for label in labels:
        a, b = (obj('checks/'+label+'/'+part) for part in ('intent.json', 'result.json'))
        require(b['exit'] == (1 if label == labels[0] else 0) and b['outer_refusal'] is None and
                b['source_unchanged'] is b['head_unchanged'] is True and a['source'] == b['source_after'] and
                a['bound_seconds'] == 1200 and a['disk_floor_bytes'] == 4*1024**3 and
                all(a[k] is False for k in ('dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'qualification', 'retry')), 'check verdict/source differs')
    for label, count in zip(labels[:5]+labels[-1:], (12, 15, 17, 18, 56, 74)):
        log = members['checks/'+label+'/stderr.log']
        require(f'Ran {count} tests'.encode() in log, 'test count differs')
        if label == labels[0]:
            require(b'FAILED (errors=1)' in log and b'original worker identity drift' in log, 'first fixture refusal lost')
        else:
            require(log.rstrip().endswith(b'OK'), 'passing model verdict differs')
    post = obj('checks/capture-leaves-postcommit/intent.json')
    require(post['head'] == SOURCE, 'postcommit source differs')
    for name, body in members.items():
        if name.startswith('source/'):
            require(sha(body) == post['source'][name[7:]], 'final source copy differs')
    for label, check in zip(('first12', 'pre15', 'final17'), labels[:3]):
        before = obj('checks/'+check+'/intent.json')
        for name in ('capture_leaves.py', 'test_capture_leaves.py', 'git_cpu_overhead.py'):
            require(sha(members['history/'+label+'/'+name]) == before['source']['tools/xtask-package-attribution-074/'+name],
                    'historical source copy differs')
    baseline_raw = members['baseline/index.json']
    require(sha(baseline_raw) == '55b802621e147681f9478a164053a04c055ec28d8d35227d68d49ad0b4de380f', 'baseline index differs')
    old = json.loads(baseline_raw)
    for name, body in members.items():
        if name.startswith('baseline/source/'):
            key = name[len('baseline/'):]
            require(sha(body) == old['members'][key]['sha256'], 'original body differs')
            if key in members and key != 'source/tools/xtask-package-attribution-074/git_cpu_overhead.py':
                require(body == members[key], 'original dependency changed')
    original = members['baseline/source/tools/xtask-package-attribution-074/git_cpu_overhead.py']
    require(ast.dump(function(original, 'measure')) ==
            ast.dump(function(members['source/tools/xtask-package-attribution-074/git_cpu_overhead.py'], 'measure')), 'default frame changed')
    outer = members['source/tools/xtask-package-attribution-074/coordinator_frames.py']
    job = members['source/tools/xtask-package-attribution-074/git_job_cpu.py']
    for source in (members['source/tools/xtask-package-attribution-074/capture_leaves.py'],
                   *(members['history/'+p+'/capture_leaves.py'] for p in ('first12', 'pre15', 'final17'))):
        for old_body, old_name, name in ((outer, 'measure', 'measure'), (job, 'rows', 'leaf_rows')):
            a, b = function(old_body, old_name), function(source, name)
            require(b.args.args.pop().arg == 'leaves', 'leaf signature differs')
            b.name = a.name
            require(ast.dump(a) == ast.dump(Strip().visit(b)), 'operational AST differs')
        a, b = function(job, 'child', 'AccountingJob', setter=True), function(source, 'child', 'LeafAccountingJob', setter=True)
        require(ast.dump(b.decorator_list[0]) == ast.dump(ast.parse('AccountingJob.child.setter', mode='eval').body), 'owner descriptor differs')
        b.decorator_list = a.decorator_list
        require(ast.dump(a) == ast.dump(Strip().visit(b)), 'owner setter AST differs')
    require(sha(members['baseline/source/tools/xtask-package-attribution-074/pipe_capture.rs']) ==
        post['source']['tools/xtask-package-attribution-074/pipe_capture.rs'], 'worker capture helper changed')
    protocol = tomllib.loads(members['source/tools/xtask-package-attribution-074/capture-leaves-protocol.toml'].decode())
    require(protocol['schema'] == 'hc074-capture-leaves-v1' and protocol['owner_phases'] == list(GROUPS['owner']) and
        protocol['decode_phases'] == list(GROUPS['decode']) and
        (protocol['max_additional_probes_per_worker'], protocol['max_additional_clock_reads_per_worker'],
         protocol['max_clock_reads_including_outer_per_worker'], protocol['bound_seconds'], protocol['cleanup_seconds']) ==
        (16, 48, 84, 120, 5) and protocol['native_control_executed'] is False, 'protocol differs')
    require(sha(members['qualification-manifest.toml']) ==
        '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification changed')
    return dict(models=74, first_fixture_refusal_preserved=True, operational_ast_preserved=True,
                native_control_executed=False, new_workload_calls=0, overhead_accepted=False, package_pass=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
