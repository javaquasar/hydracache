"""Offline analysis only; an entry alone never proves a successful setter."""
from collections import Counter
from decimal import Decimal
import json
import re

EVENTS = {'adjust', 'adjust_return', 'settime', 'settime_return', 'offset', 'offset_return', 'host'}
HEADER = re.compile(r'^\s*(?P<comm>.+)-(?P<pid>\d+)\s+\[(?P<cpu>\d+)\]\s+\S+\s+(?P<time>\d+\.\d+):\s+(?P<text>.*)$')
FIELD = re.compile(r'\b(modes|freq|tick|sec|ns|retval)=(-?(?:0x[0-9a-fA-F]+|\d+))\b')


def number(value):
    return int(value, 16) if 'x' in value.lower() else int(value)


def parse(text):
    records = []
    pending_stacks = {}
    stack = None
    for line in text.splitlines():
        match = HEADER.match(line)
        if match:
            d = match.groupdict()
            pid, cpu = int(d['pid']), int(d['cpu'])
            if d['text'] == '<stack trace>':
                stack = pending_stacks.get((pid, cpu))
                continue
            stack = None
            name, sep, payload = d['text'].partition(':')
            if not sep or name not in EVENTS:
                raise ValueError('Unrecognized event in private trace: ' + d['text'])
            fields = {k: number(v) for k, v in FIELD.findall(payload)}
            required = {'adjust': {'modes', 'freq', 'tick', 'sec', 'ns'},
                        'settime': {'sec', 'ns'}, 'offset': {'sec', 'ns'}, 'host': set()}
            if set(fields) != required.get(name, {'retval'}):
                raise ValueError('Missing/unexpected/faulted argument: ' + payload)
            record = {'event': name, 'kernel_pid': pid, 'comm': d['comm'].strip(), 'cpu': cpu,
                      'raw_ns': int(Decimal(d['time']) * 1_000_000_000),
                      'fields': fields, 'stack': []}
            records.append(record)
            pending_stacks[(pid, cpu)] = record
        elif line.lstrip().startswith('=>'):
            if stack is None:
                raise ValueError('Orphan stack frame')
            stack['stack'].append(line.strip()[2:].strip())
        elif line.strip() and not line.startswith('#'):
            raise ValueError('Unrecognized trace line: ' + line)
    return records


def pair(records):
    active = {}
    calls, unmatched_returns = [], Counter()
    for r in records:
        name = r['event']
        if name == 'host':
            continue
        base = name.removesuffix('_return')
        key = (r['kernel_pid'], base)
        if name.endswith('_return'):
            if not active.get(key):
                # modes=0 entries are intentionally filtered. No entry => no write claim.
                unmatched_returns[base] += 1
                continue
            entry = active[key].pop()
            calls.append({'entry': entry, 'return': r, 'successful': r['fields']['retval'] >= 0
                          if base == 'adjust' else r['fields']['retval'] == 0})
        else:
            active.setdefault(key, []).append(r)
    return calls, dict(unmatched_returns), [r for items in active.values() for r in items]


def loss_check(stats, profile):
    if not stats:
        raise ValueError('Missing per-CPU loss counters')
    for text in stats.values():
        counters = {key.strip(): value.strip() for key, sep, value in
                    (line.partition(':') for line in text.splitlines()) if sep}
        for key in ('overrun', 'commit overrun', 'dropped events'):
            if key not in counters or int(counters[key]) != 0:
                raise ValueError('Missing/nonzero trace loss: ' + key)
    profiles = {}
    for line in profile.splitlines():
        name, hits, misses = line.split()
        if name not in EVENTS or name in profiles or int(misses) != 0:
            raise ValueError('Foreign/duplicate/missed kprobe')
        profiles[name] = {'hits': int(hits), 'misses': int(misses)}
    if set(profiles) != EVENTS:
        raise ValueError('Missing probe profile')
    return profiles


def analyze(folder):
    def load(name):
        return json.loads((folder / name).read_text())
    completed = load('capture.json')
    post = load('postflight.json')
    if completed['completed'] is not True or completed['deadline_changed'] is not False:
        raise ValueError('Incomplete first capture')
    if any(v is not True for k, v in post.items() if k != 'root'):
        raise ValueError('Unconfirmed cleanup or context')
    if load('guardian.json')['reason'] != 'normal_completion':
        raise ValueError('Guardian deadline/owner loss is not a complete capture')
    profiles = loss_check(load('loss.json'), (folder / 'kprobe-profile.txt').read_text())
    records = parse((folder / 'trace.txt').read_text())
    calls, unmatched, incomplete = pair(records)
    rows = [json.loads(s) for s in (folder / 'samples.jsonl').read_text().splitlines()]
    if len(rows) != completed['sample_count'] or len(rows) < 2:
        raise ValueError('Incomplete query series')
    if any(r['modes'] != 0 or r['euid'] != 1000 or r['raw_after_ns'] < r['raw_before_ns'] for r in rows):
        raise ValueError('Unexpected query context/bracket')
    if any(b['raw_before_ns'] <= a['raw_after_ns'] for a, b in zip(rows, rows[1:])):
        raise ValueError('Reordered RAW samples')
    changes = []
    for call in calls:
        e = call['entry']
        if e['event'] == 'adjust' and e['fields']['modes'] == 0:
            raise ValueError('Read-only entry passed filter')
        item = dict(call)
        before = [r for r in rows if r['raw_after_ns'] <= e['raw_ns']]
        after = [r for r in rows if r['raw_before_ns'] >= call['return']['raw_ns']]
        if before and after:
            a, b = before[-1], after[0]
            item['enclosing_query_residual_ns'] = ((b['realtime_ns'] - a['realtime_ns']) -
                                                 (b['monotonic_ns'] - a['monotonic_ns']))
            item['query_bracket_raw_ns'] = [a['raw_after_ns'], b['raw_before_ns']]
        item['hyperv_settime_stack'] = (e['event'] == 'settime' and
                                       any(re.match(r'hv_set_host_time(?:\+|$)', s) for s in e['stack']))
        changes.append(item)
    return {'method': 'hc074-bounded-clock-trace-analysis-v1', 'rows': len(rows),
            'loss_checked': True, 'probe_profiles': profiles,
            'event_counts': dict(Counter(r['event'] for r in records)),
            'calls': changes, 'unmatched_returns_not_counted_as_writes': unmatched,
            'incomplete_entries_not_counted_as_success': incomplete,
            'host_entries': [r for r in records if r['event'] == 'host'],
            'tick_setting_requests': [c for c in changes if c['entry']['event'] == 'adjust' and c['entry']['fields']['modes'] & 0x4000],
            'query_ticks_us': sorted({r['tick_us'] for r in rows}),
            'query_frequencies_scaled_ppm': sorted({r['frequency_scaled_ppm'] for r in rows}),
            'backwards_realtime_intervals': sum(b['realtime_ns'] < a['realtime_ns'] for a, b in zip(rows, rows[1:])),
            'historical_tick_setter': None, 'original_owner_run': False,
            'admission': False, 'server': False, 'product': False}
