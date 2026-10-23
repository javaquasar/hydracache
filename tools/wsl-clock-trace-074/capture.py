"""One bounded local WSL trace. Only uniquely owned tracing state is modified."""
import contextlib
import errno
import fcntl
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import time
import traceback
import uuid

import btf

TRACE = Path('/sys/kernel/tracing')
OUT = Path('/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/wsl-clock-trace-20261023')
HELPER = OUT.parent / 'wsl-clock-causality-root-context-20261023/discipline-correlation'
HELPER_SHA = 'f3e81c0e8080969e00ead542269566b6e079e3eacc586e771236a65f413180dc'
BTF_SHA = '7a56bbf53f875fcf5c09d907b178bf15e6ee77748289532a85435e74222fa00a'
ROOT_FILES = ('tracing_on', 'current_tracer', 'trace_clock', 'trace_options',
              'set_event', 'kprobe_events', 'dynamic_events', 'events/enable')


def raw_ns():
    return time.clock_gettime_ns(time.CLOCK_MONOTONIC_RAW)


def pid1_start(stat):
    return int(stat[stat.rindex(')') + 2:].split()[19])


def publish(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def definitions(group):
    if not group.startswith('hc074clock_') or len(group) != 43 or any(c not in '0123456789abcdef' for c in group[11:]):
        raise ValueError('Not an owned random trace group')
    return [f'p:{group}/adjust do_adjtimex modes=+0(%di):u32 freq=+16(%di):s64 tick=+88(%di):s64 sec=+72(%di):s64 ns=+80(%di):s64',
            f'r64:{group}/adjust_return do_adjtimex retval=$retval:s64',
            f'p:{group}/settime do_settimeofday64 sec=+0(%di):s64 ns=+8(%di):s64',
            f'r64:{group}/settime_return do_settimeofday64 retval=$retval:s64',
            f'p:{group}/offset timekeeping_inject_offset sec=+0(%di):s64 ns=+8(%di):s64',
            f'r64:{group}/offset_return timekeeping_inject_offset retval=$retval:s64',
            f'p:{group}/host hv_set_host_time']


def key(definition):
    return definition.split()[0].split(':', 1)[1]


def normalized(definition):
    return ' '.join(definition.split())


def snapshot(root=TRACE):
    return {name: (root / name).read_text() for name in ROOT_FILES} | {
        'instances': sorted(p.name for p in (root / 'instances').iterdir())}


def append_command(path, value):
    """One control command, no implicit seek, creation, truncation or resubmit."""
    if not value or '\n' in value or '\r' in value:
        raise ValueError('Expected exactly one nonempty registry command')
    raw = (value + '\n').encode('ascii')
    fd = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_CLOEXEC)
    try:
        written = os.write(fd, raw)
        if written != len(raw):
            raise OSError(errno.EIO, f'Short registry command write: {written}/{len(raw)} bytes')
    except BaseException as first:
        try:
            os.close(fd)
        except OSError as closing:
            first.add_note(f'Registry close also failed: {closing!r}')
        raise
    else:
        os.close(fd)


def write(path, value, append=False):
    # Never truncate kprobe_events: append registration/deletion commands only.
    try:
        if append:
            append_command(path, value)
        else:
            with path.open('w', encoding='ascii') as stream:
                stream.write(value + '\n')
    except OSError as error:
        # Buffered write/close errors often omit filename. Retain the exact path.
        raise OSError(error.errno, error.strerror, str(path)) from error


def cleanup(group, planned, root=TRACE):
    """Idempotent cleanup of this nonce, never restoration of global config files."""
    instance = root / 'instances' / group
    problems = []
    if instance.exists():
        try:
            write(instance / 'tracing_on', '0')
            for definition in planned:
                event = key(definition).split('/')[1]
                enable = instance / 'events' / group / event / 'enable'
                if enable.exists():
                    write(enable, '0')
        except OSError as error:
            problems.append(repr(error))
    current = {key(line): normalized(line) for line in (root / 'kprobe_events').read_text().splitlines()}
    for definition in reversed(planned):
        name = key(definition)
        if name not in current:
            continue
        if current[name] != normalized(definition):
            problems.append('Refuse changed probe definition: ' + name)
            continue
        try:
            write(root / 'kprobe_events', '-:' + name, append=True)
        except OSError as error:
            problems.append(repr(error))
    if instance.exists():
        try:
            instance.rmdir()  # tracefs removes only this owned instance; never recursive.
        except OSError as error:
            problems.append(repr(error))
    remaining = [line for line in (root / 'kprobe_events').read_text().splitlines()
                 if key(line).startswith(group + '/')]
    return {'ok': not problems and not instance.exists() and not remaining,
            'problems': problems, 'remaining_owned_probes': remaining,
            'instance_absent': not instance.exists()}


