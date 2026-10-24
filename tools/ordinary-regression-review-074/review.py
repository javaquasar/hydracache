"""Offline ordinary-regression review. Never retries or admits a launch."""
import argparse
import json
import re
from pathlib import Path

PREFIX = 'HC074-KERNEL-FIRST-REFUSAL '
EVENT = re.compile(r'^HC074-KERNEL-FIRST-REFUSAL Event \{ phase: "([^"\n]+)", site: "([^"\n]+)", file: "([^"\n]+)", line: ([0-9]+), clock: \[([0-9]+(?:, [0-9]+){5})\] \}$')
VERDICT = re.compile(r'^test result: (ok|FAILED)\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored; ([0-9]+) measured; ([0-9]+) filtered out; finished in ([0-9.]+)s$', re.M)


def events(stderr):
    rows = []
    for number, text in enumerate(stderr.splitlines(), 1):
        if PREFIX not in text:
            continue
        match = EVENT.fullmatch(text)
        if not match:
            rows.append(dict(stderr_line=number, raw=text, parsed=False))
            continue
        phase, site, source, source_line, values = match.groups()
        clock = list(map(int, values.split(', ')))
        row = dict(stderr_line=number, parsed=True, phase=phase, site=site,
                   source=source, source_line=int(source_line), clock=clock)
        if site == 'clock-order':
            before_boot, after_boot, before_real, after_real, same_boot, hold = clock
            if same_boot not in (0, 1) or hold != 60_000_000_000:
                raise ValueError('Unknown original clock-order contract')
            row['failed_predicates'] = [name for name, failed in (
                ('boot-id-changed', not same_boot),
                ('boottime-backwards', after_boot < before_boot),
                ('realtime-backwards', after_real < before_real)) if failed]
            if not row['failed_predicates']:
                raise ValueError('Clock-order refusal has no failed original predicate')
            row['realtime_backward_ns'] = max(0, before_real - after_real)
            row['boottime_delta_ns'] = after_boot - before_boot
        rows.append(row)
    return rows


def ordinary(stdout):
    verdicts = VERDICT.findall(stdout)
    if len(verdicts) != 1:
        raise ValueError('One complete parent verdict required')
    status, *numbers, elapsed = verdicts[0]
    passed, failed, ignored, measured, filtered = map(int, numbers)
    names = re.findall(r'^test (\S+) \.\.\. FAILED$', stdout, re.M)
    if len(names) != failed or len(set(names)) != failed:
        raise ValueError('Failed-case membership differs from parent verdict')
    total = passed + failed + ignored
    if f'running {total} tests\n' not in stdout or measured or filtered:
        raise ValueError('Incomplete or filtered ordinary parent cohort')
    if (status == 'ok') != (failed == 0):
        raise ValueError('Inconsistent ordinary result')
    return dict(passed=passed, failed=failed, ignored=ignored, filtered=filtered,
                selected=total, failed_names=names, elapsed_seconds=float(elapsed))


def review(folder):
    result = json.loads((folder / 'result.json').read_bytes())
    before = json.loads((folder / 'run-intent.json').read_bytes())['source']
    if not result['source_unchanged'] or before != result['source_after']:
        raise ValueError('Source changed or final receipt incomplete')
    return dict(ordinary=ordinary((folder / 'run.stdout.log').read_text()),
                kernel_events=events((folder / 'run.stderr.log').read_text()),
                retained_failure=result['failure'], image=result['image'],
                adjacent_event_to_test_attribution=False,
                historical_failure_cause_proved=False, timing_speedup_claim=False,
                admission=False, server=False, retry=False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    args = parser.parse_args()
    print(json.dumps(review(args.folder), indent=2, sort_keys=True))
