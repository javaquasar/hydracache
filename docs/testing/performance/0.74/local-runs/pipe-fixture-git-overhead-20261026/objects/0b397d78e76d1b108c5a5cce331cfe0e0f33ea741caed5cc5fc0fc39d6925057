"""Fixed default-Windows package review after reader and owned-CI cleanup repairs.

This is a separate manual receipt, not a refresh of v5 or a runtime admission.
"""
import json
from pathlib import Path
import sys

import package_effects_review_v5 as previous

require, sha, legacy = previous.require, previous.sha, previous.legacy
REVIEWED_AST_SHA256 = '2552f3a6d96786433ea27b111a3f9f3b7274e82446634d68ca7d49090e24aa54'
PREVIOUS_SYNTAX = 'docs/testing/performance/0.74/local-runs/module-context-v5-20261026/objects/f3fc9b9f158a84334decac504c39a4f5d820a0a0265474dc91a7a3271d6bfc2d'
CHANGED = {
    'crates/xtask/tests/authority_rust_history_074.rs': '9333de6f4dff9c470eadff2a97dc3434ae8a760e51ec7de2b6256903ccef07e1',
    'crates/xtask/tests/fresh_status_reader_074.rs': '257eb0bd04c5bc20b0b37625f33c45a6dc6b56e95b7c8b3d71851f07b3ae88a9',
    'tools/authority-runtime-074/history_reader.rs': '3803936eb6944c949d9f5c04b1626eac9b61d570a3770e4c93413c2d235d5f08',
}
ADDED = {
    'crates/xtask/tests/history_status_readers_074.rs': '0c4b01d77f9808e3170cb7703231a4c380bab5c6db439afa7ce6b0b9aeea9e03',
    'tools/history-status-readers-074/contract.rs': 'a2e58267a81350b967036fc42a5a521d3cc2843ec98fb9e026ccac4eb3d714d0',
    'tools/history-status-readers-074/inputs.rs': '6588950833aa4ed0b73178ec0021bc94a5c338508b70f4b3017c0f700c1f9f06',
}
# Override only the explicitly reviewed heartbeat successor; retain all other
# previous auxiliary bytes. The new companion is an actual imported effect edge.
EXTRA_AUXILIARY = {
    'scripts/ci/run-with-heartbeat.py': '4fe6cd53a04e0d1e24cb60287fdad5317d1f20bc5472881b3bab28e37b2a965c',
    'scripts/ci/watchdog_windows_job.py': '8b392811e106eb7e692e21ee5b55cd04ea8ac421cad015edb66ce736b08c2ab1',
    'tools/history-status-readers-074/rules.json': '188763c292af6f6f603414ace264926009c59d7795cea1047084867609c77023',
    'tools/historical-blob-batch-074/package_effects_review_v5.py': '46a02302218d99e8fbf00c9d1ffad5d9c1b9b79dd56dc0641dea86f3a75ab805',
}
AUXILIARY = {**previous.AUXILIARY, **EXTRA_AUXILIARY}
DELTA_REVIEW = {
    'history': 'One exact current-input facade successor retains the old 9/81 rules, fixed Git command, full current-file comparisons and conditional IO. The unchanged runtime/history/cache code reads pinned historical text; no executable is revived.',
    'status': 'The complete expected reader gains its fixed kernel successor and reverse whole-body check. This adds one historical expected-input lookup through the existing cache, not a removed fresh observation or a performance win.',
    'new_target': 'Four ordinary source/mutation controls; direct filesystem reads, fixed read-only historical Git through unchanged helpers, caught expected panics and source AST inspection. Descriptor is inert include_str JSON, not executable input. No service or worker release is invoked.',
    'windows_ci': 'The existing twelve CI cases still launch cooperative Python fixtures. Heartbeat now imports the fixed Job companion: private noninheritable kill-on-close Job, atomic suspended job-list CreateProcessW, explicit bounded standard handles, retained original process/thread handles, membership check and one ResumeThread. Cleanup terminates only that Job and original child handle; root success requires active count zero. Both fresh five-second cleanup deadline checks remain. No taskkill, PID reacquisition, breakaway, service/clock/host operation or extra target is introduced by this companion.',
    'limits': 'The package outer wrapper is unchanged and still uses its historical cleanup, not this new Job. POSIX leader-exit behavior is unchanged and outside this Windows review. Cooperative CI containment is not hostile-writer retirement or production lifecycle authority.',
    'unchanged': '362 complete Rust source/AST rows and 41 old auxiliary inputs remain identical to v5. The default catalog adds exactly one ordinary target. Existing protoc, temp Git repositories, self/Python children and scratch operations retain the inherited review.',
    'scope': 'Manual default-Windows --lib --tests review, four threads, 780 seconds including compilation, no filter/ignored or diagnostic opt-in. Not compiler expansion, a sandbox, GNU/all-feature admission, a performance comparison or production authorization.',
}


