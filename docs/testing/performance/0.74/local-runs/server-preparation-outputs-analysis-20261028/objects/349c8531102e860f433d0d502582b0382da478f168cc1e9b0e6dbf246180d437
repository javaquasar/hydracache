"""Offline outputs/Starting decomposition from immutable first-run observations."""
import hashlib
import json
from pathlib import Path
import re

import run

INDEX = run.ROOT / 'docs/testing/performance/0.74/local-runs/server-preparation-profile-20261028/index.json'
INDEX_SHA = '16cea26f61984e9f07cf434cc17d0ad21961159cb55d61374dfb8d5d5e446b2f'
OUT = run.ROOT / 'target/server-preparation-outputs-analysis-20261028'


def enum(raw, name):
    matches = re.findall(r'enum ' + name + r'\s*\{([^}]+)\}', raw)
    run.need(len(matches) == 1, 'one source-pinned enum: ' + name)
    names = tuple(n.strip() for n in matches[0].split(',') if n.strip())
    run.need(all(re.fullmatch(r'[A-Za-z]+', n) for n in names), 'plain exact enum variants')
    return names


def nonnegative(value):
    return type(value) is int and value >= 0


def delta(before, after):
    result = {}
    for name in ('wall_ns', 'process_cpu_ns'):
        a, b = before[name], after[name]
        if name == 'process_cpu_ns' and (a is None or b is None):
            result[name] = None
        else:
            run.need(nonnegative(a) and nonnegative(b) and b >= a, 'ordered observed clock')
            result[name] = b - a
    return result


def review(phase, steps, stages):
    run.need(phase['phase'] == 'outputs' and phase['returned'] is True,
             'only a fully observed outputs phase')
    run.need(phase['instrumented'] is True and phase['guard_clock_added'] is False
             and phase['deadline_changed'] is False and phase['healthy_gate'] is False
             and phase['includes_child_cpu'] is False and phase['overlapping_spans'] is True,
             'unchanged observation semantics')
    rounds = phase['round_steps']
    run.need(len(rounds) == len(steps) == 8 and len(stages) == 9, 'exact source step/stage census')
    calls = rounds[0]['calls']
    run.need(type(calls) is int and calls > 0, 'actual complete validation rounds')
    table = []
    for index, (step, row) in enumerate(zip(steps, rounds)):
        run.need(row['step'] == index and row['calls'] == row['completed'] == calls
                 and nonnegative(row['wall_ns']) and row['guard_clock_added'] is False,
                 'missing/reordered/incomplete round')
        table.append(dict(step=step, calls=calls, wall_ns=row['wall_ns'],
                          authentication_calls=row['authentication_calls']))
    run.need(sum(r['wall_ns'] for r in table) <= phase['wall_ns'], 'disjoint step sum exceeds phase')
    marks = phase['boundaries']
    for before, after in zip(marks, marks[1:]):
        delta(before, after)
    effects, opened = [], None
    for mark in marks:
        if mark['name'] == 'st-effect':
            run.need(opened is None, 'nested effect marker')
            opened = mark
        elif mark['name'] == 'st-effected':
            run.need(opened is not None and len(effects) < len(stages), 'unpaired/extra effect marker')
            effects.append(dict(stage=stages[len(effects)], **delta(opened, mark)))
            opened = None
    run.need(opened is None and len(effects) == len(stages), 'missing effect stages')
    names = ('outputs-enter', 'outputs-starting', 'outputs-pipes', 'outputs-sinks')
    selected = []
    for name in names:
        found = [m for m in marks if m['name'] == name]
        run.need(len(found) == 1, 'unique source boundary: ' + name)
        selected.append(found[0])
    sections = [dict(section=name, **delta(a, b)) for name, a, b in zip(
        ('starting', 'pipes', 'sinks'), selected, selected[1:])]
    cpu = [e['process_cpu_ns'] for e in effects]
    return dict(validation_steps=table, storage_effects=effects, output_sections=sections,
        storage_effect_wall_ns=sum(e['wall_ns'] for e in effects),
        storage_effect_process_cpu_ns=None if any(n is None for n in cpu) else sum(cpu),
        validation_step_wall_ns=sum(s['wall_ns'] for s in table),
        source_unchanged=True, original_windows_seconds=[300, 60, 10],
        syscall_only_cost=False, parent_cpu_only=True, nested_views_overlap=True,
        performance_improvement_proven=False, original_delivery_proven=False,
        new_measurement=False, new_server_run=False, admission=False)


def main():
    run.need(run.sha(INDEX) == INDEX_SHA, 'original immutable packet index')
    index = json.loads(INDEX.read_bytes())
    held = {}
    for name, row in index['files'].items():
        raw = (INDEX.parent / 'objects' / row['sha256']).read_bytes()
        run.need(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256'],
                 'whole retained raw object: ' + name)
        held[name] = raw
    freeze = json.loads(held['capture/input/source/source-freeze.json'])
    source = {}
    for name in ('diagnostic_validation_frame.rs', 'diagnostic_collector_starting_storage.rs'):
        path = run.ROOT / 'tools/long-run-supervisor-074/src' / name
        run.need(run.sha(path) == freeze['files']['fd/tools/long-run-supervisor-074/src/' + name],
                 'actual source/order differs from compiled input')
        source[name] = path.read_text()
    rows = [json.loads(line.split('HC074-HELD-PHASE ', 1)[1]) for line in
        held['capture/collection/live.stdout.log'].decode().splitlines() if 'HC074-HELD-PHASE ' in line]
    phases = [r for r in rows if r['phase'] == 'outputs']
    run.need(len(phases) == 1, 'one complete original outputs report')
    result = review(phases[0], enum(source['diagnostic_validation_frame.rs'], 'ValidationFrameStep'),
                    enum(source['diagnostic_collector_starting_storage.rs'], 'CollectorStartingStage'))
    OUT.mkdir(exist_ok=False)
    (OUT / 'producer.py').write_bytes(Path(__file__).read_bytes())
    for name, raw in source.items():
        (OUT / name).write_text(raw, encoding='utf8', newline='\n')
    run.put(OUT / 'result.json', dict(result, original_index_sha256=INDEX_SHA,
                                   first_refusal_preserved=True))
    print(json.dumps(dict(storage_effect_wall_ns=result['storage_effect_wall_ns'],
                         validation_step_wall_ns=result['validation_step_wall_ns'],
                         new_measurement=False, admission=False)))


if __name__ == '__main__':
    main()
