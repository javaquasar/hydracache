"""Compare fixed inert probe records. Never load or execute a parser here."""
def require(value, reason):
    if not value:
        raise ValueError(reason)


def compare(cases, baseline, candidate):
    ids = [row['id'] for row in cases]
    require(ids and len(ids) == len(set(ids)), 'unique nonempty corpus')
    require([r['id'] for r in baseline] == ids == [r['id'] for r in candidate], 'exact ordered case coverage')
    semantic = []
    diagnostics = []
    for case, old, new in zip(cases, baseline, candidate):
        require(case['expect'] in ('valid', 'invalid', 'compare'), 'known expectation')
        for label, row in (('baseline', old), ('candidate', new)):
            require(type(row['exit']) is int and row['exit'] == 0 and row['timeout'] is False, 'probe process failed: '+case['id'])
            value = row['value']
            require(value['id'] == case['id'] and type(value['parsed']) is bool, 'probe record identity/type')
            expected = {'id', 'parsed', 'tree', 'roundtrip'} if value['parsed'] else {'id', 'parsed', 'message', 'location'}
            require(set(value) == expected, 'probe response schema')
            if not value['parsed']:
                require(isinstance(value['message'], str) and value['message'], 'loud parse refusal')
            if case['expect'] != 'compare' and value['parsed'] != (case['expect'] == 'valid'):
                semantic.append(dict(id=case['id'], reason=label+' expectation mismatch'))
        a, b = old['value'], new['value']
        if a['parsed'] != b['parsed']:
            semantic.append(dict(id=case['id'], reason='parse decision drift'))
        elif a['parsed']:
            if a != b:
                semantic.append(dict(id=case['id'], reason='typed/serialization/roundtrip drift'))
        elif a != b:
            diagnostics.append(dict(id=case['id'], baseline=a, candidate=b))
    return dict(cases=len(cases), semantic_differences=semantic, diagnostic_differences=diagnostics,
                semantic_contract_pass=not semantic, diagnostic_review_required=bool(diagnostics),
                release_admitted=False, probe_replayed=False)