def previous_document(root):
    path = root / PREVIOUS_SYNTAX
    raw = path.read_bytes()
    require(sha(raw) == path.name, 'fixed v5 syntax bytes drift')
    document = json.loads(raw)
    require(legacy.document_digest(document) == previous.REVIEWED_AST_SHA256, 'v5 syntax identity drift')
    return document


def source_review(root, document):
    root = root.resolve(strict=True)
    require(legacy.document_digest(document) == REVIEWED_AST_SHA256, 'unreviewed v6 syntax/source inventory')
    require(document['executed_input_code'] is False and document['compiler_expansion'] is False,
            'source-only inventory scope')
    rows = {row['source']: row for row in document['sources']}
    require(len(rows) == len(document['sources']) == 368, 'duplicate or changed source count')
    constructors = []
    for name, row in rows.items():
        path = (root / name).resolve(strict=True)
        require(path.is_relative_to(root) and path.suffix == '.rs' and sha(path.read_bytes()) == row['sha256'],
                'source bytes/path drift')
        for site in row['sites']:
            require(site['kind'] != 'include', 'unexpanded include edge')
            if site['kind'] in ('command-constructor', 'macro-command-constructor'):
                require(site['detail'] not in {'literal:ssh', 'literal:systemctl', 'literal:systemd-run',
                                              'literal:wsl', 'literal:timedatectl'}, 'host/service constructor requires separate review')
                constructors.append(dict(source=name, **site))
    additions, unresolved = previous.module_context_v5.module_edges(root, document)
    require(not additions and not unresolved, 'module graph not closed')
    old = {row['source']: row for row in previous_document(root)['sources']}
    require(len(old) == 365 and not old.keys() - rows.keys(), 'old source removal')
    changed = {name for name in old if old[name] != rows[name]}
    added = rows.keys() - old.keys()
    require({name: rows[name]['sha256'] for name in changed} == CHANGED and len(changed) == 3,
            'manual changed source/AST delta drift')
    require({name: rows[name]['sha256'] for name in added} == ADDED and len(added) == 3,
            'manual added source/AST delta drift')
    auxiliary = {name: sha((root / name).read_bytes()) for name in AUXILIARY}
    require(auxiliary == AUXILIARY and len(auxiliary) == 45, 'reviewed auxiliary input drift')
    targets = legacy.package_replay_audit.audit(root)['targets']
    old_targets = ['unittests src/lib.rs', 'unittests src/main.rs'] + sorted(
        'tests/' + Path(name).name for name in old if Path(name).parent == Path('crates/xtask/tests'))
    require(len(old_targets) == 144 and len(set(targets)) == len(targets) == 145 and
            targets == old_targets[:2] + sorted(old_targets[2:] + ['tests/history_status_readers_074.rs']),
            'default target catalog drift')
    return dict(schema='hc074-package-effect-review-v6', reviewed_ast_sha256=REVIEWED_AST_SHA256,
                previous_ast_sha256=previous.REVIEWED_AST_SHA256,
                sources={name: row['sha256'] for name, row in rows.items()}, auxiliary=auxiliary,
                targets=targets, constructors=constructors, changed_sources=sorted(changed),
                added_sources=sorted(added), delta_review=DELTA_REVIEW,
                inherited_delta_review=previous.DELTA_REVIEW, inherited_review=legacy.REVIEW,
                inherited_repair_review=previous.previous.previous.REPAIR_REVIEW,
                source_effect_review_complete=True, formal_whole_program_proof=False,
                package_dispatched=False, package_pass=False, linux_or_all_features_cleared=False,
                original_execution_or_product_or_qualification_admission=False)


environment_preflight = previous.environment_preflight


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[2]
    result = source_review(root, json.loads(Path(sys.argv[1]).read_bytes()))
    result['environment'] = environment_preflight(root)
    print(json.dumps(result, indent=2, sort_keys=True))
