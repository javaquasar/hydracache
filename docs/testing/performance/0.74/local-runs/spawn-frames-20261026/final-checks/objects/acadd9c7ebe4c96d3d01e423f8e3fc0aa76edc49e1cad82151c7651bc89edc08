"""Verify fixed receipts and source AST, without importing retained code."""
import ast
import hashlib
import json
from pathlib import Path
import tomllib

INDEX = 'a518934e04231820823c0535c5865c4fe5e19794694fc2b787ea6dc102a7de88'
SOURCE = '7a6556aa0f04f4e2cf0d716968e5eab36f2cc529'
PHASES = ('preparation', 'create_to_retention', 'membership_release')
PREFIX = 'source/tools/xtask-package-attribution-074/'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def load(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and index['checks'] == 7 and len(index['members']) == 42 and
            index['native_attempts'] == index['new_workload_calls'] == 0 and all(index[k] is False for k in
            ('final_checks', 'caller_connected', 'native_control_executed', 'overhead_accepted', 'package_pass', 'server')),
            'scope differs')
    members = {}
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'raw member differs')
        members[name] = body
    return members


class Strip(ast.NodeTransformer):
    def visit_Expr(self, node):
        call = node.value
        if isinstance(call, ast.Call) and isinstance(call.func, ast.Attribute):
            receiver = call.func.value
            if isinstance(receiver, ast.Attribute) and receiver.attr == 'spawn_frames' and \
                    isinstance(receiver.value, ast.Name) and receiver.value.id in ('owner', 'self'):
                require(call.func.attr in ('begin', 'end') and not call.keywords and len(call.args) == 1 and
                        isinstance(call.args[0], ast.Constant) and call.args[0].value in PHASES,
                        'unsupported stripped mark')
                return None
        return self.generic_visit(node)


def method(raw, owner, name):
    cls = next(n for n in ast.parse(raw).body if isinstance(n, ast.ClassDef) and n.name == owner)
    return next(n for n in cls.body if isinstance(n, ast.FunctionDef) and n.name == name)


def compare_bodies(old_api, new_api, old_owner, new_owner):
    for a, b, name in ((method(old_api, 'Api', 'spawn'), method(new_api, 'SpawnAccountingApi', 'spawn'), 'API'),
                      (method(old_owner, 'LeafAccountingJob', 'child'), method(new_owner, 'SpawnAccountingJob', 'child'), 'owner')):
        b.decorator_list = a.decorator_list
        require(ast.dump(a) == ast.dump(Strip().visit(b)), name+' operational AST differs')
    # Check physical source placement as well as stripped equivalence.
    api = method(new_api, 'SpawnAccountingApi', 'spawn')
    create = next(n for n in ast.walk(api) if isinstance(n, ast.If) and isinstance(n.test, ast.UnaryOp) and
                  isinstance(n.test.operand, ast.Call) and isinstance(n.test.operand.func, ast.Attribute) and
                  n.test.operand.func.attr == 'CreateProcessW')
    marks = [n for n in ast.walk(api) if isinstance(n, ast.Expr) and isinstance(n.value, ast.Call) and
             isinstance(n.value.func, ast.Attribute) and isinstance(n.value.func.value, ast.Attribute) and
             n.value.func.value.attr == 'spawn_frames']
    require(not any(create.lineno <= n.lineno <= create.end_lineno for n in marks), 'create/error path has new mark')
    require([(n.value.func.attr, n.value.args[0].value) for n in sorted(marks, key=lambda n: n.lineno)] ==
            [('begin', 'preparation'), ('end', 'preparation'), ('begin', 'create_to_retention'),
             ('begin', 'membership_release'), ('end', 'membership_release')], 'API mark schedule differs')
    body = next(n.body for n in api.body if isinstance(n, ast.Try))
    position = body.index(create)
    require(isinstance(body[position+1], ast.Assign) and ast.unparse(body[position+1]).startswith('child = WindowsChild(') and
            ast.unparse(body[position+2]) == 'owner.child = child', 'successful create/publication gap has new mark')
    child = method(new_owner, 'SpawnAccountingJob', 'child')
    assign = next(n for n in child.body if isinstance(n, ast.Assign) and isinstance(n.targets[0], ast.Attribute)
                  and n.targets[0].attr == '_child')
    mark = child.body[-1].body[0]
    require(isinstance(mark, ast.Expr) and mark.lineno > assign.lineno and
            ast.unparse(mark) == "self.spawn_frames.end('create_to_retention')", 'probe precedes retention')


