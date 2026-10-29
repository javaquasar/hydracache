"""Pure controls for joining asynchronous saved clock/NTP observations."""
import json
from pathlib import Path
import tempfile
import unittest

import offline_join as join
from test_causality import data


def publication(messages, first=5, spike=False):
    for message in messages[first:]:
        message.update(packet=6, spike=spike, timestamps_us=[1000000, 1500000, 1500000, 1100000])


class Windows(unittest.TestCase):
    def test_publication_does_not_pretend_getter_and_clock_are_simultaneous(self):
        rows, messages = data()
        publication(messages)
        for row in rows[6:]:
            row['realtime_ns'] += 450000000  # Step after row 5 clock, before row 6.
        result = join.windows(rows, messages)[0]
        self.assertEqual((result['first_clock_row'], result['last_clock_row']), (4, 6))
        self.assertEqual(result['remainder_if_ntp_step_applied_ns'], 0)
        self.assertIsNone(result['exact_caller'])

    def test_opposing_remainder_stays_conditional(self):
        rows, messages = data()
        publication(messages)
        for row in rows[5:]:
            row['realtime_ns'] -= 1_000_000
        result = join.windows(rows, messages)[0]
        self.assertEqual(result['remainder_if_ntp_step_applied_ns'], -451_000_000)
        self.assertIn('assumption', ' '.join(result))
        self.assertIsNone(result['exact_caller'])

    def test_end_of_capture_has_no_future_observation(self):
        rows, messages = data()
        publication(messages, first=80)
        result = join.windows(rows, messages)[0]
        self.assertFalse(result['complete'])
        self.assertEqual(result['reason'], 'no_clock_after_getter')
        self.assertNotIn('remainder_if_ntp_step_applied_ns', result)

    def test_missing_packets_and_spikes_are_not_invented_steps(self):
        rows, messages = data()
        publication(messages, spike=True)
        self.assertEqual(join.windows(rows, messages)[0]['reason'], 'spike_is_not_an_applied_step')
        for m in messages[5:]:
            m['packet'] = 8
        self.assertEqual(join.windows(rows, messages)[0]['reason'], 'missing_packet_history')

    def test_base_method_checks_are_not_bypassed(self):
        rows, messages = data()
        rows[0]['modes'] = 1
        with self.assertRaises(ValueError):
            join.windows(rows, messages)

    def test_small_ntp_offset_is_not_treated_as_an_instant_full_step(self):
        rows, messages = data()
        publication(messages)
        for m in messages[5:]:
            m['timestamps_us'] = [1000, 1500, 1500, 1100]
        self.assertEqual(join.windows(rows, messages)[0]['reason'], 'small_offset_uses_slew_not_full_step')


class Archive(unittest.TestCase):
    def test_exact_objects_and_nonadmitting_flags(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'objects').mkdir()
            raw = b'original first result\n'
            sha = join.digest(raw)
            obj = root / 'objects' / sha
            obj.write_bytes(raw)
            index = {'files': {'one': {'sha256': sha, 'bytes': len(raw)}},
                     'admission': False, 'product': False, 'server': False, 'original_owner_run': False}
            encoded = json.dumps(index).encode()
            (root / 'index.json').write_bytes(encoded)
            self.assertEqual(join.load(root, join.digest(encoded))['one'], raw)
            with self.assertRaises(ValueError):
                join.load(root, '0' * 64)
            obj.write_bytes(b'rewritten')
            with self.assertRaises(ValueError):
                join.load(root, join.digest(encoded))
            index['admission'] = True
            encoded = json.dumps(index).encode()
            (root / 'index.json').write_bytes(encoded)
            with self.assertRaises(ValueError):
                join.load(root, join.digest(encoded))


if __name__ == '__main__':
    unittest.main()
