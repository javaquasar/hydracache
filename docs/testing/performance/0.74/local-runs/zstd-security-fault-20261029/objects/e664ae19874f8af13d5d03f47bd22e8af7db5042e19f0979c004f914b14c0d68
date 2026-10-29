"""Pure partial-profile review. Nested wall views are never disjoint CPU costs."""
import json

PHASES = ('prepare-consent', 'accept-consent', 'outputs', 'enrollment-frame',
          'fixed-manager-message', 'owned-root-preparation', 'enrollment',
          'placement', 'fixed-slice', 'consuming')
KERNEL = ('PacketContext', 'Sender', 'Alive', 'Projection', 'Origin', 'Boot',
          'Namespace', 'Sample', 'WorkerOrigin', 'Document')
LIVE = ('KernelWorker', 'WorkerPolicy', 'WorkerSource', 'ActiveAuthority',
        'KernelContext', 'LocalContext')


def require(ok, why):
    if not ok:
        raise ValueError(why)


def integer(value):
    return type(value) is int and value >= 0


def records(raw, marker):
    result = []
    for line in raw.decode('utf-8', errors='strict').splitlines():
        if marker in line:
            require(line.count(marker) == 1, 'Duplicate record marker')
            result.append(json.loads(line.split(marker, 1)[1]))
    return result


def review(raw, matrix_exit, wrapper_exit, postflight_exit):
    phases = records(raw, 'HC074-HELD-PHASE ')
    originals = records(raw, 'HC074-HELD-LIVE-ORIGINAL ')
    require(phases and tuple(row['phase'] for row in phases) == PHASES[:len(phases)],
            'Missing, reordered or repeated profile phases')
    require(len(originals) == 1 and originals[0]['case'] == 'healthy', 'Not one original case')
    original = originals[0]
    require(original['guard_clock_added'] is False and original['deadline_changed'] is False,
            'Original guard/window changed')
    facts = []
    for index, row in enumerate(phases):
        require(row['instrumented'] is True and row['guard_clock_added'] is False
                and row['deadline_changed'] is False and row['healthy_gate'] is False
                and row['includes_child_cpu'] is False and row['overlapping_spans'] is True,
                'Profile semantics changed')
        require(integer(row['wall_ns']) and (row['process_cpu_ns'] is None or integer(row['process_cpu_ns'])),
                'Invalid phase clock')
        require(type(row['returned']) is bool and (row['returned'] or index == len(phases) - 1),
                'Continued after failed phase')
        views = {}
        for key, names in (('kernel', KERNEL), ('live', LIVE)):
            require(len(row[key]) == len(names), 'View shape changed')
            views[key] = []
            for name, span in zip(names, row[key]):
                require(all(integer(span[field]) for field in ('calls', 'inclusive_ns', 'outside_children_ns'))
                        and span['outside_children_ns'] <= span['inclusive_ns'], 'Invalid nested span')
                views[key].append(dict(kind=name, **span))
        facts.append(dict(phase=row['phase'], returned=row['returned'], wall_ns=row['wall_ns'],
                          process_cpu_ns=row['process_cpu_ns'], authentication_calls=row['authentication_calls'],
                          signature_calls=row['signature_calls'], **views))
    successful_shape = (len(phases) == len(PHASES) and all(row['returned'] for row in phases)
                        and original['returned'] is True)
    if matrix_exit == 0:
        require(successful_shape and wrapper_exit == 0, 'Zero exit with incomplete profile')
    else:
        require(not successful_shape and original['returned'] is False and wrapper_exit != 0,
                'Inconsistent failed profile')
    refusal = original['causal']['first_refusal']
    if refusal is not None:
        require(integer(refusal['age_ns']) and refusal['now_ns'] - refusal['issued_ns'] == refusal['age_ns'],
                'Inconsistent original refusal age')
        if refusal['diagnostic_reason'] == 'expired':
            require(refusal['age_ns'] >= 10_000_000_000, 'Early expiry')
    return dict(phases=facts, first_original_refusal=refusal, first_failed_phase=next(
        (row['phase'] for row in phases if not row['returned']), None),
        not_observed_phases=list(PHASES[len(phases):]), original=original,
        matrix_exit=matrix_exit, wrapper_exit=wrapper_exit, postflight_exit=postflight_exit,
        postflight_passed=postflight_exit == 0, diagnostic_complete=successful_shape and postflight_exit == 0,
        nested_wall_views_overlap=True, cpu_includes_children=False, wall_residual_is_io=False,
        performance_improvement_proven=False, original_delivery_proven=False,
        all_production_writers_retired_proven=False, original_windows_seconds=[300, 60, 10],
        server=True, product=False, admission=False, retry=False)
