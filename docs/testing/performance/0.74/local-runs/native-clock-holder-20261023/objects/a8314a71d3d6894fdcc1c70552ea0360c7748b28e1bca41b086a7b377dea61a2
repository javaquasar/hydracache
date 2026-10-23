"""Offline holder outcome classification; a clock-control pass is not admission."""
import json
import re

CASES = ['healthy', 'revalidate', 'role-before', 'role-after', 'trust-after']


def holder(raw, matrix_exit, wrapper_exit, postflight_exit):
    text = raw.decode('utf-8')
    if 'HC074-HELD-LIVE-COST' in text:
        raise ValueError('Unapproved attribution override')
    reports = [json.loads(line.removeprefix('HC074-HELD-LIVE-ORIGINAL '))
               for line in text.splitlines() if line.startswith('HC074-HELD-LIVE-ORIGINAL ')]
    reported = [r['case'] for r in reports]
    if reported != CASES[:len(reports)] or len(reports) > len(CASES):
        raise ValueError('Different case schedule or duplicate report')
    successes = re.findall(r'^HC074-HELD-LIVE case=([^ ]+) seed=0x074c051e actual_fixed=true actual_consuming=true', text, re.M)
    summaries = text.count('test result: ok. 1 passed; 0 failed; 0 ignored;')
    complete = (matrix_exit == wrapper_exit == postflight_exit == 0
                and reported == successes == CASES and summaries == 6
                and all(r['returned'] is True for r in reports))
    if matrix_exit == 0 and not complete:
        raise ValueError('Zero exit without complete original shape')
    refusal = None
    for report in reports:
        if report['deadline_changed'] is not False or report['guard_clock_added'] is not False:
            raise ValueError('Changed original clock checks')
        if report['returned'] is False and refusal is None:
            causal = report['causal']['first_refusal']
            if causal is not None:
                if causal['age_ns'] != causal['now_ns'] - causal['issued_ns']:
                    raise ValueError('Inconsistent original age')
                if causal['diagnostic_reason'] == 'expired' and causal['age_ns'] < 10_000_000_000:
                    raise ValueError('Expiry before original ten-second boundary')
            refusal = {'case': report['case'], 'causal': causal, 'kernel': report['kernel'],
                       'age_sites': report['age']['sites']}
    if matrix_exit != 0 and successes == CASES:
        raise ValueError('Failed result cannot be a complete pass')
    return {'schema': 'hc074-native-holder-offline-outcome-v1', 'matrix_exit': matrix_exit,
            'wrapper_exit': wrapper_exit, 'postflight_exit': postflight_exit,
            'attempted_cases': reported, 'completed_cases': successes,
            'not_executed_cases': CASES[len(reports):],
            'complete_matrix_passed': complete, 'first_original_refusal': refusal,
            'server': True, 'product': False, 'admission': False,
            'original_delivery_proven': False, 'all_production_writers_retired_proven': False,
            'original_windows': [300, 60, 10], 'retry': False}
