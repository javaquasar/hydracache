"""Single authorized configuration-only localization; no trace activation/query."""
import argparse
import json
import os
from pathlib import Path
import select
import subprocess
import traceback
import uuid

import run as base

OUT = base.ROOT / 'target/wsl-clock-einval-20261023'
ARCHIVE = base.ROOT / 'docs/testing/performance/0.74/local-runs/wsl-clock-einval-20261023'
FIRST = 'local-runs/wsl-clock-trace-20261023/index.json'
FIRST_SHA = '89912e476413847377bed1f22d1e52c799d2ded988fd3f861ecfedc56960da03'


class Journal:
    def __init__(self, path):
        self.path = path
        self.number = 0

    def record(self, value):
        with self.path.open('a', encoding='utf-8', newline='\n') as stream:
            stream.write(json.dumps(value, sort_keys=True) + '\n')
            stream.flush()
            os.fsync(stream.fileno())

    def perform(self, kind, path, value, effect):
        self.number += 1
        item = {'sequence': self.number, 'operation': kind, 'path': str(path), 'value': value}
        self.record(item | {'phase': 'before'})  # Must be durable before effect.
        try:
            result = effect()
        except BaseException as error:
            self.record(item | {'phase': 'failed', 'error': repr(error),
                                'errno': getattr(error, 'errno', None), 'traceback': traceback.format_exc()})
            raise
        self.record(item | {'phase': 'after'})
        return result


def configure(cap, group, planned, journal):
    """Same ordered configuration as the first candidate, stop before tracing_on=1."""
    instance = cap.TRACE / 'instances' / group
    journal.perform('mkdir', instance, None, instance.mkdir)

    def write(path, value, append=False):
        journal.perform('append' if append else 'write', path, value,
                        lambda: cap.write(path, value, append=append))

    write(instance / 'tracing_on', '0')
    write(instance / 'buffer_size_kb', '256')
    write(instance / 'trace_clock', 'mono_raw')
    write(instance / 'options/stacktrace', '1')
    for definition in planned:
        write(cap.TRACE / 'kprobe_events', definition, append=True)
        event = cap.key(definition).split('/')[1]
        if event == 'adjust':
            write(instance / 'events' / group / event / 'filter', 'modes != 0')
        write(instance / 'events' / group / event / 'enable', '1')
    actual = journal.perform('read', cap.TRACE / 'kprobe_events', None,
                             lambda: (cap.TRACE / 'kprobe_events').read_text().splitlines())
    if {cap.normalized(s) for s in actual} != {cap.normalized(s) for s in planned}:
        raise RuntimeError('Probe registration differs from exact plan')
    for definition in planned:
        path = instance / 'events' / group / cap.key(definition).split('/')[1] / 'format'
        journal.perform('read', path, None, path.read_text)
    if (instance / 'tracing_on').read_text().strip() != '0':
        raise RuntimeError('Private trace unexpectedly enabled')


