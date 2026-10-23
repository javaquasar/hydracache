"""Pure observer analysis and safety controls; no WSL or clock reads."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import diagnose as d


def observations():
    header = {'schema': 'hc074-clock-readonly-v1', 'samples': 601, 'period_ms': 100,
              'clock_ids': [7, 0, 1, 4], 'order': 'clock-major libc-before,syscall,libc-after'}
    rows = [{'i': i, 'cpu': [i % 2, i % 2],
             'clocks': [[1_000_000_000 + c * 1000 + i * 100_000_000 + j for j in range(3)]
                        for c in range(4)]} for i in range(601)]
    return [header] + rows


def encode(rows):
    return '\n'.join(json.dumps(row) for row in rows)


class Analysis(unittest.TestCase):
    def test_ordered_observations_do_not_admit(self):
        result = d.analyze(encode(observations()))
        self.assertIsNone(result['first_anomaly'])
        self.assertFalse(result['admission'])
        self.assertIsNone(result['caused_by_specific_service'])
        self.assertEqual(result['direct_elapsed_ns']['boottime'], 60_000_000_000)
        self.assertEqual(result['cpus'], [0, 1])
        self.assertEqual(result['direct_realtime_minus_boottime_delta_residual_ns'], [0, 0])

    def test_first_backwards_not_hidden_by_later_recovery(self):
        rows = observations()
        rows[3]['clocks'][1] = [1, 2, 3]
        result = d.analyze(encode(rows))
        self.assertEqual(result['first_anomaly']['kind'], 'backwards')
        self.assertEqual(result['first_anomaly']['row'], 2)
        self.assertEqual(result['stats']['realtime']['backwards_by_api'], [1, 1, 1])

    def test_syscall_disagreement_separate_from_elapsed_delta(self):
        rows = observations()
        rows[1]['clocks'][0][1] += 10
        result = d.analyze(encode(rows))
        self.assertEqual(result['first_anomaly']['kind'], 'syscall_outside_libc_bracket')
        self.assertEqual(result['stats']['boottime']['bracket_violations'], 1)

    def test_incomplete_unknown_reordered_and_invalid_refuse(self):
        cases = []
        rows = observations(); cases.append(rows[:-1])
        rows = observations(); rows[0]['period_ms'] = 101; cases.append(rows)
        rows = observations(); rows[2]['i'] = 7; cases.append(rows)
        rows = observations(); rows[1]['clocks'][0] = [1]; cases.append(rows)
        rows = observations(); rows[1]['clocks'][0][0] = -1; cases.append(rows)
        rows = observations(); rows[1]['cpu'][0] = -1; cases.append(rows)
        for rows in cases:
            with self.subTest(rows=rows[0]), self.assertRaises(ValueError):
                d.analyze(encode(rows))

    def test_pid1_cpu_time_changes_not_new_init_but_start_ticks_are(self):
        # State is field 3; starttime is field 22, independent of cpu counters.
        fields = ['S'] + ['0'] * 18 + ['99'] + ['0'] * 30
        raw = 'boot\n1 (systemd) ' + ' '.join(fields) + '\n1/mnt=mnt:[7]\n'
        changed = fields.copy(); changed[11] = '100'
        self.assertEqual(d.identity(raw), d.identity(raw.replace(' '.join(fields), ' '.join(changed))))
        self.assertNotEqual(d.identity(raw), d.identity(raw.replace('99', '100')))


class Safety(unittest.TestCase):
    def test_only_readonly_clock_ffi_and_explicit_bound(self):
        source = (d.HERE / 'sample.rs').read_text()
        body = source.split('unsafe extern "C" {', 1)[1].split('\n}', 1)[0]
        self.assertEqual(body.count('    fn '), 3)
        for name in ('clock_gettime', 'syscall', 'sched_getcpu'):
            self.assertIn('fn ' + name + '(', body)
        self.assertEqual(source.count('syscall(SYS_CLOCK_GETTIME,'), 1)
        self.assertIn('const SYS_CLOCK_GETTIME: c_long = 228;', source)
        self.assertIn('const COUNT: usize = 601;', source)
        self.assertIn('const PERIOD_MS: u64 = 100;', source)

    def test_shell_has_no_unit_or_clock_mutations(self):
        shell = (d.HERE / 'session.sh').read_text()
        self.assertEqual([line.strip() for line in shell.splitlines() if line.strip().startswith('systemctl ')],
                         ['systemctl --version', "systemctl list-units --all --no-pager --no-legend | grep -Ei 'time|chron|ntp|supervisor' || test \"$?\" -eq 1"])
        for forbidden in ('timedatectl', 'date -s', 'hwclock', 'adjtimex', 'mount ', 'useradd', 'ssh ', 'native_fixture', 'original_service_live'):
            self.assertNotIn(forbidden, shell)
        self.assertIn('timeout --signal=TERM --kill-after=2s 80s', shell)
        self.assertLess(shell.index('identity > "$2/identity-before.txt"'), shell.index('80s "$1"'))
        self.assertGreater(shell.index('identity > "$2/identity-after.txt"'), shell.index('80s "$1"'))

    def test_evidence_publication_is_exclusive(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'first.json'
            d.put(path, {'exit': 101})
            with self.assertRaises(FileExistsError):
                d.put(path, {'exit': 0})
            self.assertEqual(json.loads(path.read_text())['exit'], 101)

    def test_frozen_and_workspace_guards(self):
        with patch.object(d, 'sha', return_value='changed'), self.assertRaises(ValueError):
            d.frozen()
        with self.assertRaises(ValueError):
            d.linux(Path('C:/unrelated/sample'))

    def test_refusal_first_vs_optional_metadata(self):
        with tempfile.TemporaryDirectory() as folder, patch.object(d, 'OUT', Path(folder)), \
             patch.object(d.subprocess, 'run') as run:
            run.return_value.returncode = 1
            with self.assertRaises(RuntimeError):
                d.command('required', ['unused'])
            d.command('optional', ['unused'], required=False)
            self.assertEqual(json.loads((Path(folder) / 'optional.json').read_text())['exit'], 1)
            self.assertEqual(run.call_count, 2)  # Once each, no retry.


if __name__ == '__main__':
    unittest.main()
