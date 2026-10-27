"""Read-only accounting for the fixed commands; never dispatch archived code."""
import re
from pathlib import Path

import prepare as p

PARSER_SHA = '0229d4067fffd0d0b5acee7d864efc64aadfb201de1ea3f17944a2f8dddf3c4d'
STAGES = ['doc-check', 'management-center-check 0.72', 'performance-contract-check 0.73',
    'release feature leak', 'format', 'clippy workspace', 'clippy', 'clippy hydracache common',
    'clippy hydracache system allocator', 'clippy hydracache mimalloc', 'dependency bans',
    'dependency policy', 'DST fast budget', 'soak fast budget', 'raft failpoint crash-safety',
    'clippy (Windows allocator)', 'tests (workspace excluding xtask)',
    'tests (xtask lib/integration)', 'docs', 'performance budget contract', 'management console']
START = re.compile(r'^running (\d+) tests?$')
CASE = re.compile(r'^test (.+) \.\.\. (ok|FAILED|ignored)(?:, .+)?$')
END = re.compile(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
                 r'(\d+) measured; (\d+) filtered out; finished in (\d+(?:\.\d+)?)s$')


def rust_blocks(text):
    rows, active = [], None
    for line in text.splitlines():
        start, case, end = START.fullmatch(line), CASE.fullmatch(line), END.fullmatch(line)
        if start:
            p.require(active is None, 'nested libtest start')
            active = dict(declared=int(start[1]), cases=[], complete=False)
            rows.append(active)
        elif case:
            p.require(active is not None, 'case outside libtest block')
            p.require(case[1] not in {v['name'] for v in active['cases']}, 'duplicate case')
            active['cases'].append(dict(name=case[1], verdict=case[2]))
        elif end:
            p.require(active is not None, 'summary outside libtest block')
            passed, failed, ignored, measured, filtered = map(int, end.group(2, 3, 4, 5, 6))
            observed = [r['verdict'] for r in active['cases']]
            p.require((observed.count('ok'), observed.count('FAILED'), observed.count('ignored')) ==
                (passed, failed, ignored), 'case/summary mismatch')
            p.require(passed+failed+ignored == active['declared'] and measured == filtered == 0,
                      'declared/filter/measurement mismatch')
            p.require(end[1] == ('FAILED' if failed else 'ok'), 'summary verdict mismatch')
            active.update(complete=True, passed=passed, failed=failed, ignored=ignored, seconds=end[7])
            active = None
        elif line.startswith(('running ', 'test result:')):
            raise ValueError('unsupported libtest framing')
    return rows


