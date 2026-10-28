"""Native test identity/count accounting; process exit alone is insufficient."""
import re


def require(value, reason):
    if not value:
        raise ValueError(reason)


def inventory(raw):
    lines = raw.decode('utf-8').splitlines()
    names = [line[:-6] for line in lines if line.endswith(': test')]
    require(len(names) == 105 and len(set(names)) == len(names), 'complete nonempty fixture inventory')
    require(lines[-1] == '105 tests, 0 benchmarks', 'exact inventory total')
    require('serialize::mtu_review::minimal_bound_contract' in names and
            'delta::mtu_review::supported_delta_operations' in names and
            'migration_review_074::snapshot' in names, 'original and new contracts present')
    return sorted(names)


def account(raw, names, selection, exact):
    expected = [n for n in names if n == selection] if exact else [n for n in names if n.startswith(selection)]
    require(expected, 'nonempty selected inventory')
    text = raw.decode('utf-8').replace('\r\n', '\n')
    rows = re.findall(r'^test (.+?) (?:- should panic )?\.\.\. (ok|FAILED)$', text, re.M)
    require(sorted(n for n, _ in rows) == sorted(expected), 'exact executed test identities')
    summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [0-9.]+s$', text, re.M)
    require(len(summaries) == 1, 'one native summary')
    status, passed, failed, ignored, measured, filtered = summaries[0]
    passed, failed, ignored, measured, filtered = map(int, (passed, failed, ignored, measured, filtered))
    require(passed == sum(s == 'ok' for _, s in rows) and failed == sum(s == 'FAILED' for _, s in rows), 'matching native counts')
    require(passed+failed == len(expected) and ignored == measured == 0 and filtered == len(names)-len(expected), 'complete native selection')
    require((status == 'ok') == (failed == 0), 'native status agreement')
    return dict(passed=passed, failed=failed, filtered=filtered, selected=len(expected),
                complete=True, test_pass=failed == 0, executed=rows, release_admitted=False)


def extended_inventory(raw, baseline):
    lines = raw.decode('utf-8').splitlines()
    names = [line[:-6] for line in lines if line.endswith(': test')]
    extra = 'delta::oversize_review::oversized_rejection_preserves_writer'
    require(len(baseline) == len(set(baseline)) == 105 and extra not in baseline, 'original catalog authority')
    require(len(names) == len(set(names)) == 106 and lines[-1] == '106 tests, 0 benchmarks', 'complete extended native total')
    require(sorted(names) == sorted(baseline+[extra]), 'only one precise regression addition')
    return sorted(names)