def session():
    import capture as cap  # Linux-only import; Windows controller never imports it.
    cap.OUT = OUT
    if not OUT.exists() or (OUT / 'session-intent.json').exists():
        raise FileExistsError('Exclusive configuration lane already consumed or absent')
    initial = cap.preflight()  # Unchanged BTF/helper/ABI/symbol/root-state guards.
    base.put(OUT / 'preflight.json', initial)
    group = 'hc074clock_' + uuid.uuid4().hex
    planned = cap.definitions(group)
    started = cap.raw_ns()
    base.put(OUT / 'session-intent.json', {'group': group, 'definitions': planned,
        'configuration_only': True, 'activate_buffer': False, 'clock_queries': 0,
        'guardian_seconds_raw': 120, 'original_windows': [300, 60, 10]})
    owner = os.pidfd_open(os.getpid())
    cr, cw = os.pipe()
    rr, rw = os.pipe()
    child = os.fork()
    if child == 0:
        os.close(cw)
        os.close(rr)
        cap.guardian(owner, cr, rw, group, planned, started + 120_000_000_000)
    os.close(cr)
    os.close(rw)
    os.close(owner)
    failure = None
    try:
        if not select.select([rr], [], [], 3)[0] or os.read(rr, 1) != b'R':
            raise RuntimeError('Guardian not armed; no instrumentation allowed')
        with cap.serialized():
            configure(cap, group, planned, Journal(OUT / 'operations.jsonl'))
        base.put(OUT / 'configuration-complete.json', {'complete': True, 'activated': False})
    except BaseException as error:
        failure = {'error': repr(error), 'errno': getattr(error, 'errno', None),
                   'filename': getattr(error, 'filename', None), 'traceback': traceback.format_exc()}
        base.put(OUT / 'first-failure.json', failure)
    finally:
        # Preserve partial state before deleting only our exact definitions.
        paths = {'root_error': cap.TRACE / 'error_log',
                 'private_error': cap.TRACE / 'instances' / group / 'error_log',
                 'registered_probes': cap.TRACE / 'kprobe_events'}
        base.put(OUT / 'partial-state.json', {name: p.read_text() if p.exists() else None for name, p in paths.items()})
        with cap.serialized():
            own = cap.cleanup(group, planned)
        base.put(OUT / 'cleanup-owner.json', own)
        try:
            os.write(cw, b'D')
        except BrokenPipeError:
            failure = failure or {'error': 'Guardian control closed before completion'}
        os.close(cw)
        os.waitpid(child, 0)
        after = cap.snapshot()
        guard = json.loads((OUT / 'guardian.json').read_text())
        post = {'root': after, 'root_controls_equal': after == initial['root'],
                'owner_cleanup': own['ok'], 'guardian_cleanup': guard['ok'],
                'boot_unchanged': Path('/proc/sys/kernel/random/boot_id').read_text().strip() == initial['boot'],
                'pid1_start_unchanged': cap.pid1_start(Path('/proc/1/stat').read_text()) == cap.pid1_start(initial['pid1_stat']),
                'time_namespace_unchanged': os.readlink('/proc/self/ns/time') == initial['time_namespace']}
        base.put(OUT / 'postflight.json', post)
        if not all(v is True for k, v in post.items() if k != 'root'):
            raise RuntimeError('Unconfirmed cleanup/context; retain and stop')
    if failure:
        raise RuntimeError('First configuration refusal retained; no retry')


def configure_archive():
    base.OUT, base.ARCHIVE = OUT, ARCHIVE
    base.PLAN = base.ROOT / 'docs/testing/performance/0.74/wsl-clock-einval-plan-20261023.md'
    base.FROZEN = base.FROZEN | {FIRST: FIRST_SHA}


def collect():
    configure_archive()
    if subprocess.check_output(['git', 'status', '--porcelain'], cwd=base.ROOT).strip():
        raise ValueError('Commit reviewed configuration-only candidate first')
    proof = base.sources()
    OUT.mkdir(parents=True, exist_ok=False)
    base.put(OUT / 'intent.json', {'sources': proof, 'frozen_inputs': base.FROZEN,
        'git': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=base.ROOT, text=True).strip(),
        'configuration_only': True, 'clock_queries': 0, 'trace_activation': False,
        'authorization': 'user approved one separate limited local WSL session to localize EINVAL',
        'retry': False, 'original_windows': [300, 60, 10],
        'clock_change': False, 'service_change': False, 'original_owner_run': False,
        'server': False, 'product': False, 'admission': False})
    base.command('controls', ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec',
        'python3', '-B', '-m', 'unittest', 'discover', '-s', base.linux(base.HERE), '-v'], 45)
    try:
        base.command('session', ['wsl.exe', '-d', 'Ubuntu', '-u', 'root', '--exec',
            'python3', '-B', base.linux(Path(__file__)), 'session'], 180)
    finally:
        if proof != base.sources():
            raise ValueError('Source or frozen-input drift')
        # Analyze configuration results even on a first refusal; never redispatch.
        if (OUT / 'postflight.json').exists():
            steps = [json.loads(s) for s in (OUT / 'operations.jsonl').read_text().splitlines()]
            post = json.loads((OUT / 'postflight.json').read_text())
            base.put(OUT / 'analysis.json', {'method': 'hc074-clock-configuration-localization-v1',
                'configuration_only': True, 'clock_queries': 0, 'trace_activation': False,
                'failed_operations': [s for s in steps if s['phase'] == 'failed'],
                'completed_operations': sum(s['phase'] == 'after' for s in steps),
                'cleanup_context_confirmed': all(v is True for k, v in post.items() if k != 'root'),
                'admission': False, 'server': False, 'original_owner_run': False})


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('collect', 'session', 'retain', 'verify'))
    mode = parser.parse_args().mode
    if mode in ('retain', 'verify'):
        configure_archive()
        getattr(base, mode)()
    else:
        {'collect': collect, 'session': session}[mode]()