@contextlib.contextmanager
def serialized():
    with (OUT / 'cleanup.lock').open('a') as stream:
        fcntl.flock(stream, fcntl.LOCK_EX)
        yield


def guardian(owner_fd, control, ready, group, planned, deadline):
    os.setsid()
    # Disconnect output from the Windows controller without creating a service.
    with open(os.devnull, 'r+b', buffering=0) as null:
        for fd in (0, 1, 2):
            os.dup2(null.fileno(), fd)
    poll = select.poll()
    poll.register(owner_fd, select.POLLIN)
    poll.register(control, select.POLLIN | select.POLLHUP)
    os.write(ready, b'R')
    os.close(ready)
    reason = 'hard_deadline'
    while raw_ns() < deadline:
        events = poll.poll(100)
        if events:
            reason = 'owner_exit_or_control_close'
            if any(fd == control for fd, _ in events) and os.read(control, 1) == b'D':
                reason = 'normal_completion'
            break
    with serialized():
        result = cleanup(group, planned)
    result['reason'] = reason
    publish(OUT / 'guardian.json', result)
    os._exit(0 if result['ok'] else 1)


def preflight():
    if os.geteuid() != 0 or os.uname().machine != 'x86_64' or os.uname().release != '6.6.87.2-microsoft-standard-WSL2':
        raise ValueError('Only the pinned local Linux x86_64 kernel')
    raw = Path('/sys/kernel/btf/vmlinux').read_bytes()
    if hashlib.sha256(raw).hexdigest() != BTF_SHA:
        raise ValueError('Pinned kernel BTF changed')
    layout = btf.layouts(raw)
    if hashlib.sha256(HELPER.read_bytes()).hexdigest() != HELPER_SHA:
        raise ValueError('Read-only helper changed')
    symbols = {line.split()[0] for line in (TRACE / 'available_filter_functions').read_text().splitlines()}
    if not {'do_adjtimex', 'do_settimeofday64', 'timekeeping_inject_offset', 'hv_set_host_time'} <= symbols:
        raise ValueError('Missing exact target symbols')
    before = snapshot()
    if before['kprobe_events'].strip() or before['dynamic_events'].strip() or before['instances'] or before['set_event'].strip():
        raise ValueError('Existing instrumentation: leave it untouched')
    if before['current_tracer'].strip() != 'nop' or before['events/enable'].strip() != '0':
        raise ValueError('Existing root tracer: leave it untouched')
    return {'btf_sha256': hashlib.sha256(raw).hexdigest(), 'layouts': layout,
            'root': before, 'boot': Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
            'pid1_stat': Path('/proc/1/stat').read_text(),
            'time_namespace': os.readlink('/proc/self/ns/time')}


