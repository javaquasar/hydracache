"""Separate manual Windows package review after exact reader repairs.

Fixed source/AST and auxiliary pins below follow the 42-body delta review.
No CLI can learn an admission pin, alter the command or dispatch a package.
"""
import json
from pathlib import Path
import sys

import module_context_v5
import package_effects_review_v4 as previous

require, sha = previous.require, previous.sha
legacy = previous.previous.previous.legacy
REVIEWED_AST_SHA256 = '074a38382393cf290329a5018a71a5946348b024da3ed7e7e0ccd502bbac8733'
PREVIOUS_PACKET = 'docs/testing/performance/0.74/local-runs/package-effect-syntax-v4-closure-20261025'
PREVIOUS_INDEX_SHA256 = 'e46b334397fcb7ef6ffe36335fbe86a908be481dc12832109bf1da99b93ea6fb'
DELTA = {
    'crates/xtask/tests/artifact_inventory_phase_544_074.rs': '83b801e8f21b5d20ef1d92649450f364438ea9a7259d37b14e7cc2ace04b4344',
    'crates/xtask/tests/authority_hex_reader_074.rs': '8583b415df7c3f88881918b9d7a9528ca66e212f52e27159e4b3e09eb106ae84',
    'crates/xtask/tests/authority_reader_phase_074.rs': '04b3d1c95ec0a5ecfedf89a880e01c9a4a9cd843bdbf5118ce8fcb2f1da248f4',
    'crates/xtask/tests/authority_runtime_074.rs': 'fab409c900b1e3188d9ea5e4aaa40a2a33dde2c656dcf8e8b37dc522f82ebc60',
    'crates/xtask/tests/bootstrap_build_owner_074.rs': '61b2a36945b470de67c57da90c2a85169c88fd1c626fad5b076f38353e2046bc',
    'crates/xtask/tests/bootstrap_child_boundary_074.rs': '2fdb8eccff508532bb30173ea5b1cd8cb7fb12b1c52a3f9b2ba61626e6c7a739',
    'crates/xtask/tests/bootstrap_child_census_074.rs': '74ab14264aece844010f5ccdc82c99b06c07be3b3fcafc3d7285d438cd08f204',
    'crates/xtask/tests/bootstrap_child_reader_074.rs': '88a89f705032085c50906c7e06f60411c1cdd91f029ef140ebe42157465f8523',
    'crates/xtask/tests/bootstrap_current_image_074.rs': '435cc579ba5fdd856dae104835e5c298232fd4dbdb03a88b39ee812646b625e3',
    'crates/xtask/tests/bootstrap_doc_reader_074.rs': '586e748182ddc5ad701b85da3d35b55483e43e037dbea346036ccad47aff7aa3',
    'crates/xtask/tests/bootstrap_enrollment_074.rs': '0fc78fc37c7c98d45cf878ffac6a4dcbac2601ec0f92563e9430c9b051a6fb9a',
    'crates/xtask/tests/bootstrap_entry_binding_074.rs': 'd1e1018fa96cc863e72610a38d8e67a91ea50e75a55e77c4ae11369ff2423140',
    'crates/xtask/tests/bootstrap_image_census_074.rs': 'd1a654883f3ad9d906c79dbc1a13637816e5cd7c5354c536329e7603cd7b2953',
    'crates/xtask/tests/bootstrap_installed_origin_074.rs': '1913c9a0dc47701b999b8077ac55fc302b4c9264d21c48f79a530c385224d21d',
    'crates/xtask/tests/current_frontier_074.rs': '62b2b19f1c71cd9d3328e63aac6c300cdf85f80204047bb4ffac5c02a8a3c784',
    'crates/xtask/tests/current_tree_input_audit_074.rs': '99f97a36d1bc3e974dda573da731875f1d7af511dceb30aa7c6e2bcc101418cb',
    'crates/xtask/tests/current_tree_reader_074.rs': '8354282f75a1cff07b5bdad8c5ecf50e4fb50d24e26a6d5a53935d4f8669b89d',
    'crates/xtask/tests/fresh_kernel_cost_074.rs': '5d96ab2d3de23744fcc8970efcf8fd6add4a62bb7af8eec3deccb649a095667b',
    'crates/xtask/tests/held_activation_ingress_074.rs': 'e895b3f0fe09678d39201840124742a18ce2aea98045006c1dd3fe90c9a53541',
    'crates/xtask/tests/historical_input_view_074.rs': '4d5b7bc62cccc129367f6b6f86a5013588b933a6c25c58697112ebf79510b420',
    'crates/xtask/tests/owned_case_074.rs': '6161ee41edfbe03b2390a33fc0d7783ec3509f98a52a1f013a914f467e650120',
    'crates/xtask/tests/remaining_readers_074.rs': 'c18d2549236de288609abd7d001b080f50f7de802f54da78f10471eaf8ff6428',
    'crates/xtask/tests/source_successor_view_074.rs': '24d2886ed060075a1117eaa0bb17eb8ca594a6a4c124608d8a19e64465a7b3de',
    'crates/xtask/tests/support/artifact_inventory_input.rs': '050bec3b7dcdc169ceedbb4a6933c6f52c6f5d39aea4c241180380cbf8327605',
    'crates/xtask/tests/support/artifact_inventory_phase_544.rs': '592a3e359201323501f87301bce55e385c516527097db4220ea517dfc0318df9',
    'crates/xtask/tests/support/document_scratch_inputs.rs': '753e0400c824468f76666b40e3326af2d605bf1d81af8dd1c382a0a5770da1e6',
    'tools/authority-reader-phase-074/contract.rs': '78ccbfc0bef5349cb324343472bc9d1cf3ea041c2cf365ae8fc748fe1ee94fc7',
    'tools/authority-reader-phase-074/main.rs': '3deda541166414b4cce7cfe9102f6777052f7b310b08a44ef081cbcadaea53eb',
    'tools/authority-runtime-074/gate.rs': '6eb0e78bc5b8cb4f7324747c3d0b78b0113c6a58ac9af2c1871d9fbf267b1363',
    'tools/bootstrap-doc-reader-074/contract.rs': '38ed33f48332bbefc4e153433d40cde7730c007b4efacbecd632a032baf15ced',
    'tools/bootstrap-doc-reader-074/installed_io.rs': '9aac3ff31b2778264d66325fe7d8c3b5659d5957e9c4c2be880a7799dbfa2472',
    'tools/bootstrap-doc-reader-074/io.rs': 'c3200aab126abfcf059fe6c79509938aeb860bf0eefbb664e2a1d6a4b2a45821',
    'tools/bootstrap-doc-reader-074/proof.rs': '736f70ea64ec11ea82dfac8e505468a4f429c0c2ff6b067b5059547181689947',
    'tools/bootstrap-doc-reader-074/read.rs': '1cffd334559b2e2a265d0bbafda3853b116f200cc6e2bdf5cf80c6d2af37e3ba',
    'tools/bootstrap-doc-reader-074/reader.rs': 'ec21e788a65b921f0b6b4707c9e4919390ed9247a8b152d22862c6dbe9577f6c',
    'tools/current-frontier-074/additions.rs': '8fdb366c6bade0767a7bdd3b4b9570dcb8e6af036d156836d23c828a4699b9cc',
    'tools/current-frontier-074/contract.rs': 'd214be91c11ec2f93f76ca20ea29deb697dbaffaac4aceddfcc62131c5b2caae',
    'tools/current-frontier-074/reader.rs': '3fc4f7da5124b18a418c69d4a8e7e2af855e085c9409d4ceaa3a1e590117870f',
    'tools/historical-input-view-074/contract.rs': '8c09a9e9e694fb939ffef27cc27b73ace09323f56d85dd8944f145971c056a92',
    'tools/remaining-readers-074/contract.rs': '23c985301e96e572c815f50a71397c1eeb9791f6d684b70d7dc08ce25d938a8c',
    'tools/remaining-readers-074/inputs.rs': '96651cff002a24f192526b4a619376a6229bb45ee5d1e791640c47ef94b42c88',
    'tools/source-successor-view-074/contract.rs': 'd0fea549848e1a0d51f424d12423e6765c2f9f16d6cf55b1c9659a8d173dd75e',
}
EXTRA_AUXILIARY = {
    'tools/historical-blob-batch-074/package_effects_review_v3.py': 'b82ade2053bea6532efe4483c4dc51d5cc4f9aeb02cebee1b5a7fff4fc5ac0b3',
    'tools/historical-blob-batch-074/package_effects_review_v4.py': 'bdd999e6a33b6daadcd233c6073c69507e864e845f5fae1bfe0e61ac9959d50b',
    'tools/historical-blob-batch-074/module_context_v5.py': 'c4bbc2d6b69e416ac3fcb30a1a64fb3bb54473b70977a1875ff90ca264b0cd44',
    'tools/historical-blob-batch-074/package_replay_v2.py': 'c0bae2f0d8b1f7b90d61aa5da39ef8bcb7187ac2b7268fbc57a5cc78b083611f',
    'crates/xtask/tests/support/original_service_gate.txt': 'f16badb4ce2044d91d3d20f1f8471216f990d754294d6ab7a4344632922012cf',
    'tools/authority-runtime-074/filesystem.rs': '4555af85ed9304f4f19687619e6dc5ec5a73f55a3159a1cd53c36909df0ed0c4',
    'docs/testing/performance/0.74/local-runs/role-census-inventory-20261024/objects/cfd5e7f1c153f5214dd71da196dfdbac2d1c05806fa14865605e9ce0776bc9c0': 'cfd5e7f1c153f5214dd71da196dfdbac2d1c05806fa14865605e9ce0776bc9c0',
    'docs/testing/performance/0.74/local-runs/inventory544-provenance-20261026/objects/fec2195d38bd7784e9c584704b90d5851f5a5c228a900513aa11cf91b42fad8f': 'fec2195d38bd7784e9c584704b90d5851f5a5c228a900513aa11cf91b42fad8f',
    'tools/authority-reader-phase-074/main-rules.json': '9373e36370e3893316a983fb159debba29afbbf4095c86f8f71d4384c80cc4f5',
    'tools/bootstrap-doc-reader-074/rules.json': 'f780c166ff6e708eb2b09013075e414426bad8ae443f537a5ad0a08dcebfb3ce',
    'tools/source-successor-view-074/rules.json': '61d3e1941cdae497600592a49af0487e4cf545573ba3f091add173110b560cda',
    'tools/current-frontier-074/rules.json': '4ef20240b953381c2e4dbfe7892b0a9205dbdefba3012e89a113030372d0d389',
    'tools/remaining-readers-074/rules.json': 'b8803a5f12460fa2278ffef033dc0738c4dc925a65a661f701ea09ae9bb6883e',
}
AUXILIARY = {**previous.previous.AUXILIARY, **EXTRA_AUXILIARY}
DELTA_REVIEW = {
    'inventory': '544 current entries and original 530/446 projections. New producer data is inert JSON. Git ls-tree/ls-files/show and physical source reads only; original metadata/index/body guards remain. No writes to source or supervisor.',
    'authority': 'Five exact producer-to-predecessor text projections, separate current fixture entry, retired-main test text and fixed whole-caller constructors. Existing frozen Git-object reads/cache; historical text is never compiled or executed.',
    'bootstrap': 'Two selected document reads use one physical observation and an exact pinned predecessor before old full assertions. Other paths/IO errors unchanged. No bootstrap, manager, transport or child entry is invoked.',
    'frontier': 'Same protected input selection, original 9/81 gate plus twelve fixed source transitions and fourteen exact index/body additions. Current-root observations remain fresh; expected historical bodies may use existing cache. No runtime permission follows.',
    'remaining': 'Fresh-kernel and activation source tests retain their condition/read order and every old assertion. A seven-file exact caller-version join precedes unchanged old predicates; unknown/stale/partial bodies refuse.',
    'six_new_targets': 'Finite source/model/mutation controls. Added Git constructors use explicit read-only show/ls-tree/ls-files arguments. Only current_frontier writes/removes a supplied relative file inside its own tempfile root; no external directory is accepted.',
    'unchanged_effects': 'All 323 other AST/source rows and 29 prior auxiliary pins are identical to v4. Existing inherited self-child/protoc/temp-repository/Python controls remain in the prior review. No new measurement/host/clock/service/execution entry is enabled.',
    'scope': 'Default Windows --lib --tests, no filter/ignored, four threads, original 780-second bound including compilation. Manual entry-point review plus complete pinned syntax/module inventory is not macro expansion, a sandbox, GNU/all-feature clearance or production admission.',
}


