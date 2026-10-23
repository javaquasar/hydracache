"""Offline controls, without reading or setting live clocks."""
import unittest
import causality as c


def data():
    rows = [{'modes': 0, 'result': 0, 'euid': 1000, 'user_hz': 100, 'status': 8192,
             'tick_us': 9167, 'frequency_scaled_ppm': 0,
             'raw_before_ns': i * 1_000_000_000, 'raw_after_ns': i * 1_000_000_000 + 100,
             'realtime_ns': 1_000_000_000_000 + i * 916_700_000,
             'monotonic_ns': i * 916_700_000, 'boottime_ns': i * 916_700_000} for i in range(81)]
    messages = [{'row': i, 'packet': 5, 'spike': False, 'offset_us': 0} for i in range(81)]
    return rows, messages


class Causality(unittest.TestCase):
    def test_tick_model_and_raw_enclosure_not_admission(self):
        r = c.analyze(*data())
        self.assertEqual(r['tick_plus_frequency_rate_range'], [0.9167, 0.9167])
        lo, hi = r['observed_monotonic_per_raw_enclosure']
        self.assertLess(lo, 0.9167)
        self.assertGreater(hi, 0.9167)
        self.assertFalse(r['admission'])
        self.assertIsNone(r['historical_tick_setter'])

    def test_ntp_decoding_and_signed_offset(self):
        msg = c.ntp('7 (uuuuittayttttbtt) 0 4 4 2 -24 1 2 4 1 2 3 4 1000 1500 1500 1100 false 9 0')
        self.assertEqual(msg['offset_us'], 450)
        self.assertEqual(msg['delay_us'], 100)
        self.assertEqual(msg['packet'], 9)
        with self.assertRaises(ValueError):
            c.ntp('7 unknown')

    def test_updates_and_first_backward_preserved_without_caller_claim(self):
        rows, messages = data()
        rows[10]['realtime_ns'] -= 10_000_000_000
        for m in messages[11:]:
            m['packet'] = 6
        r = c.analyze(rows, messages)
        self.assertEqual(r['backwards_realtime'][0]['row'], 10)
        self.assertEqual(r['ntp_updates'][0]['row'], 11)
        self.assertIsNone(r['exact_clock_setting_caller'])

    def test_incomplete_changed_modes_and_bad_brackets_refuse(self):
        cases = []
        rows, msg = data(); cases.append((rows[:-1], msg))
        rows, msg = data(); rows[0]['modes'] = 0x4000; cases.append((rows, msg))
        rows, msg = data(); rows[0]['euid'] = 0; cases.append((rows, msg))
        rows, msg = data(); msg[2]['row'] = 8; cases.append((rows, msg))
        rows, msg = data(); rows[1]['monotonic_ns'] = -1; cases.append((rows, msg))
        rows, msg = data(); rows[0]['raw_after_ns'] = -1; cases.append((rows, msg))
        rows, msg = data(); msg[2]['packet'] = 0; cases.append((rows, msg))
        for rows, msg in cases:
            with self.assertRaises(ValueError):
                c.analyze(rows, msg)

    def test_query_abi_has_no_setting_inputs(self):
        source = (c.base.HERE / 'discipline.c').read_text()
        self.assertIn('struct timex state = {0};', source)
        self.assertEqual(source.count('adjtimex(&state)'), 1)
        self.assertNotIn('state.modes =', source)
        for forbidden in ('clock_settime(', 'settimeofday(', 'ADJ_TICK', 'clock_adjtime('):
            self.assertNotIn(forbidden, source)
        shell = (c.base.HERE / 'causality-session.sh').read_text()
        self.assertEqual(shell.count('systemctl '), 1)
        self.assertIn('systemctl show ', shell)
        for line in shell.splitlines():
            if line.strip().startswith('busctl '):
                self.assertIn('--auto-start=no', line)
                self.assertNotIn(' call ', line)
        self.assertIn('test "$i" -lt 81', shell)


if __name__ == '__main__':
    unittest.main()
