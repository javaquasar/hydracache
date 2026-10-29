"""Separate fixed default-Windows review; never refreshes V6 or dispatches tests.

The retained V6 receipt is data. Project Python is imported only after all its
inherited auxiliary bytes are checked. No pin-learning or filter CLI exists.
"""
import hashlib
import importlib.util
import json
from pathlib import Path
import tomllib

REVIEWED_AST_SHA256 = '95441e92c2072e08c34e73b4dcfe59d66ae7fe6728b014d54f3c7c15dc4be8f7'
PREVIOUS_AST_SHA256 = '2552f3a6d96786433ea27b111a3f9f3b7274e82446634d68ca7d49090e24aa54'
PREVIOUS_SYNTAX = 'docs/testing/performance/0.74/local-runs/package-review-v6-20261026/objects/e8c6a3141d7381fb024e712e0287a980bc0e9d062597bac437558dbce3bd6996'
PREVIOUS_PACKET = 'docs/testing/performance/0.74/local-runs/package-closure-v6-20261026'
PREVIOUS_INDEX_SHA256 = 'd194f84a6258a060156271d6704af3f38f3f5d7590e0f9f8ad672b531abacbd7'
PREVIOUS_GATE_SHA256 = '1a789f972418ffa531dde23ac2400651e0d55b6b30c664928e813db9dd9d5075'
CHANGED = {
    'crates/xtask/tests/direct_output_cost_074.rs': '17ece7e0fb740aca1a0626a087427b24e0126a793361099eecbb67f15ea0cd1e',
    'crates/xtask/tests/performance_contract_074.rs': '0565355c592d59d49a7c2a1b1631f10711ff5d6101e42cf50925261a6e4bed6c',
    'crates/xtask/tests/source_successor_view_074.rs': '3e1ff0e727d676ea84879282e251c46358acdb8f30c8ee14b9ffe39371097ff3',
    'tools/source-successor-view-074/contract.rs': '185714b442986a81bc0e2f9b6f8107a8151c78bee768a9594b6244ddebdbca4c',
}
ADDED = {
    'tools/source-successor-view-074/diagnostic.rs': '71e7926afafdb181aa65cb614a9eb0ec21f18c1e43a3d70f77208418cb8b9fee',
    'crates/xtask/tests/security_input_view_074.rs': 'f038c06a075081ec2ab137dc7c083ddeaf16f78bd92f5a556812210ceaea377b',
    'tools/security-input-view-074/contract.rs': 'ae4138993c239ff11032ef333c1cb40c487badd9ea65ea4d1a40b99423071b4c',
    'crates/xtask/tests/git_descendant_cpu_074.rs': 'b9e775a0a63662b364f544c01de58ca9b1f90b521e7d2100440976e276da2624',
}
AUXILIARY_SUCCESSORS = {
    'Cargo.lock': '91a009119e13184958de1a29f3b58d900e8756aaa47878c991a114b5ea2079d0',
    'Cargo.toml': 'd33222d3205112762984df4088523e5acb7b5fcb3f2afaf913e8c31f2bbed6d8',
    'scripts/ci/run-with-heartbeat.py': 'cedf938f80b3697efa219e5e8566d5d4f869cc2769dc7de6a176efb6e3af7afb',
}
EXTRA_AUXILIARY = {
    'scripts/ci/watchdog_linux_group.py': 'e5ff52835fc2486171dbdbbb1b6df47177f583b4539f561779992c2c5065a691',
    'tools/security-input-view-074/rules.json': '279a9a8c88039fd2224618c0f406d0d4c48eb3135c76560c1981b615c3dd9bc2',
    'tools/source-successor-view-074/diagnostic-additions.txt': '18d06dbd21744e236d90bc7004fb4fad531ea8c8342d5c8a917e0fe7e8627160',
    'docs/COMPAT.md': 'c4c236af9b4ba357885f36c791f5dcb6369fe7124a85b73d0c537b304c87d6b5',
    'docs/testing/performance/0.74/qualification-manifest.toml': '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc',
    'deny.toml': '16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930',
}
DELTA_REVIEW = {
    'git_cpu_target': 'Two diagnostic children require exact single-thread child argv; default four-thread unfiltered package returns before CPU/Git capture. Ordinary cases inspect bounded vectors and a fixed include. Existing pipe_capture remains unchanged.',
    'pipe_semantics': 'Self-only synthetic child flushes exact streams and exits before the libtest footer. Independent one-MiB-per-stream oracle rejects altered bytes/status/footer; raw captures are not normalized. Existing six self-child calls, empty/nonzero/large cases and absent-executable controls remain. No Git cohort is enabled.',
    'security_view': 'Whole current-file hash, exact inverse byte edits and whole predecessor hash for four allowlisted source-test inputs. Cached data are immutable compiled rules only. Root lock observation now uses the existing fresh IO facade. Historical terminal rejection text is data, never executable. New seven tests use fixed historical Git, source/mutation/parse controls and expected caught panics; no worker or manager action.',
    'diagnostic_view': 'Whole-document diagnostic compatibility predecessor reverses exactly the reviewed 2395-byte insertion. Fragment is inert include_str data. Existing five-path rules and fresh observation behavior remain unchanged.',
    'security_graph': 'Previously accepted three locked TLS/WebPKI/Tokio updates and two minimum versions; MSRV 1.88 and roots unchanged. Dependency build/test behavior still requires the new package result. No security-policy or product-nonregression pass follows from source review.',
    'heartbeat': 'Windows still selects the unchanged WindowsJob companion. New LinuxGroup import is inert on Windows; prctl/pidfd operations occur only in the POSIX constructor/methods. Linux subreaper cleanup has separate focused evidence, not default-Windows or production all-writer authority. Both fresh five-second cleanup checks and original origin remain.',
    'inherited': '364 complete Rust source/AST rows and 42 prior auxiliary bodies are identical. Complete inherited manual reviews remain in the sealed V6 gate; they are not recomputed by the old lexical scanner. Two ordinary auto-discovered test targets are added.',
    'scope': 'Default Windows --lib --tests, four threads, 780 seconds including compilation. No ignored/filter/diagnostic opt-in, macro expansion, GNU/all-feature admission, performance claim, original execution, server operation or qualification.',
}


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def decode(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs)


