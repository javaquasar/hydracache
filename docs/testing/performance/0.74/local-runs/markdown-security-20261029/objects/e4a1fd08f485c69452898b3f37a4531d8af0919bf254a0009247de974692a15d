"""Scope and non-promotion controls only; no live checks or dispatch."""
import copy
from pathlib import Path
import tomllib
import unittest

import scope


class Closure(unittest.TestCase):
    def data(self):
        base = scope.ROOT/'docs/testing/performance/0.74'
        return (tomllib.loads((scope.ROOT/scope.SCOPE).read_text()),
                tomllib.loads((base/'composition-ledger.toml').read_text()),
                (base/'qualification-manifest.toml').read_bytes())

    def test_scope_is_not_freeze_or_admission(self):
        result = scope.check(scope.ROOT)
        self.assertTrue(result['scope_recorded'])
        self.assertFalse(result['freeze_c74_allowed'])
        self.assertFalse(result['server_admitted'])

    def test_claim_candidate_and_qualification_promotions_refuse(self):
        for key, value in (('performance_improvement_claim', True), ('accepted_performance_candidates', True),
                           ('accepted_performance_candidates', 1), ('freeze_c74_allowed', True),
                           ('qualification_allowed', True), ('skip_release_checks', True),
                           ('reopen_rejected_candidates', True), ('candidate_source_sha', 'f'*40)):
            d, c, m = self.data()
            d[key] = value
            with self.assertRaises(ValueError):
                scope.validate(d, c, m)

    def test_original_checks_and_deadlines_are_not_relaxed(self):
        for key in ('package_seconds', 'package_case_threads', 'original_preparation_seconds',
                    'original_admission_seconds', 'original_effect_seconds'):
            for value in (True, 999, 60.0):
                d, c, m = self.data()
                d['invariants'][key] = value
                with self.assertRaises(ValueError):
                    scope.validate(d, c, m)

    def test_composition_and_frozen_manifest_remain_independent(self):
        d, c, m = self.data()
        bad = copy.deepcopy(c)
        bad['accepted_candidate_count'] = 1
        with self.assertRaises(ValueError):
            scope.validate(d, bad, m)
        with self.assertRaises(ValueError):
            scope.validate(d, c, m+b' ')

    def test_host_observation_cannot_become_delivery_or_installation_proof(self):
        for key in ('compatible_original_delivery_proved', 'installation_performed', 'services_changed',
                    'clocks_changed', 'product_workload_started', 'active_markers', 'fixture_cgroups'):
            d, c, m = self.data()
            d['server_preflight'][key] = True
            with self.assertRaises(ValueError):
                scope.validate(d, c, m)


if __name__ == '__main__':
    unittest.main()
