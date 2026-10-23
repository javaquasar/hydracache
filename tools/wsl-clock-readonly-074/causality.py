"""A separate fixed read-only discipline/NTP correlation lane; never admission."""
import argparse
from fractions import Fraction
import json
from pathlib import Path
import subprocess
import sys
import diagnose as base

OUT = base.ROOT / 'target/wsl-clock-causality-20261023'
ARCHIVE = base.ROOT / 'docs/testing/performance/0.74/local-runs/wsl-clock-causality-20261023'
PREVIOUS = base.ARCHIVE / 'index.json'
PREVIOUS_SHA = 'edec06e2ac4fa97126ce14ac5c0e4d1aff9cae7bd39f96bb91391102f3d606e3'
FILES = [base.HERE / p for p in ('discipline.c', 'causality-session.sh', 'causality.py', 'test_causality.py')]


def ntp(line):
    tokens = line.split()
    if len(tokens) != 21 or tokens[1] != '(uuuuittayttttbtt)' or tokens[9] != '4':
        raise ValueError('Unknown NTPMessage shape')
    row = int(tokens[0])
    t1, t2, t3, t4 = map(int, tokens[14:18])
    if tokens[18] not in ('true', 'false') or min(t1, t2, t3, t4) < 0:
        raise ValueError('Invalid NTPMessage')
    return {'row': row, 'packet': int(tokens[19]), 'spike': tokens[18] == 'true',
            'offset_us': float(Fraction((t2 - t1) + (t3 - t4), 2)),
            'delay_us': (t4 - t1) - (t3 - t2), 'timestamps_us': [t1, t2, t3, t4]}


def analyze(rows, messages):
    if len(rows) != 81 or len(messages) != 81:
        raise ValueError('Incomplete first collection')
    if [m['row'] for m in messages] != list(range(81)):
        raise ValueError('Reordered NTP rows')
    for r in rows:
        if r['modes'] != 0 or r['result'] < 0 or r['euid'] != 1000 or r['user_hz'] != 100:
            raise ValueError('Not the admitted unprivileged read method')
        if r['raw_after_ns'] < r['raw_before_ns']:
            raise ValueError('RAW bracket reversed')
    events = []
    updates = []
    for i, (a, b) in enumerate(zip(rows, rows[1:]), 1):
        if messages[i]['packet'] < messages[i - 1]['packet']:
            raise ValueError('NTP packet sequence reset; do not join daemon incarnations')
        dr = b['realtime_ns'] - a['realtime_ns']
        dm = b['monotonic_ns'] - a['monotonic_ns']
        if dm < 0 or b['boottime_ns'] < a['boottime_ns'] or b['raw_before_ns'] < a['raw_after_ns']:
            raise ValueError('Cannot join backwards monotonic/RAW samples')
        # Record every residual, not only those exceeding a newly chosen cutoff.
        events.append({'row': i, 'realtime_delta_ns': dr, 'monotonic_delta_ns': dm,
                       'residual_ns': dr - dm, 'packet_before': messages[i - 1]['packet'],
                       'packet_after': messages[i]['packet']})
        if messages[i]['packet'] != messages[i - 1]['packet']:
            updates.append(messages[i])
    a, b = rows[0], rows[-1]
    dm = b['monotonic_ns'] - a['monotonic_ns']
    raw_min = b['raw_before_ns'] - a['raw_after_ns']
    raw_max = b['raw_after_ns'] - a['raw_before_ns']
    if raw_min <= 0:
        raise ValueError('No elapsed RAW enclosure')
    predictions = sorted({Fraction(r['tick_us'] * r['user_hz'], 1_000_000) +
                          Fraction(r['frequency_scaled_ppm'], 65536 * 1_000_000) for r in rows})
    return {'method': 'hc074-clock-discipline-ntp-readonly-v1', 'rows': 81,
            'events': events, 'ntp_updates': updates,
            'backwards_realtime': [e for e in events if e['realtime_delta_ns'] < 0],
            'largest_residuals': sorted(events, key=lambda e: abs(e['residual_ns']), reverse=True)[:8],
            'ticks_us': sorted({r['tick_us'] for r in rows}),
            'frequencies_scaled_ppm': sorted({r['frequency_scaled_ppm'] for r in rows}),
            'statuses': sorted({r['status'] for r in rows}),
            'tick_plus_frequency_rate_range': [float(predictions[0]), float(predictions[-1])],
            'observed_monotonic_per_raw_enclosure': [float(Fraction(dm, raw_max)), float(Fraction(dm, raw_min))],
            'admission': False, 'exact_clock_setting_caller': None, 'historical_tick_setter': None}


def sources():
    if base.sha(PREVIOUS) != PREVIOUS_SHA:
        raise ValueError('Previous first clock series changed')
    return base.frozen() | {p.relative_to(base.ROOT).as_posix(): base.sha(p) for p in FILES}


def collect():
    base.OUT = OUT
    before = sources()
    # The parent contains the exploratory read-only query binary. Never overwrite it.
    if (OUT / 'intent.json').exists() or (OUT / 'kernel.jsonl').exists():
        raise FileExistsError('Consumed lane: no retry')
    base.command('controls', [sys.executable, '-B', '-m', 'unittest', 'discover', '-s', str(base.HERE), '-v'])
    base.command('compile-correlation', ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec',
        'cc', '-std=c11', '-O2', '-Wall', '-Wextra', '-Werror',
        base.linux(base.HERE / 'discipline.c'), '-o', base.linux(OUT / 'discipline-correlation')])
    base.command('shell-syntax', ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec', '/bin/sh', '-n',
        base.linux(base.HERE / 'causality-session.sh')])
    if before != sources():
        raise ValueError('Build/source drift')
    base.put(OUT / 'intent.json', {'sources': before, 'binary_sha256': base.sha(OUT / 'discipline-correlation'),
        'git': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=base.ROOT, text=True).strip(),
        'rows': 81, 'sleep_seconds': 1, 'distro': 'Ubuntu', 'user': 'artur',
        'retry': False, 'original_owner_run': False, 'server': False, 'service_change': False,
        'clock_change': False, 'admission': False, 'previous_index_sha256': PREVIOUS_SHA})
    base.command('session', ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec',
        'timeout', '--signal=TERM', '--kill-after=2s', '110s', '/bin/sh',
        base.linux(base.HERE / 'causality-session.sh'), base.linux(OUT)], 130)
    before_id = base.identity((OUT / 'identity-before.txt').read_text())
    after_id = base.identity((OUT / 'identity-after.txt').read_text())
    if before_id != after_id or sources() != before:
        raise ValueError('Context or source changed')
    result = analyze([json.loads(s) for s in (OUT / 'kernel.jsonl').read_text().splitlines()],
                     [ntp(s) for s in (OUT / 'ntp.txt').read_text().splitlines()])
    result.update(identity=before_id, same_session=True)
    base.put(OUT / 'analysis.json', result)
    print(json.dumps({k: v for k, v in result.items() if k != 'events'}))


def retain():
    # Reuse only the generic exact-byte archive writer, never the old observer.
    sources()
    base.OUT = OUT
    base.ARCHIVE = ARCHIVE
    # Keep base.frozen's qualification/first-refusal anchors in its final slots.
    base.FILES = FILES + [PREVIOUS] + base.FILES
    base.retain()


def verify():
    base.ARCHIVE = ARCHIVE
    base.verify()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('collect', 'retain', 'verify'))
    {'collect': collect, 'retain': retain, 'verify': verify}[parser.parse_args().mode]()