def workspace(stdout, stderr, receipt):
    p.checked(p.ROOT, 'crates/xtask/src/verify.rs', p.PINS['crates/xtask/src/verify.rs'])
    labels = [line[3:-3] for line in stdout.splitlines() if line.startswith('== ') and line.endswith(' ==')]
    p.require(labels == STAGES[:len(labels)] and len(labels) <= len(STAGES), 'verify stage order/catalog drift')
    terminal = stdout.splitlines().count('verify: all gates passed')
    p.require(terminal <= 1, 'duplicate verify terminal marker')
    blocks = rust_blocks(stdout)
    scheduled = []
    for line in stderr.splitlines():
        text = line.strip()
        if text.startswith('Running '):
            name, separator, _ = text[8:].partition(' (')
            p.require(separator, 'unrecognized Cargo target')
            scheduled.append(dict(kind='test', name=name.replace('\\','/')))
        elif text.startswith('Doc-tests '):
            scheduled.append(dict(kind='doctest', name=text[10:]))
    # Rustdoc may emit several complete libtest groups for a single Cargo Doc-tests
    # target. Do not invent a case-to-target mapping from two asynchronous streams.
    doctests=any(t['kind']=='doctest' for t in scheduled)
    p.require(doctests or len(blocks)<=len(scheduled),'libtest has no Cargo target')
    one_to_one=len(blocks)==len(scheduled)
    if one_to_one:
        for row, target in zip(blocks, scheduled):row['target']=target
    good = receipt['classification'] == 'success' and receipt['exit'] == 0
    covered=(len(blocks)>=len(scheduled)) if doctests else one_to_one
    complete = len(labels) == len(STAGES) and terminal == 1 and covered and all(r['complete'] for r in blocks)
    if good:
        p.require(complete and blocks and not any(r['failed'] for r in blocks), 'zero exit lacks complete verify accounting')
        console = [line for line in stdout.splitlines() if line.startswith('management console: SKIP ')]
        p.require(len(console) <= 1 and all(line in ('management console: SKIP (node not found)',
            'management console: SKIP (npm not found)') for line in console), 'unapproved console skip')
        if not console:
            p.require([line for line in stdout.splitlines() if line.startswith('-- console ')] ==
                ['-- console npm deps', '-- console static check', '-- console playwright'], 'console stage accounting')
    done = [r for r in blocks if r['complete']]
    completed_stages = len(labels) if good else max(0,len(labels)-1)
    return dict(complete=complete, workspace_pass=good and complete,
        stages=[dict(name=name, state='complete' if i < completed_stages else
            ('incomplete-or-failed' if i < len(labels) else 'unstarted')) for i,name in enumerate(STAGES)],
        scheduled_targets=scheduled, targets=blocks, unsummarized_targets=scheduled[len(blocks):],
        case_to_target_assignment='ordered-one-to-one' if one_to_one else 'not-asserted-rustdoc-grouping',
        passed=sum(r['passed'] for r in done), failed=sum(r['failed'] for r in done),
        ignored=sum(r['ignored'] for r in done), ignored_policy='original workspace default/nightly tier unchanged',
        target_discovery_authority='fixed unfiltered Cargo argv and pinned workspace manifests',
        static_workspace_target_prediction=False, release_admitted=False)


def account(mode, stdout, stderr, receipt, catalog):
    p.require(receipt['retired'] and receipt['cleanup_seconds'] == 5, 'unretired original Job')
    if mode == 'package':
        p.checked(p.ROOT, 'tools/xtask-package-attribution-074/review.py', PARSER_SHA)
        parser = p.load(p.ROOT/'tools/xtask-package-attribution-074/review.py','hc074_fixed_accounting')
        p.require(receipt['bound_seconds'] == 780, 'package bound drift')
        build=any(line.strip().startswith('Finished ') for line in stderr.splitlines())
        if receipt['classification'] not in ('success','test-failure','timeout') or not build:
            scheduled=[]
            for line in stderr.splitlines():
                if line.strip().startswith('Running '):
                    name,sep,_=line.strip()[8:].partition(' (')
                    p.require(sep,'partial Cargo target framing')
                    scheduled.append(name.replace('\\','/'))
            p.require(scheduled==catalog[:len(scheduled)],'partial target catalog drift')
            blocks=rust_blocks(stdout)
            p.require(len(blocks)<=len(scheduled),'unscheduled partial test block')
            p.require(receipt['classification']!='success','zero exit without original build framing')
            done=[r for r in blocks if r['complete']]
            return dict(complete=False,package_pass=False,reason=receipt['classification'],
                expected_targets=len(catalog),scheduled_targets=scheduled,targets=blocks,
                passed=sum(r['passed'] for r in done),failed=sum(r['failed'] for r in done),
                ignored=sum(r['ignored'] for r in done),unstarted_targets=catalog[len(scheduled):],
                unsummarized_targets=scheduled[len(blocks):],build_finished=build)
        return parser.parse_transcript(stdout, stderr, catalog, receipt['exit'],
            'outer-timeout' if receipt['classification']=='timeout' else None,780)
    p.require(mode == 'workspace' and receipt['bound_seconds'] == 4800, 'workspace command/bound drift')
    return workspace(stdout,stderr,receipt)
