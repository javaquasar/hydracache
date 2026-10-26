"""Model evidence arithmetic and operational AST, never retained-code execution."""
import ast
import hashlib
import json
from pathlib import Path

INDEX = '7f768549711cb4cdd5a0e2de8bcac1502951499c5a36e28f05c678dd483d2eb1'
PHASES = ('setup', 'spawn', 'wait', 'terminal_reads', 'decode', 'close')
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


class Strip(ast.NodeTransformer):
    def visit_Expr(self, node):
        value = node.value
        if isinstance(value, ast.Call) and isinstance(value.func, ast.Attribute) and isinstance(value.func.value, ast.Name) and value.func.value.id == 'trace':
            require(value.func.attr in ('begin', 'end') and len(value.args) == 1 and not value.keywords and
                    isinstance(value.args[0], ast.Constant) and value.args[0].value in PHASES, 'unsafe AST strip')
            return None
        return self.generic_visit(node)

    def visit_Return(self, node):
        if isinstance(node.value, ast.Call) and isinstance(node.value.func, ast.Name) and node.value.func.id == 'attach':
            require([v.id for v in node.value.args] == ['result', 'trace'] and not node.value.keywords, 'unsafe return strip')
            return ast.Return(value=ast.Name(id='result', ctx=ast.Load()))
        return self.generic_visit(node)


def verify(folder):
    raw = (folder/'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['source_commit'] == 'e6ef8937297042a2cb4f45386af360db6c9c8732' and
            (index['new_models'], index['caller_regressions'], index['accounting_regressions'], index['synthetic_cases']) == (20, 16, 20, 4)
            and index['new_workload_calls'] == 0, 'index scope differs')
    require(all(index[key] is False for key in ('native_control_executed', 'overhead_accepted', 'package_pass', 'server')),
            'model evidence promoted')
    members = {}
    for name, entry in index['members'].items():
        body = (folder/'objects'/entry['sha256']).read_bytes()
        require(len(body) == entry['bytes'] and sha(body) == entry['sha256'], 'raw member differs: '+name)
        members[name] = body
    require(len(members) == 40, 'member count differs')
    def obj(name):
        return json.loads(members[name])
    labels = ('coordinator-frames-model-first', 'coordinator-frames-model-precommit', 'coordinator-frames-caller-regressions',
              'coordinator-frames-accounting-regressions', 'coordinator-frames-syntax', 'coordinator-frames-synthetic-first')
    for label in labels:
        before, after = (obj('checks/'+label+'/'+part) for part in ('intent.json', 'result.json'))
        require(after['exit'] == 0 and after['outer_refusal'] is None and after['source_unchanged'] is True and
                after['head_unchanged'] is True and before['source'] == after['source_after'], 'check verdict/source differs')
        require(before['bound_seconds'] == 1200 and all(before[key] is False for key in ('dependency_changes',
            'observation_frequency_changed', 'services_or_clocks', 'server', 'original_owner', 'qualification', 'retry')),
            'check scope differs')
    for label, count in zip(labels[:4], (17, 20, 16, 20)):
        log = members['checks/'+label+'/stderr.log']
        require(('Ran '+str(count)+' tests').encode() in log and log.rstrip().endswith(b'OK'), 'model verdict differs')
    current = obj('checks/coordinator-frames-synthetic-first/intent.json')
    require(current['head'] == index['source_commit'], 'synthetic source commit differs')
    for name, body in members.items():
        if name.startswith('source/'):
            require(sha(body) == current['source'][name[7:]], 'source copy differs')
        elif name.startswith('raw-first-model/'):
            require(sha(body) == obj('checks/coordinator-frames-model-first/intent.json')['source'][
                'tools/xtask-package-attribution-074/'+name.split('/')[-1]], 'first model source differs')
    old_index = members['baseline/index.json']
    require(sha(old_index) == '1ba276dcf62f97801f69806ea5efba138c7d47425f30d2563874aa393681fcff', 'baseline index differs')
    require(sha(members['baseline/git_cpu_overhead.py']) == json.loads(old_index)['members'][
        'source/tools/xtask-package-attribution-074/git_cpu_overhead.py']['sha256'], 'original collector differs')
    def function(body, name):
        return next(v for v in ast.parse(body).body if isinstance(v, ast.FunctionDef) and v.name == name)
    baseline = function(members['baseline/git_cpu_overhead.py'], 'measure')
    ordinary = function(members['source/tools/xtask-package-attribution-074/git_cpu_overhead.py'], 'measure')
    traced = function(members['source/tools/xtask-package-attribution-074/coordinator_frames.py'], 'measure')
    require(traced.args.args.pop().arg == 'trace', 'traced signature differs')
    require(ast.dump(baseline) == ast.dump(ordinary) == ast.dump(Strip().visit(traced)), 'operational AST changed')
    intent, result = (obj('synthetic/'+part) for part in ('intent.json', 'result.json'))
    require(intent['new_workload_calls'] == result['new_workload_calls'] == 0 and result['passed'] is True and
            intent['native_control_executed'] is result['native_control_executed'] is False and
            intent['package_pass'] is result['package_pass'] is False, 'synthetic evidence promoted')
    cases = result['cases']
    require([v['name'] for v in cases] == ['healthy', 'clock-fault', 'dual-refusal', 'interrupted'] and
            [v['clock_reads'] for v in cases] == [36, 8, 1, 9] and
            [v['result']['passed'] for v in cases] == [True, False, False, False], 'synthetic schedule/verdict differs')
    for case in cases:
        require(case['modeled_operation_only'] is True, 'synthetic operation promoted')
        tape = case['result']['coordinator_frames']
        contract = tape['contract']
        require(tape['clock_domain'] == 'synthetic-control' and contract['phases'] == list(PHASES) and
                (contract['max_probes'], contract['max_clock_reads']) == (12, 36) and len(tape['probes']) <= 12,
                'bounded clock contract differs')
        require(all(contract[key] is False for key in ('existing_predicates_removed_or_reordered', 'window_renewed',
            'complete_frame_partition', 'cpu_is_native_git', 'overhead_accepted')), 'contract promoted')
        for span in tape['spans']:
            if span['status'] != 'complete':
                require(span['elapsed'] is None and span['end_probe'] is None, 'invented incomplete span')
                continue
            a, b = (tape['probes'][i]['raw'] for i in (span['start_probe'], span['end_probe']))
            require(span['elapsed'] == dict(wall_lower_ns=b['wall_before_ns']-a['wall_after_ns'],
                wall_upper_ns=b['wall_after_ns']-a['wall_before_ns'], process_cpu_ns=b['process_cpu_ns']-a['process_cpu_ns']),
                'synthetic bracket arithmetic differs')
    require(cases[0]['result']['coordinator_frames']['complete'] is True, 'healthy synthetic frame incomplete')
    require(cases[1]['result']['operation_verdict']['passed'] is True and
            cases[1]['result']['coordinator_frames']['capture_valid'] is False, 'clock refusal promoted/operation changed')
    require(cases[2]['result']['first_refusal'] == 'Refusal: synthetic original operation' and
            'synthetic first clock fault' in cases[2]['result']['coordinator_frames']['first_refusal'], 'first refusal replaced')
    require(cases[3]['result']['coordinator_frames']['capture_valid'] is True and
            cases[3]['result']['coordinator_frames']['complete'] is False, 'interruption promoted')
    return dict(models=56, synthetic_examples=4, operational_ast_preserved=True,
                native_control_executed=False, new_workload_calls=0, overhead_accepted=False, package_pass=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