def audit(members):
    def obj(name):
        return json.loads(members[name])
    native = 'baseline/capture-leaves-native-20261026-index.json'
    require(sha(members[native]) == '926f30f0eca6ae708b3706976f256a9ca52a3a5d7e9a9c447a8f45cec59a0855', 'native index differs')
    require(sha(members['baseline/leaf-attribution-20261026-index.json']) ==
            '70ba30460b8e4c995c23d3f37090b0ef849bc2915e6507e62dfc5b23a7837417', 'first offline history differs')
    native = obj(native)
    for path in ('scripts/ci/watchdog_windows_job.py', 'crates/xtask/tests/git_descendant_cpu_074.rs',
                 'tools/xtask-package-attribution-074/git_cpu_overhead.py',
                 'tools/xtask-package-attribution-074/capture_leaves.py'):
        require(sha(members['source/'+path]) == native['members']['source/'+path]['sha256'], 'original source changed')
    labels = ('spawn-frames-models-first', 'spawn-frames-models-extended', 'spawn-frames-models-final',
              'spawn-frames-regressions', 'spawn-frames-syntax', 'spawn-frames-check', 'spawn-frames-clippy')
    final = None
    for i, label in enumerate(labels):
        before, after = (obj('checks/'+label+'/'+part) for part in ('intent.json', 'result.json'))
        require(before['head'] == '19918fbc758a37334fee26991dee509feecf7e6d' and after['exit'] == 0 and
                after['outer_refusal'] is None and after['source_unchanged'] is after['head_unchanged'] is True and
                before['source'] == after['source_after'], 'check receipt differs')
        require(before['bound_seconds'] == 1200 and before['disk_floor_bytes'] == 4*1024**3 and all(before[k] is False for k in
            ('dependency_changes', 'observation_frequency_changed', 'services_or_clocks', 'server', 'qualification', 'retry')),
            'check scope differs')
        if i == 2:
            final = before['source']
        if i >= 2:
            require(before['source'] == final, 'final implementation inputs differ')
    for name, raw in members.items():
        if name.startswith('source/'):
            require(sha(raw) == final[name[7:]], 'final source binding differs')
    require(b'Ran 11 tests' in members['checks/spawn-frames-models-first/stderr.log'] and
            b'Ran 14 tests' in members['checks/spawn-frames-models-extended/stderr.log'] and
            b'Ran 15 tests' in members['checks/spawn-frames-models-final/stderr.log'] and
            b'Ran 130 tests' in members['checks/spawn-frames-regressions/stderr.log'], 'test stage count differs')
    require(sha(members['qualification-manifest.toml']) ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification changed')
    protocol = tomllib.loads(members[PREFIX+'spawn-frames-protocol.toml'].decode())
    require(protocol['schema'] == 'hc074-spawn-frames-model-v1' and protocol['phases'] == list(PHASES) and
            (protocol['max_additional_probes_per_worker'], protocol['max_additional_clock_reads_per_worker'],
             protocol['max_recorder_clock_reads_with_outer_and_leaves']) == (6, 18, 102) and
            (protocol['bound_seconds'], protocol['cleanup_seconds'], protocol['free_bytes'], protocol['log_bytes']) ==
            (120, 5, 4*1024**3, 16*1024**2), 'protocol/bounds differ')
    require(all(protocol[k] is False for k in ('cleanup_instrumented', 'pure_create_cost_isolated', 'caller_connected',
        'native_control_executed', 'existing_predicates_removed_or_reordered', 'window_renewed', 'overhead_accepted',
        'package_pass', 'server_services_clocks_changed')) and protocol['complete_frame_does_not_prove_resume'] is True,
        'protocol promoted')
    compare_bodies(members['source/scripts/ci/watchdog_windows_job.py'], members[PREFIX+'spawn_frames.py'],
                   members[PREFIX+'capture_leaves.py'], members[PREFIX+'spawn_frames.py'])
    return dict(checks=7, new_models=15, combined_cases=130, additional_clock_positions=18,
                ordinary_and_fatal_model_cuts=36, operational_bodies_equal=True, native_attempts=0,
                new_workload_calls=0, caller_connected=False, overhead_accepted=False, package_pass=False)


def verify(folder):
    return audit(load(folder))


if __name__ == '__main__':
    print(json.dumps(verify(Path(__file__).resolve().parent), sort_keys=True))