def document_digest(document):
    return sha(json.dumps(document, sort_keys=True, separators=(',', ':')).encode())


def read_inside(root, name):
    relative = Path(name)
    require(not relative.is_absolute() and '..' not in relative.parts, 'nonrelative review input')
    path = (root / relative).resolve(strict=True)
    require(path.is_relative_to(root), 'review input outside repository')
    return path.read_bytes()


def previous_gate(root):
    raw = read_inside(root, PREVIOUS_PACKET + '/index.json')
    require(sha(raw) == PREVIOUS_INDEX_SHA256, 'V6 index drift')
    row = decode(raw)['members']['coordinator/preflight-first/gate.json']
    require(row['sha256'] == PREVIOUS_GATE_SHA256, 'V6 gate pointer drift')
    raw = read_inside(root, PREVIOUS_PACKET + '/objects/' + PREVIOUS_GATE_SHA256)
    require(len(raw) == row['bytes'] and sha(raw) == PREVIOUS_GATE_SHA256, 'V6 gate bytes drift')
    gate = decode(raw)
    require((len(gate['sources']), len(gate['auxiliary']), len(gate['targets'])) == (368, 45, 145),
            'V6 inventory drift')
    return gate


def checked_auxiliary(root):
    old = previous_gate(root)['auxiliary']
    require(set(AUXILIARY_SUCCESSORS) <= old.keys() and not old.keys() & EXTRA_AUXILIARY.keys(),
            'auxiliary delta classification drift')
    expected = {**old, **AUXILIARY_SUCCESSORS, **EXTRA_AUXILIARY}
    actual = {name: sha(read_inside(root, name)) for name in expected}
    require(actual == expected and len(actual) == 51, 'reviewed auxiliary input drift')
    return actual


