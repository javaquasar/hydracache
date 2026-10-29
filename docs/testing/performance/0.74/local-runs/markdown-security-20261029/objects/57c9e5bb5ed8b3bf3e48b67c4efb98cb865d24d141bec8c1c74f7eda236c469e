"""Offline arithmetic and refusal controls, with no native or WSL execution."""
import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import analyze as a


def costs(wall=100, cpu=80, **extra):
    return dict(wall_ns=wall, cpu_ns=cpu, user_ns=0, system_ns=0, voluntary=0,
                involuntary=0, inblock=0, oublock=0, **extra)


class Controls(unittest.TestCase):
    def test_only_exact_git_checkout_line_endings_accepted(self):
        self.assertTrue(a.same_checkout_source(b'a\nb\n', b'a\nb\n'))
        self.assertTrue(a.same_checkout_source(b'a\r\nb\r\n', b'a\nb\n'))
        self.assertTrue(a.same_checkout_source(b'a\nb\n', b'a\r\nb\r\n'))
        for changed in (b'a\r\nb\n', b'a\n b\n', b'a\nb', b'a\nb\nextra'):
            self.assertFalse(a.same_checkout_source(changed, b'a\nb\n'))

    def test_signed_residual_is_not_clamped_or_overhead_subtracted(self):
        value = a.bracket([costs(wall=100 + i, cpu=102 + 2 * i) for i in range(7)])
        self.assertEqual(value['elapsed_minus_cpu_ns']['values'], [-2, -3, -4, -5, -6, -7, -8])
        self.assertEqual(value['costs']['cpu_ns']['values'][0], 102)
        variance = value['elapsed_variance_ns2']
        self.assertAlmostEqual(variance['elapsed'], variance['cpu'] + variance['residual'] + variance['twice_covariance'])

    def test_variance_zero_and_ties(self):
        self.assertIsNone(a.correlation([1] * 7, list(range(7)))['pearson'])
        self.assertIsNone(a.correlation([1] * 7, list(range(7)))['spearman'])
        self.assertEqual(a.ranks([9, 1, 1, 7]), [4, 1.5, 1.5, 3])
        self.assertAlmostEqual(a.correlation([1, 1, 3], [9, 9, 1])['spearman'], -1)
        self.assertEqual(a.series([0] * 7)['mad'], 0)
        self.assertIsNone(a.series([0] * 7)['max_over_min'])

    def test_all_pairs_strata_and_extremes_preserved(self):
        values = [0.5, 2, 1, 3, 0.1, 4, 0.8]
        result = a.stratify(values)
        self.assertEqual(result['all']['values'], values)
        self.assertEqual(result['baseline_first']['indices'], [0, 2, 4, 6])
        self.assertEqual(result['installed_first']['values'], [2, 3, 4])
        self.assertEqual(sorted(result['early']['indices'] + result['middle']['indices'] + result['late']['indices']), list(range(7)))

    def test_membership_nonfinite_bool_and_invalid_costs_refuse(self):
        for bad in ([1] * 6, [1] * 8, [float('nan')] * 7, [float('inf')] * 7, [True] * 7, [0] * 7):
            with self.assertRaises(ValueError): a.stratify(bad)
        for bad in ([costs()] * 6, [costs(wall=0)] * 7, [costs(cpu=-1)] * 7):
            with self.assertRaises(ValueError): a.bracket(bad)
        with self.assertRaises(ValueError): a.correlation([1, 2], [1])

    def test_phase_covariance_uses_full_batch_not_cycle(self):
        rows = [dict(mode='phases', depth=127, fault='external', **{
            phase: costs(cpu=(i + 1) * (32 if phase == 'warm' else 1)) for phase in a.PHASES}) for i in range(7)]
        result = a.phase_sum(rows)
        self.assertEqual(result['measured_brackets_cpu_sum_ns']['values'][0], 37)
        self.assertAlmostEqual(result['cpu_share']['warm']['median'], 32 / 37)
        self.assertAlmostEqual(sum(result['variance_share_via_covariance'].values()), 1)
        self.assertFalse(result['cycle_decomposition'])
        rows[0]['mode'] = 'cycle'
        with self.assertRaises(ValueError): a.phase_sum(rows)

    def test_original_corpus_and_order_guards_retained(self):
        raw = (a.PACKET / 'measured/194-pair-0-baseline.stdout.log').read_text()
        _, rows = a.original.parse(raw)
        pairs = [dict(baseline=copy.deepcopy(rows), installed=copy.deepcopy(rows)) for _ in range(7)]
        pairs[0]['installed'][0]['corpus'] = 'b' * 64
        with self.assertRaises(ValueError): a.original.summarize(pairs)
        marker = 'HC074-RUNTIME-CPU '
        lines = raw.splitlines(); selected = [i for i, line in enumerate(lines) if line.startswith(marker)]
        lines[selected[0]], lines[selected[1]] = lines[selected[1]], lines[selected[0]]
        with self.assertRaises(ValueError): a.original.parse('\n'.join(lines))

    def test_pinned_inventory_tamper_refuses_before_reader(self):
        with patch.object(a, 'sha', return_value='changed'), patch.object(a.predecessor, 'verify') as reader:
            with self.assertRaises(ValueError): a.guard()
            reader.assert_not_called()

    def test_original_inference_and_membership_remain_exact(self):
        result = a.derive(); old = a.guard()
        self.assertEqual(result['processes'], 28); self.assertEqual(result['rows'], 336)
        for name in ('primary_warm_cpu', 'primary_cycle_elapsed'):
            self.assertEqual(result['original_primary'][name], old[name])
            self.assertEqual(old[name]['disposition'], 'no-clear-practical-effect-or-mixed')
        self.assertFalse(result['measurement_executed']); self.assertTrue(result['post_hoc'])

    def test_publish_verify_refuses_overwrite_tamper_and_new_member(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'derived'
            a.publish(root, None); a.verify(root)
            with self.assertRaises(ValueError): a.publish(root, None)
            facts = root / 'facts.json'; original = facts.read_bytes()
            facts.write_bytes(original + b' ')
            with self.assertRaises(ValueError): a.verify(root)
            facts.write_bytes(original)
            (root / 'extra').write_bytes(b'unregistered')
            with self.assertRaises(ValueError): a.verify(root)

    def test_constant_phase_sum_has_no_variance_shares(self):
        row = dict(mode='phases', depth=127, fault='external', **{phase: costs() for phase in a.PHASES})
        self.assertEqual(a.phase_sum([row] * 7)['variance_share_via_covariance'], dict.fromkeys(a.PHASES))


if __name__ == '__main__': unittest.main(verbosity=2)
