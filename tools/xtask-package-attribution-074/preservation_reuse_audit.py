"""Offline literal-query reuse audit. Never resolves refs or admits a cache."""
from collections import Counter
import json
from pathlib import Path
import re

import helper_capture
import helper_review
import review
import sweep_review as sweep

PACKET = 'docs/testing/performance/0.74/local-runs/preservation-helper-spans-20261025'
PACKET_SHA256 = 'f239630ed26fc19beae9ab4163a59e4807747307b81293e859ef6499208b8b99'
SCOPE = 'tools/long-run-supervisor-074/src'
FIXTURE = 'tools/long-run-supervisor-074/tests/fixtures/diagnostic-pilot-root-Cargo.lock'


def path_ok(path):
    return (bool(path) and not any(c in path for c in '\0\r\n\\')
            and all(part not in ('', '.', '..') for part in path.split('/')))


def revision_shape(revision):
    review.require(re.fullmatch('[0-9a-f]{4,40}', revision) is not None, 'unreviewed revision spelling')
    return 'full40' if len(revision) == 40 else 'short'


def classify(args):
    review.require(type(args) is list and bool(args) and all(type(v) is str for v in args), 'literal argv')
    if len(args) == 2 and args[0] == 'show':
        revision, sep, path = args[1].partition(':')
        review.require(sep and path_ok(path), 'show path')
        return revision_shape(revision) + '_default_replacements'
    if len(args) == 6 and args[:3] == ['ls-tree', '-r', '--name-only'] and args[4:] == ['--', SCOPE]:
        revision_shape(args[3])
        return 'historical_inventory_default_replacements'
    if args == ['ls-files', '--', SCOPE]:
        return 'fresh_worktree_inventory'
    if args == ['hash-object', '--no-filters', FIXTURE]:
        return 'fresh_fixture_bytes'
    if args == ['check-attr', 'text', '--', FIXTURE]:
        return 'fresh_fixture_attributes'
    raise ValueError('unreviewed command shape')


def analyze(frames):
    review.require(type(frames) is list and bool(frames), 'complete frames required')
    counts, shows, cases = Counter(), Counter(), []
    seen_cases = set()
    reads = 0
    for frame in frames:
        helper_review.trace_frame(frame)
        review.require(frame['case'] not in seen_cases, 'duplicate case')
        seen_cases.add(frame['case'])
        local = Counter()
        commands = Counter()
        for row in frame['records']:
            if row['operation'] == 'read':
                reads += 1
            if row['operation'] != 'git':
                continue
            args = row['arguments']
            counts[classify(args)] += 1
            commands[args[0]] += 1
            if args[0] == 'show':
                # Keep the original short/full spelling. No expansion, path
                # canonicalization, object-ID joining or inferred shared key.
                key = tuple(args)
                shows[key] += 1
                local[key] += 1
        cases.append(dict(case=frame['case'], commands=dict(sorted(commands.items())),
                          show_calls=sum(local.values()), distinct_literal_show_queries=len(local),
                          literal_occurrences_beyond_first=sum(local.values())-len(local)))
    review.require(bool(shows), 'show coverage')
    repeated = [dict(arguments=list(key), occurrences=count) for key, count in sorted(shows.items()) if count > 1]
    return dict(schema='hc074-preservation-reuse-audit-v1',
        frames=len(frames), git_calls=sum(counts.values()), fresh_read_calls=reads,
        command_shapes=dict(sorted(counts.items())), show_calls=sum(shows.values()),
        distinct_literal_show_queries=len(shows),
        literal_occurrences_beyond_first=sum(shows.values())-len(shows),
        repeated_queries=repeated, cases=sorted(cases, key=lambda v: v['case']),
        idealized_exact_key_model='initially empty shared cache; no misses or failed publication hidden; literal keys only',
        observed_cache_hits=False, other_helpers_or_previous_warming_included=False,
        original_replacement_or_config_stability_attested=False,
        independently_enrolled_per_query_body_pins=False,
        literal_reuse_is_admission=False, reduced_startup_candidate_admitted=False,
        default_replacement_policy_may_change_between_original_calls=True,
        preplanned_prefetch_changes_conditional_observation_schedule=True,
        elapsed_or_cpu_saving_estimated=False, runtime_changed=False,
        measurements_repeated=False, subprocesses_executed=False, server_admission=False)


def audit(root):
    folder = root / PACKET
    raw = (folder/'index.json').read_bytes()
    review.require(review.sha(raw) == PACKET_SHA256, 'sealed helper packet drift')
    index = review.decode_json(raw)
    original = sweep.load_snapshot(root)
    proof = sweep.analyze_snapshot(original)
    syntax = review.decode_json(original['syntax/final-closure/result.json'])
    source, = [v for v in syntax['sources'] if v['source'] == sweep.SOURCE]
    review.require(source['sha256'] == sweep.SOURCE_SHA256, 'independent original AST')
    cases = {v['scope'] for v in source['sites'] if v['kind'] == 'function-attribute' and v['detail'] == 'test'}
    required = {v['scope'].split('::')[0] for v in source['sites']
                if v['kind'] == 'function-call' and v['detail'] in ('git', 'read')
                and v['scope'].split('::')[0] in cases}
    review.require(len(cases) == 20 and len(required) == 19, 'original complete case catalog')
    log = review.member(folder, index, 'capture/profile/stderr.log')
    trace = helper_review.traces(log, cases, required_cases=required)
    stored = review.decode_json(review.member(folder, index, 'capture/profile/result.json'))
    review.require(stored['trace'] == trace and stored['target_pass'] is True, 'original complete trace verdict')
    contract = (root/'tools/xtask-package-attribution-074/helper_contract.rs').read_text()
    helper_capture.erase((root/sweep.SOURCE).read_text(), contract, original['reviewed-source/'+sweep.SOURCE].decode())
    result = analyze(trace['frames'])
    review.require(result['git_calls'] == trace['counts']['git'] == 3157
                   and result['show_calls'] == trace['shows'] == 3141
                   and result['fresh_read_calls'] == trace['counts']['read'] == 3140, 'original scope counts')
    result.update(helper_packet_sha256=PACKET_SHA256, original_packet_sha256=sweep.PACKET_SHA256,
                  current_helper_erases_to_original=True, source_sha256=sweep.SOURCE_SHA256,
                  original_proven_sweep_lower_bound=proof['proven_direct_show_launches_lower_bound'],
                  profile_log_sha256=review.sha(log), package_pass=False)
    return result


if __name__ == '__main__':
    print(json.dumps(audit(Path(__file__).resolve().parents[2]), indent=2, sort_keys=True))
