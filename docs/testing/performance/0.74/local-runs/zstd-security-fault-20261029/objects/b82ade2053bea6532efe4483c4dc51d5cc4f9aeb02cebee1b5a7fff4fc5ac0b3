"""Independent repaired-source Windows effect scope; no execution dispatcher."""
from pathlib import Path
import json
import sys

import package_effects_review_v2 as previous

REVIEWED_AST_SHA256 = 'ec5fdf083b8f489f1ee5b51c646322b705bf502669d80f4dfe4e874100de4b75'
ADDED = {
    'crates/xtask/tests/direct_output_cost_074.rs': '0c8740ec73b56b3f791fc0b25ca879327dc23bcfb45a781fd5a0d1356a913b8d',
    'crates/xtask/tests/package_diagnostics_074.rs': '6b7c8ee178d4e849feaa20d543813564a14dfa2e378cab663f52082b454f87ff',
    'tools/guard-path-comparison-074/contract.rs': 'b24e9938aa5af4c9f45f1a542d21aac22da6adb168465df490d48c8c32d084c5',
    'tools/normalization-attribution-074/history_probe.rs': '7ca89559130b3b8271f59c39f039ad7622ea2cecf1a2bc0e31472c9e4eba0dc1',
    'tools/normalization-attribution-074/probe.rs': '10681270967fb942e56944ac95bdf5a86d4891d63d0a69104c3ef2b1e4c30ec7',
    'tools/xtask-package-attribution-074/helper_contract.rs': 'b3a6c8bdac36154f51fa1e26cea24f44a93ae9f02d2ff1f528518b739ee06a13',
    'tools/xtask-package-attribution-074/helper_model.rs': '137db5f7bfb7fc6146b8ace70143fec8a1515f97eda0d49fb00e543ba83c5907',
    'tools/xtask-package-attribution-074/helper_trace.rs': '91aff09249e6d8caa8df4bd49769015006f2701b9396167ba7775106d38ef029',
    'tools/xtask-package-attribution-074/pipe_capture.rs': '7b81ec42ec234dc6bb3d98fc551ea4e7dbc40d986a18d14cfb8d639c1a7587d3',
}
AUXILIARY = {**previous.legacy.REVIEWED_AUXILIARY, **previous.EXTRA_AUXILIARY,
    'tools/historical-blob-batch-074/package_effects_review_v2.py':
        '9e590fb92e218f67e39b2d8b0dd3ef49a98e1e0bd7f38473edb23bfcab374a23'}
REPAIR_REVIEW = {
    'old_sources': 'All 334 original AST rows and raw source hashes are identical to the independent v1 packet. Artifact helper, authority context, fixed-slice predicate and gate are restored, not normalized through new admission exceptions.',
    'diagnostic_target': 'Sixteen relocated tests, no duplicated original authority tests. Thirteen bodies unchanged, one module-alias-only, two require observers/path candidate not installed. Source/pure-model/protocol controls only; opt-in cost/fixture entries return without inherited HC074_* inputs. Same original namespace/document module chain, no duplicate exported macros.',
    'standalone_candidate': 'Borrowed comparison exists only in the isolated contract/probe. Ordinary mutation controls require its rejection by the restored installed gate. No product path or skipped observation.',
    'unchanged_additions': 'The eight previously reviewed added Rust sources retain their exact v2 bytes. Pipe semantic cases retain self-only children and missing-executable controls. No allocation/timing cohort enabled.',
    'clearance': 'Bounded manual syntax/entry-point review only, not a sandbox, compiler-expansion theorem, Linux/all-feature test clearance, package pass, live original delivery, retirement or qualification admission.',
}
require, sha = previous.require, previous.sha


def source_review(root, document):
    root = root.resolve(strict=True)
    require(previous.legacy.document_digest(document) == REVIEWED_AST_SHA256, 'unreviewed v3 syntax/source inventory')
    require(document['executed_input_code'] is False and document['compiler_expansion'] is False, 'source-only inventory scope')
    additions, unresolved = previous.legacy.syntax_audit.module_edges(root, document)
    require(not additions and not unresolved, 'module graph not closed')
    rows = {row['source']:row for row in document['sources']}
    require(len(rows) == len(document['sources']) == 343, 'duplicate or changed source count')
    constructors = []
    for name, row in rows.items():
        path = (root/name).resolve(strict=True)
        require(path.is_relative_to(root) and path.suffix == '.rs' and sha(path.read_bytes()) == row['sha256'], 'source bytes/path drift')
        for site in row['sites']:
            require(site['kind'] != 'include', 'unexpanded include edge')
            if site['kind'] in ('command-constructor','macro-command-constructor'):
                require(site['detail'] not in {'literal:ssh','literal:systemctl','literal:systemd-run','literal:wsl','literal:timedatectl'}, 'host/service constructor requires separate review')
                constructors.append(dict(source=name, **site))
    original = {row['source']:row for row in previous.previous_document(root)['sources']}
    require(len(original) == 334 and all(rows.get(name) == row for name,row in original.items()), 'original source/AST drift')
    require({name:rows[name]['sha256'] for name in rows.keys()-original.keys()} == ADDED and len(ADDED) == 9, 'manual added-source pins drift')
    auxiliary = {name:sha((root/name).read_bytes()) for name in AUXILIARY}
    require(auxiliary == AUXILIARY and len(auxiliary) == 29, 'reviewed auxiliary input drift')
    targets = previous.legacy.package_replay_audit.audit(root)['targets']
    require(len(set(targets)) == len(targets) == 137 and all(targets.count('tests/'+name+'.rs') == 1
        for name in ('package_diagnostics_074','direct_output_cost_074','performance_contract_074')), 'default target catalog drift')
    return dict(schema='hc074-package-effect-review-v3',
        reviewed_ast_sha256=REVIEWED_AST_SHA256, original_ast_sha256=previous.PREVIOUS_AST_SHA256,
        scope='Windows default --lib --tests; no --ignored; original four case threads/780-second package bound',
        sources={name:row['sha256'] for name,row in rows.items()}, auxiliary=auxiliary,
        added_sources=sorted(ADDED), changed_original_sources=[], constructors=constructors, targets=targets,
        inherited_review=previous.legacy.REVIEW, repair_review=REPAIR_REVIEW,
        source_effect_review_complete=True, formal_whole_program_proof=False,
        linux_or_all_features_cleared=False, package_dispatched=False, package_pass=False,
        original_execution_or_product_or_qualification_admission=False)


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[2]
    result = source_review(root, json.loads(Path(sys.argv[1]).read_bytes()))
    result['environment'] = previous.environment_preflight(root)
    print(json.dumps(result,indent=2,sort_keys=True))
