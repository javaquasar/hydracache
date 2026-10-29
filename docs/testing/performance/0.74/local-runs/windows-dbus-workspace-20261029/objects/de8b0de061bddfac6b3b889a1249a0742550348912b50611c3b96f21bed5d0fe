"""Read-only closure-scope consistency, never release admission or dispatch."""
import hashlib
from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parents[2]
SCOPE = 'docs/testing/performance/0.74/release-closure-scope-20261026.toml'


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def validate(scope, composition, manifest_bytes):
    require(scope['schema_version'] == 1 and scope['release'] == '0.74' and
            scope['decision'] == 'defer-rejected-performance-candidates-and-close-release-gates', 'scope identity differs')
    require(type(scope['accepted_performance_candidates']) is int and scope['accepted_performance_candidates'] == 0 and
            composition['accepted_candidate_count'] == 0 and composition['freeze_c74_allowed'] is False and
            scope['candidate_source_sha'] == composition['candidate_source_sha'] == 'UNRESOLVED', 'candidate promoted')
    require(all(scope[k] is False for k in ('performance_improvement_claim', 'reopen_rejected_candidates',
            'skip_release_checks', 'freeze_c74_allowed', 'qualification_allowed')), 'scope changes admission')
    require(scope['baseline_product_sha'] == composition['baseline_product_sha'] ==
            '16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b', 'baseline differs')
    require(scope['qualification_manifest_sha256'] == hashlib.sha256(manifest_bytes).hexdigest() ==
            '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc', 'qualification differs')
    inv = scope['invariants']
    require(all(type(inv[k]) is int and inv[k] == v for k, v in
            (('package_seconds', 780), ('package_case_threads', 4), ('original_preparation_seconds', 300),
             ('original_admission_seconds', 60), ('original_effect_seconds', 10))) and
            all(inv[k] is False for k in ('live_check_frequency_changed', 'thresholds_changed', 'frozen_073_changed')),
            'frozen checks or windows differ')
    host = scope['server_preflight']
    require(host['read_only'] is True and host['supervisor_active'] is True and host['supervisor_pid'] == 377415 and
            host['supervisor_restarts'] == 0 and host['manager_version'] == '255.4-1ubuntu8.17' and
            all(host[k] is False for k in ('active_markers', 'fixture_cgroups', 'compatible_original_delivery_proved',
                'installation_performed', 'services_changed', 'clocks_changed', 'product_workload_started')),
            'read-only observation promoted')
    return dict(scope_recorded=True, accepted_performance_candidates=0, freeze_c74_allowed=False,
                package_pass=False, server_admitted=False, qualification_allowed=False)


def check(root):
    root = Path(root)
    base = root/'docs/testing/performance/0.74'
    return validate(tomllib.loads((root/SCOPE).read_text()),
                    tomllib.loads((base/'composition-ledger.toml').read_text()),
                    (base/'qualification-manifest.toml').read_bytes())


if __name__ == '__main__':
    print(check(ROOT))
