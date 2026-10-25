"""Source-bound manual review receipt for DEFAULT WINDOWS xtask package effects.

This is not a compiler-expansion theorem or production/security admission.
The recorded entry-point review is invalid on any source/auxiliary input change.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

import census_cost
import package_replay_audit
import syntax_audit

REVIEWED_AST_SHA256 = '9b516072c8d034cf33c11b17ec8e07fdd4a4c337903f77ab9a82ba727f9cbc59'
REVIEWED_AUXILIARY = {
    ".cargo/config.toml": "90f023744f969c176d98d811145b883b411da0fe8a294e930a0f965456ac49a3",
    "Cargo.toml": "18a8a27cf140a1d6eb31c8316ee39bb7d5832866a990346eee16cd0c30de344b",
    "Cargo.lock": "d2c7736ebaa8381cece9162bbbf384db60c46111051e4e78e51d4bae12feaf5c",
    "crates/xtask/Cargo.toml": "0b00525d41119bdb1215b1fb658ac66fa1bcd0d99428e9903cf1bb8ee8f4e0b5",
    "scripts/ci/run-with-heartbeat.py": "f42cda4480a8c29c649954f838ddda3425a163e062a1016cbad612a393a7f53c",
    "scripts/ci/public_api_compat_071.py": "e6c093f732df392cfb7eda4aa6f274d8b7fc425146e08e66e2b67f82255efcb9",
    "scripts/perf/memory-providers/test_provider_contract.py": "0badc7d752fc38a78604443551388a5770d8ff715ab69ce54efa54cee6bb5ee1",
    "scripts/perf/memory-providers/provider_common.py": "d04439d35c0a3c5cbd112dc0a06cb02c381e24a4b3d5eea501339383d73ad4df",
    "scripts/perf/memory-providers/system.py": "582ccf04b5e9883a24b1636302b53972bd2d89ff49cdc86cb1ada85aa42f67a7",
    "scripts/perf/render-exploratory-report.py": "ee39a9533a712f845fe14099ecf476fe2231270b6bbb611bab43aa02cb899354",
    "scripts/perf/sync-ci-instruction-lock.py": "8286bf431ef987f218f73f2863116ee63feb88d202c95e3017da1e90b1bbd026",
    "tools/historical-blob-batch-074/oracle.py": "1e360e21f79f1ecec1b7305fe1b914cd471d63f42686d1badc7548c9e8d1e90d",
    "tools/historical-blob-batch-074/reader.py": "71207da9f35a2b3458092e761c9a86ce9c47d07b8e99db3743bd430c2bd57470",
    "tools/historical-blob-batch-074/caller_pins.py": "f5c8ccaf9dcff9b32309ece12806af510fd022ee6bf2530ff439fa9db5831857",
    "tools/historical-blob-batch-074/caller_integration.py": "3ef9fdbec1c5969d460edadd41238c8d9eb2f3f7a72eb36f1f327a7d9df911ec",
    "tools/historical-blob-batch-074/collision.py": "29d3b9bb0dcd23d47cdf79052336e2da8daf4f4244903ff8df4b7e713f03b706",
    "tools/long-run-supervisor-074/src/archive.rs": "733cb99bff1768b5d97111857a507293f0f345ff808d20f225e2f2f324c00957",
    "tools/long-run-supervisor-074/src/artifact.rs": "b13806efd66a695fdc5fb5b6bfb2ae38cb6b32ca70df6272709655d8ec65e7da",
    "tools/long-run-supervisor-074/src/sealed_permissions.rs": "72a89f8651bba1df518abf2d84a18835ef4aeba268a55c45b75e12a0b25f77ec",
    "tools/long-run-supervisor-074/src/diagnostic_builder.rs": "7cc438ed0d76a4896abea77c4070c8a017730e7ef8d82b3e40259629e1ec259a",
    "tools/long-run-supervisor-074/src/diagnostic_authority_registry.rs": "1e936fe855eedad6d0e5dacbd76d534a45cc8df11cd30f06f8e2ed77258bc53e"
}


def document_digest(document):
    return hashlib.sha256(json.dumps(document, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


REVIEW = {
    'library': 'Unit roots inspect fixtures/validators, not CLI run dispatch. Python generation invokes only vendored protoc into target/hc2-python-generation. Registry/evidence/canary executors use self-only synthetic commands; probe/measurement/venv/Maven/curl/verify CLI routes are not invoked.',
    'test_processes': 'Git historical reads; owned synthetic Git refs/config repositories; self-only child modes; Python oracle/collision/caller pins; heartbeat child controls; report/normalization/lock-sync scripts; two exact xtask admission/report modes on scratch receipts.',
    'macro_review': 'Standard assertion/format operands parsed; remaining macro token constructors are Git reads or negative missing/must-not-spawn controls. Quoted source is data. No include! implementation edge remains.',
    'dependency_api_slices': 'Supervisor packet/archive build/verify/sealed permissions operate only on supplied temp fixtures; builder load_policy/decode_key and authority constants are pure byte verification. Linux-only local_context/local_files/intent_journal refs are cfg-disabled on Windows. Loadgen profile/budget/overload and fault-proxy calls are data/model validation, not live runners.',
    'cleanup': 'Evidence Windows job/heartbeat/canary cleanup owns synthetic child handles/trees. Protoc scratch replacement is confined to repository target. Modelled service/cgroup/clock commands are source strings, not operations.',
}
AUXILIARY = (
    '.cargo/config.toml', 'Cargo.toml', 'Cargo.lock', 'crates/xtask/Cargo.toml',
    'scripts/ci/run-with-heartbeat.py', 'scripts/ci/public_api_compat_071.py',
    'scripts/perf/memory-providers/test_provider_contract.py',
    'scripts/perf/memory-providers/provider_common.py', 'scripts/perf/memory-providers/system.py',
    'scripts/perf/render-exploratory-report.py', 'scripts/perf/sync-ci-instruction-lock.py',
    'tools/historical-blob-batch-074/oracle.py', 'tools/historical-blob-batch-074/reader.py',
    'tools/historical-blob-batch-074/caller_pins.py', 'tools/historical-blob-batch-074/caller_integration.py',
    'tools/historical-blob-batch-074/collision.py',
    'tools/long-run-supervisor-074/src/archive.rs', 'tools/long-run-supervisor-074/src/artifact.rs',
    'tools/long-run-supervisor-074/src/sealed_permissions.rs', 'tools/long-run-supervisor-074/src/diagnostic_builder.rs',
    'tools/long-run-supervisor-074/src/diagnostic_authority_registry.rs',
)


def source_review(root, document):
    census_cost.require(document_digest(document) == REVIEWED_AST_SHA256, 'unreviewed syntax/source inventory')
    additions, unresolved = syntax_audit.module_edges(root, document)
    census_cost.require(not additions and not unresolved, 'module graph not closed')
    sources = {v['source']: v['sha256'] for v in document['sources']}
    census_cost.require(len(sources) == len(document['sources']), 'duplicate source')
    census_cost.require(not document['executed_input_code'] and not document['compiler_expansion'], 'inventory scope')
    constructors = []
    for row in document['sources']:
        path = (root / row['source']).resolve(strict=True)
        census_cost.require(path.is_relative_to(root) and path.suffix == '.rs', 'source outside Rust inventory')
        census_cost.require(hashlib.sha256(path.read_bytes()).hexdigest() == row['sha256'], 'source drift')
        for site in row['sites']:
            census_cost.require(site['kind'] != 'include', 'unexpanded include edge')
            if site['kind'] in ('command-constructor', 'macro-command-constructor'):
                census_cost.require(site['detail'] not in {'literal:ssh', 'literal:systemctl', 'literal:systemd-run',
                    'literal:wsl', 'literal:timedatectl'}, 'host/service constructor requires separate review')
                constructors.append(dict(source=row['source'], **site))
    auxiliary = {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in AUXILIARY}
    census_cost.require(auxiliary == REVIEWED_AUXILIARY, 'reviewed auxiliary input drift')
    catalog = package_replay_audit.audit(root)['targets']
    return dict(method='manual source/entry-point review plus bounded AST/module inventory',
        scope='Windows default features; --lib --tests; no --ignored; fixed command/four threads/780 seconds',
        review=REVIEW, sources=sources, auxiliary=auxiliary, constructors=constructors, targets=catalog,
        source_effect_review_complete=True, formal_whole_program_proof=False,
        linux_or_all_features_cleared=False, product_or_qualification_admission=False)


def environment_preflight(root):
    blocked = census_cost.environment_overrides(os.environ)
    blocked += [name for name in os.environ if name.startswith(('HC074_', 'HYDRACACHE_', 'RUST_TEST_'))]
    census_cost.require(not blocked, 'inherited fixture/capture overrides: ' + ','.join(sorted(set(blocked))))
    census_cost.require(os.name == 'nt', 'only reviewed Windows package')
    target = root / 'target'
    census_cost.require(target.is_dir() and not target.is_symlink() and not target.is_junction(), 'target ancestry')
    scratch = target / 'hc2-python-generation'
    census_cost.require(not scratch.is_symlink() and not scratch.is_junction(), 'protoc scratch ancestry')
    # These inherited settings can execute hooks/helpers or lazily fetch objects.
    # Retain presence/hash only, never arbitrary Git configuration or credentials.
    pattern = r'^(core\.(hookspath|fsmonitor|sshcommand|gitproxy)|init\.templatedir|commit\.gpgsign|extensions\.partialclone|remote\..*\.promisor)$'
    value = subprocess.run(['git', '--no-replace-objects', 'config', '--includes', '--get-regexp', pattern],
        cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10, check=False)
    census_cost.require(value.returncode in (0, 1) and not value.stderr and len(value.stdout) <= 65536, 'Git effect config probe')
    active = []
    for line in value.stdout.decode().splitlines():
        name, _, body = line.partition(' ')
        if name.lower().endswith(('gpgsign', 'fsmonitor', '.promisor')) and body.lower() in ('false', 'no', '0'):
            continue
        active.append(name)
    census_cost.require(not active, 'inherited executable/lazy-fetch Git configuration: ' + ','.join(active))
    return dict(fixture_overrides_absent=True, target_and_protoc_scratch_not_reparse=True,
                git_effect_config_sha256=hashlib.sha256(value.stdout).hexdigest(), git_effect_config_active=False)


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[2]
    document = json.loads(Path(sys.argv[1]).read_bytes())
    result = source_review(root, document)
    result['environment'] = environment_preflight(root)
    print(json.dumps(result, indent=2, sort_keys=True))
