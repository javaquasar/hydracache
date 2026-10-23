"""Audit saved clock/NTP publication geometry only; no live collection or tracing."""
import hashlib
import json

import causality as c

INPUT = c.ARCHIVE
INDEX_SHA = '798b204c4b14865f482d64cdc4176aec7c1c3c4126b87bf635e8d9c2ba140082'
OUTPUT = c.base.ROOT / 'target/wsl-clock-offline-join-20261023'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def load(folder, expected_sha):
    raw = (folder / 'index.json').read_bytes()
    if digest(raw) != expected_sha:
        raise ValueError('Input index changed')
    index = json.loads(raw)
    if any(index[k] is not False for k in ('admission', 'product', 'server', 'original_owner_run')):
        raise ValueError('Not the recorded non-admitting input')
    result = {}
    for name, entry in index['files'].items():
        sha = entry['sha256']
        if len(sha) != 64 or any(x not in '0123456789abcdef' for x in sha):
            raise ValueError('Unsafe object digest')
        content = (folder / 'objects' / sha).read_bytes()
        if digest(content) != sha or len(content) != entry['bytes']:
            raise ValueError('Input object changed')
        result[name] = content
    return result


def windows(rows, messages):
    # A getter for row i occurs after its clock read. A newly published packet's
    # upstream correction is enclosed only by clock rows i-1 and i+1, not by i-1/i.
    c.analyze(rows, messages)  # Preserve all original method checks.
    result = []
    for i in range(1, len(messages)):
        before, now = messages[i - 1], messages[i]
        if before['packet'] == now['packet']:
            continue
        item = {'getter_row': i, 'packet': now['packet'], 'exact_caller': None,
                'applied_step_assumption': 'upstream non-spike callback succeeds before publishing NTPMessage'}
        if now['packet'] != before['packet'] + 1:
            item.update(complete=False, reason='missing_packet_history')
        elif i + 1 >= len(rows):
            item.update(complete=False, reason='no_clock_after_getter')
        elif now['spike']:
            item.update(complete=False, reason='spike_is_not_an_applied_step')
        else:
            a, b = rows[i - 1], rows[i + 1]
            net = ((b['realtime_ns'] - a['realtime_ns']) -
                   (b['monotonic_ns'] - a['monotonic_ns']))
            t1, t2, t3, t4 = now['timestamps_us']
            # Integer nanoseconds from the retained microsecond representation;
            # not an accuracy/rounding guarantee for the actual syscall argument.
            offset = ((t2 - t1) + (t3 - t4)) * 500
            if abs(offset) < 400_000_000:  # Upstream 255.4 NTP_MAX_ADJUST, not a release threshold.
                item.update(complete=False, reason='small_offset_uses_slew_not_full_step')
            else:
                item.update(complete=True, first_clock_row=i - 1, last_clock_row=i + 1,
                            net_residual_ns=net, ntp_offset_from_saved_timestamps_ns=offset,
                            remainder_if_ntp_step_applied_ns=net - offset)
        result.append(item)
    return result


def audit(files):
    for path in c.FILES + [c.base.HERE / 'diagnose.py']:
        name = 'source/' + path.relative_to(c.base.ROOT).as_posix()
        if digest(path.read_bytes()) != digest(files[name]):
            raise ValueError('Current decoder/method differs from archived source')
    rows = [json.loads(s) for s in files['observation/kernel.jsonl'].splitlines()]
    messages = [c.ntp(s.decode()) for s in files['observation/ntp.txt'].splitlines()]
    original = json.loads(files['observation/analysis.json'])
    recomputed = c.analyze(rows, messages)
    if any(original[k] != v for k, v in recomputed.items()):
        raise ValueError('Original analysis does not reproduce')
    before = c.base.identity(files['observation/identity-before.txt'].decode())
    after = c.base.identity(files['observation/identity-after.txt'].decode())
    if before != after or before != original['identity'] or original['same_session'] is not True:
        raise ValueError('Cannot join saved session identities')
    return {'method': 'hc074-clock-ntp-offline-publication-join-v1', 'input_index_sha256': INDEX_SHA,
            'original_analysis_reproduced': True, 'publication_windows': windows(rows, messages),
            'new_live_observations': 0, 'tracing_enabled': False, 'admission': False,
            'exact_backward_setter': None, 'historical_tick_setter': None}


def main():
    c.sources()  # Check frozen inputs/runtime without entering WSL.
    result = audit(load(INPUT, INDEX_SHA))
    OUTPUT.mkdir(parents=True, exist_ok=False)
    c.base.put(OUTPUT / 'analysis.json', result)
    print(json.dumps(result))


if __name__ == '__main__':
    main()
