"""Offline summary of the sealed first capture; never enters WSL or changes time."""
from collections import Counter
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
INPUT = ROOT / 'docs/testing/performance/0.74/local-runs/wsl-clock-setter-20261023'
INDEX_SHA = '6b54d38f0ce7b4ceacdeb0ce879a7aee6570e53501cfc12206b39c78eaef051d'
OUTPUT = ROOT / 'target/wsl-clock-setter-attribution-20261023'


def summarize(analysis, rows):
    if not analysis['loss_checked'] or analysis['rows'] != len(rows) or len(rows) < 2:
        raise ValueError('Incomplete/lossy source analysis')
    if any(r['modes'] != 0 or r['euid'] != 1000 for r in rows):
        raise ValueError('Unexpected query context')
    calls = analysis['calls']
    good = [c for c in calls if c['successful']]
    ticks = [c for c in good if c['entry']['event'] == 'adjust' and c['entry']['fields']['modes'] & 0x4000]
    counted = Counter((c['entry']['comm'], c['entry']['event'], c['entry']['fields'].get('modes')) for c in good)
    windows = []
    for c in good:
        e = c['entry']
        if e['event'] != 'settime':
            continue
        prior = [r for r in rows if r['raw_after_ns'] <= e['raw_ns']]
        later = [r for r in rows if r['raw_before_ns'] >= c['return']['raw_ns']]
        if not prior or not later:
            raise ValueError('No enclosing queries for settime')
        a, b = prior[-1], later[0]
        inside = [x for x in good if a['raw_after_ns'] <= x['entry']['raw_ns']
                  and x['return']['raw_ns'] <= b['raw_before_ns']]
        steps = [x for x in inside if x['entry']['event'] == 'offset']
        if len(steps) != 1 or sum(x['entry']['event'] == 'settime' for x in inside) != 1:
            raise ValueError('Window is not one absolute settime plus one offset')
        offset = steps[0]['entry']['fields']
        delta = offset['sec'] * 1_000_000_000 + offset['ns']
        net = (b['realtime_ns'] - a['realtime_ns']) - (b['monotonic_ns'] - a['monotonic_ns'])
        windows.append({'settime_kernel_pid': e['kernel_pid'], 'settime_comm': e['comm'],
            'offset_kernel_pid': steps[0]['entry']['kernel_pid'], 'offset_comm': steps[0]['entry']['comm'],
            'query_raw_bracket_ns': [a['raw_after_ns'], b['raw_before_ns']],
            'target_minus_prior_realtime_ns': e['fields']['sec'] * 1_000_000_000 + e['fields']['ns'] - a['realtime_ns'],
            'successful_offset_request_ns': delta, 'enclosing_realtime_minus_monotonic_delta_ns': net,
            'net_minus_offset_ns': net - delta})
    first, last = rows[0], rows[-1]
    raw_min = last['raw_before_ns'] - first['raw_after_ns']
    raw_max = last['raw_after_ns'] - first['raw_before_ns']
    if raw_min <= 0 or raw_max < raw_min:
        raise ValueError('Invalid RAW enclosure')
    dm = last['monotonic_ns'] - first['monotonic_ns']
    predictions = sorted({Fraction(r['tick_us'] * r['user_hz'], 1_000_000) +
                          Fraction(r['frequency_scaled_ppm'], 65536 * 1_000_000) for r in rows})
    return {'method': 'hc074-sealed-setter-attribution-v1', 'input_index_sha256': INDEX_SHA,
        'rows': len(rows), 'successful_calls': len(good),
        'call_counts': [{'comm': k[0], 'event': k[1], 'modes': k[2], 'count': n} for k, n in sorted(counted.items())],
        'tick_setting_requests': len(ticks),
        'tick_setters_observed': sorted({c['entry']['comm'] for c in ticks}),
        'step_windows': windows, 'observed_monotonic_per_raw_enclosure': [float(Fraction(dm, raw_max)), float(Fraction(dm, raw_min))],
        'tick_plus_frequency_rate_range': [float(predictions[0]), float(predictions[-1])],
        'historical_tick_setter': None, 'authenticated_executable_identity': False,
        'new_live_observations': 0, 'admission': False, 'server': False, 'product': False,
        'scope': 'Kernel comm/PID/stack attribution; arithmetic windows are not direct measurements of each isolated step or UTC accuracy'}


def main():
    sys.path.insert(0, str(ROOT / 'tools/wsl-clock-readonly-074'))
    import offline_join
    files = offline_join.load(INPUT, INDEX_SHA)
    decoder = ROOT / 'tools/wsl-clock-trace-074/analyze.py'
    if hashlib.sha256(decoder.read_bytes()).digest() != hashlib.sha256(files['source/tools/wsl-clock-trace-074/analyze.py']).digest():
        raise ValueError('Original decoder changed')
    analysis = json.loads(files['observation/analysis.json'])
    rows = [json.loads(s) for s in files['observation/samples.jsonl'].splitlines()]
    result = summarize(analysis, rows)
    OUTPUT.mkdir(parents=True, exist_ok=False)
    with (OUTPUT / 'summary.json').open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(result, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
