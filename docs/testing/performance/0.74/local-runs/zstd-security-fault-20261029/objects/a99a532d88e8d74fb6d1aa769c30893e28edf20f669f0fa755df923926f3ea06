import copy
import json
from pathlib import Path
import unittest

import transport_attribution as profile


def fixture():
    rows = []
    for block in range(6):
        variants = ('legacy', 'current', 'current', 'legacy') if block % 2 == 0 else ('current', 'legacy', 'legacy', 'current')
        for slot, variant in enumerate(variants):
            spans, child = [], []
            if variant == 'current':
                names = ['source-tempfile', 'input-write-seek', 'output-tempfiles', 'setup-and-spawn',
                         'metadata', 'try-wait', 'sleep', 'metadata', 'try-wait', 'capture-read', 'stop']
                spans = [dict(phase=name, wall_ns=1) for name in names]
                before = dict(pid=1, created_100ns=1, exited_100ns=0, kernel_ns=0, user_ns=0)
                after = dict(before, exited_100ns=2, kernel_ns=100, user_ns=200)
                child = [before, after]
            rows.append(dict(block=block, slot=slot, variant=variant, stdout_bytes=34850,
                             stderr_bytes=0, wall_ns=100, parent_cpu_ns=0, spans=spans, child=child))
    return rows


def encode(rows):
    done = dict(blocks=6, samples=24, legacy_is_unobserved_installed_transport=True,
                current_is_profile_mirror=True, product_candidate=False, checks_or_deadlines_changed=False)
    return ('\n'.join('HC074_TRANSPORT ' + json.dumps(v) for v in rows) +
            '\nHC074_TRANSPORT_DONE ' + json.dumps(done) + '\n').encode()


class TransportAttributionControls(unittest.TestCase):
    def test_mirror_erases_to_installed_function_and_helpers(self):
        folder = Path(__file__).resolve().parent
        original = (folder / 'transport.rs').read_text()
        candidate = (folder / 'transport_profile.rs').read_text()
        profile.mirror_check(original, candidate)
        for changed in (candidate.replace('Duration::from_millis(5)', 'Duration::from_millis(6)'),
                        candidate.replace('timely(deadline)?;', ''),
                        candidate.replace('256 * 1024;', '257 * 1024;'),
                        candidate.replace('owned.stop()', 'owned.child.kill()')):
            with self.assertRaises(ValueError):
                profile.mirror_check(original, changed)

    def test_complete_profile_not_an_optimization(self):
        result = profile.review(encode(fixture()))
        self.assertEqual(result['whole_child_cpu_ns']['median'], 300)
        self.assertFalse(result['product_performance_claim'])
        self.assertEqual(result['summary']['unobserved']['wall_ns']['samples'], 12)

    def test_identity_phase_order_and_overlap_refuse(self):
        for mutation in range(4):
            rows = copy.deepcopy(fixture())
            if mutation == 0:
                rows[1]['child'][1]['created_100ns'] = 2
            elif mutation == 1:
                rows[1]['spans'][4], rows[1]['spans'][5] = rows[1]['spans'][5], rows[1]['spans'][4]
            elif mutation == 2:
                rows[1]['wall_ns'] = 1
            else:
                rows[0]['spans'] = [dict(phase='stop', wall_ns=0)]
            with self.assertRaises(ValueError):
                profile.review(encode(rows))

    def test_missing_duplicate_and_reordered_samples_refuse(self):
        rows = fixture()
        for changed in (rows[:-1], rows + rows[:1], [rows[1], rows[0], *rows[2:]]):
            with self.assertRaises(ValueError):
                profile.review(encode(changed))


if __name__ == '__main__':
    unittest.main()