def collect():
    # The Windows launcher creates this directory/intent exclusively beforehand.
    if OUT.exists() is False or (OUT / 'trace-intent.json').exists():
        raise FileExistsError('No exclusive fresh trace lane')
    initial = preflight()
    publish(OUT / 'preflight.json', initial)
    group = 'hc074clock_' + uuid.uuid4().hex
    planned = definitions(group)
    instance = TRACE / 'instances' / group
    started = raw_ns()
    publish(OUT / 'trace-intent.json', {'group': group, 'definitions': planned,
            'capture_seconds_raw': 90, 'guardian_deadline_seconds_raw': 120,
            'buffer_kb_per_cpu': 256, 'maximum_trace_text_bytes': 16 * 1024 * 1024,
            'owner_pid': os.getpid(), 'owner_start_stat': Path('/proc/self/stat').read_text(),
            'read_only_helper_sha256': HELPER_SHA, 'original_owner_run': False,
            'clock_change': False, 'service_change': False, 'server': False,
            'root_global_controls_will_not_be_restored_by_overwrite': True})
    owner_fd = os.pidfd_open(os.getpid())
    control_r, control_w = os.pipe()
    ready_r, ready_w = os.pipe()
    child = os.fork()
    if child == 0:
        os.close(control_w)
        os.close(ready_r)
        guardian(owner_fd, control_r, ready_w, group, planned, started + 120_000_000_000)
    os.close(control_r)
    os.close(ready_w)
    os.close(owner_fd)
    failure = None
    try:
        if not select.select([ready_r], [], [], 3)[0] or os.read(ready_r, 1) != b'R':
            raise RuntimeError('Guardian not armed; do not touch tracing')
        with serialized():
            instance.mkdir()
            write(instance / 'tracing_on', '0')
            write(instance / 'buffer_size_kb', '256')
            write(instance / 'trace_clock', 'mono_raw')
            write(instance / 'options/stacktrace', '1')
            for definition in planned:
                write(TRACE / 'kprobe_events', definition, append=True)
                event = key(definition).split('/')[1]
                if event == 'adjust':
                    write(instance / 'events' / group / event / 'filter', 'modes != 0')
                write(instance / 'events' / group / event / 'enable', '1')
            actual = (TRACE / 'kprobe_events').read_text().splitlines()
            if {normalized(s) for s in actual} != {normalized(s) for s in planned}:
                raise RuntimeError('Probe registration differs from exact plan')
            publish(OUT / 'probe-formats.json', {key(s): (instance / 'events' / group / key(s).split('/')[1] / 'format').read_text() for s in planned})
            write(instance / 'tracing_on', '1')
        capture_start = raw_ns()
        deadline = capture_start + 90_000_000_000
        count = 0
        with (OUT / 'samples.jsonl').open('x') as samples:
            while raw_ns() < deadline:
                if count >= 200:
                    raise ValueError('Fixed query-count cap exceeded')
                query = subprocess.run(['runuser', '-u', 'artur', '--', str(HELPER)],
                                       capture_output=True, text=True, check=True, timeout=3)
                state = json.loads(query.stdout)
                if state['modes'] != 0 or state['euid'] != 1000:
                    raise ValueError('Not the original read-only helper context')
                samples.write(json.dumps(state) + '\n')
                samples.flush()
                count += 1
                time.sleep(0.5)
            os.fsync(samples.fileno())
        write(instance / 'tracing_on', '0')
        text = (instance / 'trace').read_bytes()
        if len(text) > 16 * 1024 * 1024:
            raise ValueError('Trace exceeds fixed evidence cap')
        with (OUT / 'trace.txt').open('xb') as stream:
            stream.write(text)
        publish(OUT / 'loss.json', {p.parent.name: p.read_text() for p in (instance / 'per_cpu').glob('cpu*/stats')})
        (OUT / 'kprobe-profile.txt').write_text((TRACE / 'kprobe_profile').read_text())
        publish(OUT / 'capture.json', {'completed': True, 'deadline_changed': False,
                'group': group, 'started_raw_ns': capture_start, 'sample_count': count,
                'ended_raw_ns': raw_ns(), 'trace_bytes': len(text)})
    except BaseException as error:
        failure = repr(error)
        publish(OUT / 'first-failure.json', {'error': failure, 'traceback': traceback.format_exc()})
    finally:
        with serialized():
            own = cleanup(group, planned)
        publish(OUT / 'cleanup-owner.json', own)
        try:
            os.write(control_w, b'D')
        except BrokenPipeError:
            failure = failure or 'Guardian control closed before normal completion'
        os.close(control_w)
        os.waitpid(child, 0)
        after = snapshot()
        guard = json.loads((OUT / 'guardian.json').read_text())
        post = {'root': after, 'root_controls_equal': after == initial['root'],
                'owner_cleanup': own['ok'], 'guardian_cleanup': guard['ok'],
                'boot_unchanged': Path('/proc/sys/kernel/random/boot_id').read_text().strip() == initial['boot'],
                'pid1_start_unchanged': pid1_start(Path('/proc/1/stat').read_text()) == pid1_start(initial['pid1_stat']),
                'time_namespace_unchanged': os.readlink('/proc/self/ns/time') == initial['time_namespace']}
        publish(OUT / 'postflight.json', post)
        if failure or not all(v is True for k, v in post.items() if k != 'root'):
            raise RuntimeError('First trace outcome or cleanup refused; no retry')


if __name__ == '__main__':
    collect()