def load_context(root):
    # Called only after checked_auxiliary has checked this and all legacy Python.
    path = root / 'tools/historical-blob-batch-074/module_context_v5.py'
    spec = importlib.util.spec_from_file_location('hc074_checked_context_v7', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def target_catalog(root):
    manifest = tomllib.loads(read_inside(root, 'crates/xtask/Cargo.toml').decode())
    require(manifest['package']['name'] == 'xtask' and manifest['package'].get('autotests', True)
            and 'test' not in manifest, 'default package discovery drift')
    return ['unittests src/lib.rs', 'unittests src/main.rs'] + sorted(
        'tests/' + path.name for path in (root / 'crates/xtask/tests').glob('*.rs'))


def source_review(root, document):
    root = root.resolve(strict=True)
    require(document_digest(document) == REVIEWED_AST_SHA256, 'unreviewed v7 syntax/source inventory')
    require(document['executed_input_code'] is False and document['compiler_expansion'] is False,
            'source-only inventory scope')
    rows = {row['source']: row for row in document['sources']}
    require(len(rows) == len(document['sources']) == 372, 'duplicate or changed source count')
    constructors = []
    for name, row in rows.items():
        require(Path(name).suffix == '.rs' and sha(read_inside(root, name)) == row['sha256'],
                'source bytes/path drift')
        for site in row['sites']:
            require(site['kind'] != 'include', 'unexpanded include edge')
            if site['kind'] in ('command-constructor', 'macro-command-constructor'):
                require(site['detail'] not in {'literal:ssh', 'literal:systemctl', 'literal:systemd-run',
                                              'literal:wsl', 'literal:timedatectl'}, 'host/service constructor')
                constructors.append(dict(source=name, **site))
    raw = read_inside(root, PREVIOUS_SYNTAX)
    require(sha(raw) == Path(PREVIOUS_SYNTAX).name, 'V6 syntax bytes drift')
    old_document = decode(raw)
    require(document_digest(old_document) == PREVIOUS_AST_SHA256, 'V6 AST drift')
    old = {row['source']: row for row in old_document['sources']}
    prior = previous_gate(root)
    require({name: row['sha256'] for name, row in old.items()} == prior['sources']
            and not old.keys() - rows.keys(), 'old source removal/mismatch')
    changed = {name for name in old if old[name] != rows[name]}
    added = rows.keys() - old.keys()
    require({name: rows[name]['sha256'] for name in changed} == CHANGED and len(changed) == 4,
            'manual changed source/AST delta drift')
    require({name: rows[name]['sha256'] for name in added} == ADDED and len(added) == 4,
            'manual added source/AST delta drift')
    auxiliary = checked_auxiliary(root)
    additions, unresolved = load_context(root).module_edges(root, document)
    require(not additions and not unresolved, 'module graph not closed')
    discovered = {p.relative_to(root).as_posix() for folder in ('crates/xtask/src', 'crates/xtask/tests')
                  for p in (root / folder).rglob('*.rs')}
    require(discovered <= rows.keys(), 'new uncensused Rust source')
    targets = target_catalog(root)
    expected = prior['targets'][:2] + sorted(prior['targets'][2:] +
                ['tests/git_descendant_cpu_074.rs', 'tests/security_input_view_074.rs'])
    require(targets == expected and len(set(targets)) == len(targets) == 147, 'default target catalog drift')
    return dict(schema='hc074-package-effect-review-v7', reviewed_ast_sha256=REVIEWED_AST_SHA256,
                previous_ast_sha256=PREVIOUS_AST_SHA256, sources={n: r['sha256'] for n, r in rows.items()},
                auxiliary=auxiliary, targets=targets, constructors=constructors,
                changed_sources=sorted(changed), added_sources=sorted(added), delta_review=DELTA_REVIEW,
                inherited_review={k: v for k, v in prior.items() if 'review' in k},
                source_effect_review_complete=True, formal_whole_program_proof=False,
                package_dispatched=False, package_pass=False, linux_or_all_features_cleared=False,
                original_execution_or_product_or_qualification_admission=False)


def environment_preflight(root):
    checked_auxiliary(root)
    # All imported project bodies retain their independently sealed V6 pins.
    import package_effects_review_v2
    return package_effects_review_v2.environment_preflight(root)