def previous_document(root):
    folder = root / PREVIOUS_PACKET
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == PREVIOUS_INDEX_SHA256, 'v4 packet identity drift')
    row = json.loads(raw)['members']['syntax/result.json']
    raw = (folder / 'objects' / row['sha256']).read_bytes()
    require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'v4 syntax bytes drift')
    document = json.loads(raw)
    require(legacy.document_digest(document) == previous.REVIEWED_AST_SHA256, 'v4 review identity drift')
    return document


def source_review(root, document):
    root = root.resolve(strict=True)
    require(legacy.document_digest(document) == REVIEWED_AST_SHA256, 'unreviewed v5 syntax/source inventory')
    require(document['executed_input_code'] is False and document['compiler_expansion'] is False,
            'source-only inventory scope')
    rows = {row['source']: row for row in document['sources']}
    require(len(rows) == len(document['sources']) == 365, 'duplicate or changed source count')
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
    additions, unresolved = module_context_v5.module_edges(root, document)
    require(not additions and not unresolved, 'module graph not closed')
    old = {row['source']: row for row in previous_document(root)['sources']}
    require(len(old) == 344 and not old.keys() - rows.keys(), 'old source removal')
    changed = {name for name in old if old[name] != rows[name]}
    added = rows.keys() - old.keys()
    require(len(changed) == len(added) == 21, 'unreviewed source/AST delta')
    require({name: rows[name]['sha256'] for name in changed | added} == DELTA and len(DELTA) == 42,
            'manual delta pins drift')
    auxiliary = {name: sha((root / name).read_bytes()) for name in AUXILIARY}
    require(auxiliary == AUXILIARY and len(auxiliary) == 42, 'reviewed auxiliary input drift')
    targets = legacy.package_replay_audit.audit(root)['targets']
    old_targets = ['unittests src/lib.rs', 'unittests src/main.rs'] + sorted(
        'tests/' + Path(name).name for name in old if Path(name).parent == Path('crates/xtask/tests'))
    new_targets = sorted('tests/' + Path(name).name for name in added
                         if Path(name).parent == Path('crates/xtask/tests'))
    require(len(old_targets) == 138 and len(new_targets) == 6 and len(set(targets)) == len(targets) == 144 and
            targets == old_targets[:2] + sorted(old_targets[2:] + new_targets), 'default target catalog drift')
    return dict(schema='hc074-package-effect-review-v5', reviewed_ast_sha256=REVIEWED_AST_SHA256,
                previous_ast_sha256=previous.REVIEWED_AST_SHA256, sources={name: row['sha256'] for name, row in rows.items()},
                auxiliary=auxiliary, targets=targets, constructors=constructors,
                changed_sources=sorted(changed), added_sources=sorted(added), delta_review=DELTA_REVIEW,
                inherited_review=legacy.REVIEW, inherited_repair_review=previous.previous.REPAIR_REVIEW,
                source_effect_review_complete=True, formal_whole_program_proof=False,
                package_dispatched=False, package_pass=False, linux_or_all_features_cleared=False,
                original_execution_or_product_or_qualification_admission=False)


environment_preflight = previous.environment_preflight


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[2]
    result = source_review(root, json.loads(Path(sys.argv[1]).read_bytes()))
    result['environment'] = environment_preflight(root)
    print(json.dumps(result, indent=2, sort_keys=True))
