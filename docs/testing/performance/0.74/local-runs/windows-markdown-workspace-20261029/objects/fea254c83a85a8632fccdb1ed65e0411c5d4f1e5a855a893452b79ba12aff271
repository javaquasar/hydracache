"""Inert snapshot comparison; never import or start the fixture library."""
import json
import re

MARKER = 'HC074_ZSTD_SNAPSHOT:'


def require_selected_test(raw, name, total):
    text = raw.decode('utf-8')
    require(text.splitlines().count('test '+name+' ... ok') == 1, 'exact successful selected test')
    summaries = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [0-9.]+s$', text, re.M)
    require(summaries == [('1', '0', '0', '0', str(total-1))], 'nonempty exact native accounting')


def require(value, reason):
    if not value:
        raise ValueError(reason)


def parse(raw):
    lines = [line for line in raw.decode('utf-8').splitlines() if line.startswith(MARKER)]
    require(len(lines) == 1, 'one complete native snapshot')
    value = json.loads(lines[0][len(MARKER):])
    require(isinstance(value, dict), 'snapshot object')
    return value


def compare(old, new):
    expected = [f'{t}/{n}/{k}' for t in (16, 200, 1000, 65535) for n in (0, 1, 100) for k in range(3)]+['max-string']
    fields = {'seed', 'cases', 'faults', 'encoder', 'decoder', 'native_version'}
    for snapshot in (old, new):
        require(set(snapshot) == fields and type(snapshot['seed']) is int and snapshot['seed'] == 0x0741028, 'snapshot schema/seed')
        require([c['id'] for c in snapshot['cases']] == expected, 'exact ordered stream coverage')
        require([f['id'] for f in snapshot['faults']] == list(range(7)), 'exact ordered fault coverage')
        for fault in snapshot['faults']:
            require(set(fault) == {'id', 'input', 'accepted', 'message', 'remaining'} and
                    fault['accepted'] is False and isinstance(fault['message'], str) and fault['message'], 'loud malformed refusal')
    require(old['native_version'] == new['native_version'] == '1.5.7', 'original native Zstd version')
    require(old['cases'] == new['cases'], 'stream/payload/bound/cursor drift')
    diagnostics = []
    for before, after in zip(old['faults'], new['faults']):
        require({k:v for k,v in before.items() if k != 'message'} ==
                {k:v for k,v in after.items() if k != 'message'}, 'malformed decision/cursor drift')
        if before['message'] != after['message']:
            diagnostics.append(dict(baseline=before, candidate=after))
    for name in ('encoder', 'decoder'):
        require(set(old[name]) == set(new[name]) == {'output', 'isolated'}, 'context response schema')
        require(old[name]['isolated'] is False and new[name]['isolated'] is True, 'baseline exposure and candidate isolation')
        require(new[name]['output'] == b'second job'.hex(), 'exact new synthetic job')
        require(old[name]['output'] != new[name]['output'], 'baseline output differs')
    return dict(stream_cases=37, malformed_cases=7, context_controls=2, wire_bytes_equal=True,
                semantic_contract_pass=True, diagnostic_differences=diagnostics,
                diagnostic_review_required=bool(diagnostics), fixture_only=True, release_admitted=False)


def parse_context(raw):
    marker = 'HC074_ZSTD_CONTEXT:'
    lines = [line for line in raw.decode('utf-8').splitlines() if line.startswith(marker)]
    require(len(lines) == 1, 'one independent context observation')
    return json.loads(lines[0][len(marker):])


def compare_context(old, new):
    require(set(old) == set(new) == {'encoder', 'decoder'}, 'independent context schema')
    for name in ('encoder', 'decoder'):
        require(set(old[name]) == set(new[name]) == {'output', 'isolated'}, 'context fields')
        require(old[name]['isolated'] is False and new[name]['isolated'] is True, 'paired context isolation')
        require(new[name]['output'] == b'second job'.hex() and
                old[name]['output'] != new[name]['output'], 'exact new context output')
    return dict(encoder_repaired=True, decoder_repaired=True, full_wire_gate_pass=False,
                first_wire_failure_unchanged=True, fixture_only=True, release_admitted=False)
