"""Offline command shapes and a bounded manual call-site ledger; no Git execution."""
from collections import Counter
import hashlib
import io
import json
from pathlib import Path
import re
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/xtask-git-trace-074'))
import trace_review

INDEX = '11e89b85754443b1c5ff818ebfa4e5fcd2e896d812a85065cac2ffe70389e085'
SITES = (
    ('tools/authority-runtime-074/gate.rs', 'fn load_frozen(',
     'full40_asserted_no_replace', 'conditional cached lookup; no preplanned batch; uppercase assertion broader than candidate'),
    ('crates/xtask/tests/support/census_provider_input.rs', 'fn frozen(',
     'two_full_constants_no_replace', 'conditional original/accepts/view calls; compare and error schedule must remain'),
    ('crates/xtask/tests/support/performance_contract_074_closing_validation.rs', 'fn git(',
     'short_literal_default_replacements', '418-source loop plus extras; preserve short resolution, replacement and current-read order'),
    ('crates/xtask/tests/support/performance_contract_074_collector_notices.rs', 'fn collector_notice_tests_',
     'short_literal_default_replacements', 'four fixed sources; different Git identity/flag semantics'),
    ('crates/xtask/tests/support/performance_contract_074_boot_buffer.rs', 'fn old(',
     'full_constant_default_replacements', '353-source loop; no disabling of replacements in the original loader'),
    ('crates/xtask/tests/support/performance_contract_074_fixed_grouped.rs', 'fn frozen(',
     'short_literal_default_replacements', 'conditional source comparisons and mutation controls; no silent short-ID expansion'),
)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def command_shape(argv):
    if not isinstance(argv, list) or not argv or any(not isinstance(a, str) for a in argv):
        raise ValueError('invalid argv')
    args = argv[1:]
    flag = args[:1] == ['--no-replace-objects']
    if flag:
        args = args[1:]
    if len(args) != 2 or args[0] != 'show' or ':' not in args[1]:
        return 'other_show_shape'
    revision, path = args[1].split(':', 1)
    if not path or any(c in path for c in '\0\r\n\\'):
        return 'other_show_shape'
    suffix = 'no_replace' if flag else 'default_replacements'
    if re.fullmatch('[0-9a-f]{40}', revision):
        return 'full40_' + suffix
    if re.fullmatch('[0-9a-f]{4,39}', revision):
        return 'short_' + suffix
    return 'other_revision_' + suffix


def reviewed_sites():
    entries = []
    for path, marker, shape, barrier in SITES:
        raw = (ROOT / path).read_bytes()
        text = raw.decode().replace('\r\n', '\n')
        if text.count(marker) != 1:
            raise ValueError('manual ledger anchor drift: ' + path)
        start = text.index(marker)
        end = text.index('\n}', start) + 2
        entries.append(dict(path=path, source_sha256=sha(raw),
            line=text[:start].count('\n') + 1, excerpt=text[start:end],
            reviewed_shape=shape, integration_barrier=barrier, admitted=False))
    return entries


def audit():
    folder = ROOT / 'docs/testing/performance/0.74/local-runs/static-contract-trace-20261024'
    raw = (folder / 'index.json').read_bytes()
    if sha(raw) != INDEX:
        raise ValueError('sealed trace index changed')
    index = trace_review.decode(raw)

    def member(name):
        entry = index['members'][name]
        data = (folder / 'objects' / entry['sha256']).read_bytes()
        if len(data) != entry['bytes'] or sha(data) != entry['sha256']:
            raise ValueError('sealed trace member changed')
        return data

    parser = member('source/current/tools/xtask-git-trace-074/trace_review.py')
    if parser != (ROOT / 'tools/xtask-git-trace-074/trace_review.py').read_bytes():
        raise ValueError('trace parser changed')
    ledger = trace_review.decode(member('capture/git-trace2-members.json'))
    archive = member('capture/git-trace2.tar.gz')
    counts, seen, sids = Counter(), set(), set()
    roots = 0
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz') as stream:
        for item in stream:
            if not item.isfile() or item.name not in ledger or item.name in seen:
                raise ValueError('unexpected archive member')
            seen.add(item.name)
            expected = ledger[item.name]
            if item.size != expected['bytes']:
                raise ValueError('archive length drift')
            body = stream.extractfile(item).read()
            if sha(body) != expected['sha256']:
                raise ValueError('archive hash drift')
            if item.name.startswith('target-trace/'):
                session = trace_review.session(body.decode())
                if session['sid'] in sids:
                    raise ValueError('duplicate session')
                sids.add(session['sid'])
                if not session['nested']:
                    roots += 1
                    if session['command'] == 'show':
                        counts[command_shape(session['argv'])] += 1
    if seen != set(ledger) or roots != 12424 or sum(counts.values()) != 12383:
        raise ValueError('trace coverage drift')
    return dict(schema='hc074-historical-batch-eligibility-v1', input_index_sha256=INDEX,
        archive_sha256=sha(archive), parser_sha256=sha(parser), verified_raw_members=len(seen),
        root_sessions=roots, show_shapes=dict(sorted(counts.items())),
        sites=reviewed_sites(), site_ledger_exhaustive=False,
        trace_shapes_prove_eligibility=False, revision_resolution=False,
        runtime_integrated=False, measurement=False, package_retry=False,
        live_frequency_or_deadlines_changed=False, server=False, admission=False)


if __name__ == '__main__':
    print(json.dumps(audit(), indent=2, sort_keys=True))
