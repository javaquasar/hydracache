"""Manual Windows package review after GET-owner retirement; old pins stay fixed."""
import json
from pathlib import Path
import sys

import package_effects_review_v3 as previous

require, sha = previous.require, previous.sha
REVIEWED_AST_SHA256 = '5239862a6d46ae56c30468a2cce3a0bc8d5c279d1749051870370f6effa97726'
PREVIOUS_PACKET = 'docs/testing/performance/0.74/local-runs/package-effect-syntax-v3-20261025'
PREVIOUS_INDEX_SHA256 = '1e461457bb2448b9caabfe396b0e9999ec8f34e28522036dd4c7dd4296bac0aa'
DELTA = {
    'crates/xtask/tests/get_owner_terminal_074.rs': 'b825d466e1b14dd50c00ba520e21aa1fa42835a38fbe4112713603bc1d534d19',
    'crates/xtask/tests/performance_contract_074.rs': '30dc4166f86f71a2028b0adc3287e898920adf18c92a45cb7046ab67663f9207',
}
DELTA_REVIEW = {
    'terminal_test': 'One read-only source/TOML/SHA256 regression: canonical runtime digest, retired selector, exact unchanged allocation thresholds and terminal registry. No child, service, clock, network or mutation entry point.',
    'contract': 'Only the historical overlay projection changes from two active cfg hunks to one exact cfg(test) module. Active selector/feature strings are forbidden; canonical runtime hash and original cases remain. No new IO or execution entry point.',
    'scope': 'Same default Windows package, --lib --tests, no filters/ignored, four test threads and original 780-second wrapper. No observer or performance cohort. Manual syntax review is not a sandbox, Linux/all-feature clearance or production admission.',
}


def previous_document(root):
    folder = root / PREVIOUS_PACKET
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == PREVIOUS_INDEX_SHA256, 'v3 packet identity drift')
    row = json.loads(raw)['members']['syntax/result.json']
    raw = (folder / 'objects' / row['sha256']).read_bytes()
    require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'v3 syntax bytes drift')
    document = json.loads(raw)
    require(previous.previous.legacy.document_digest(document) == previous.REVIEWED_AST_SHA256,
            'v3 review identity drift')
    return document


def source_review(root, document):
    root = root.resolve(strict=True)
    legacy = previous.previous.legacy
    require(legacy.document_digest(document) == REVIEWED_AST_SHA256, 'unreviewed v4 syntax/source inventory')
    require(document['executed_input_code'] is False and document['compiler_expansion'] is False,
            'source-only inventory scope')
    additions, unresolved = legacy.syntax_audit.module_edges(root, document)
    require(not additions and not unresolved, 'module graph not closed')
    rows = {row['source']: row for row in document['sources']}
    require(len(rows) == len(document['sources']) == 344, 'duplicate or changed source count')
    constructors = []
    for name, row in rows.items():
        path = (root / name).resolve(strict=True)
        require(path.is_relative_to(root) and path.suffix == '.rs' and sha(path.read_bytes()) == row['sha256'],
                'source bytes/path drift')
        for site in row['sites']:
            require(site['kind'] != 'include', 'unexpanded include edge')
            if site['kind'] in ('command-constructor', 'macro-command-constructor'):
                require(site['detail'] not in {'literal:ssh', 'literal:systemctl', 'literal:systemd-run',
                                              'literal:wsl', 'literal:timedatectl'},
                        'host/service constructor requires separate review')
                constructors.append(dict(source=name, **site))
    old = {row['source']: row for row in previous_document(root)['sources']}
    require(len(old) == 343 and not old.keys() - rows.keys(), 'old source removal')
    changed = {name for name in old if old[name] != rows[name]}
    added = rows.keys() - old.keys()
    require(changed == {'crates/xtask/tests/performance_contract_074.rs'} and
            added == {'crates/xtask/tests/get_owner_terminal_074.rs'}, 'unreviewed source/AST delta')
    require({name: rows[name]['sha256'] for name in changed | added} == DELTA, 'manual delta pins drift')
    auxiliary = {name: sha((root / name).read_bytes()) for name in previous.AUXILIARY}
    require(auxiliary == previous.AUXILIARY and len(auxiliary) == 29, 'reviewed auxiliary input drift')
    targets = legacy.package_replay_audit.audit(root)['targets']
    require(len(set(targets)) == len(targets) == 138 and
            targets.count('tests/get_owner_terminal_074.rs') == 1, 'default target catalog drift')
    return dict(schema='hc074-package-effect-review-v4', reviewed_ast_sha256=REVIEWED_AST_SHA256,
                previous_ast_sha256=previous.REVIEWED_AST_SHA256,
                sources={name: row['sha256'] for name, row in rows.items()}, auxiliary=auxiliary,
                targets=targets, constructors=constructors, delta_review=DELTA_REVIEW,
                inherited_review=legacy.REVIEW, inherited_repair_review=previous.REPAIR_REVIEW,
                source_effect_review_complete=True, formal_whole_program_proof=False,
                package_dispatched=False, package_pass=False, linux_or_all_features_cleared=False,
                original_execution_or_product_or_qualification_admission=False)


environment_preflight = previous.previous.environment_preflight


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[2]
    result = source_review(root, json.loads(Path(sys.argv[1]).read_bytes()))
    result['environment'] = environment_preflight(root)
    print(json.dumps(result, indent=2, sort_keys=True))
