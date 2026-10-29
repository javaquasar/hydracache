"""Synthetic metadata controls only; no WSL, probes, services or clock reads."""
import unittest
import json
from pathlib import Path
import tempfile
from unittest import mock

import identity as i


def field(name, value, rc=0):
    return f'FIELD {name}\n{value}\n\nRESULT {rc}\n\n'


def sample(pid, chain, observer, comm='systemd-timesyn'):
    stat = f'{pid} ({comm}) S ' + '0 ' * 18 + '11340 0'
    return (field('boot', i.BOOT) + field('observer-pidns', observer)
            + field('observer-timens', 'time:[1]') + f'PROCESS {pid} {comm}\n'
            + ''.join(field(k, v) for k, v in {
                'stat-before': stat, 'stat-after': stat,
                'status': f'Name:\t{comm}\nPid:\t{pid}\nNSpid:\t{chain}\nUid:\t996 996 996 996',
                'exe': '/usr/lib/systemd/systemd-timesyncd',
                'executable-digest': 'a' * 64 + f'  /proc/{pid}/exe',
                'cgroup': '0::/system.slice/systemd-timesyncd.service',
                'ns/pid': 'pid:[7]', 'ns/time': 'time:[1]', 'ns/mnt': 'mnt:[8]'}.items())
            + field('boot-after', i.BOOT))


class IdentityTests(unittest.TestCase):
    def setUp(self):
        self.system = sample(193, '193 176', 'pid:[6]')
        self.ubuntu = sample(176, '176', 'pid:[7]')
        self.docker = sample(1, '1', 'pid:[9]', 'init(docker-des')

    def test_current_join_is_not_a_historical_trace_join(self):
        result = i.summarize(self.system, self.ubuntu, self.docker)
        self.assertTrue(result['current_timesyn_views_join'])
        self.assertFalse(result['trace_pid_to_executable_authenticated'])
        self.assertTrue(all(result[k] is False for k in i.FALSE_FLAGS))

    def test_comm_parentheses(self):
        self.assertEqual(i.stat('1 (init()) S ' + '0 ' * 18 + '10903'), (1, 'init()', 10903))

    def test_crlf_decodes_without_changing_meaning(self):
        self.assertEqual(i.context(self.system), i.context(self.system.replace('\n', '\r\n')))

    def test_identity_drift_refuses(self):
        for old, new in [('11340', '11341'), ('time:[1]', 'time:[2]'),
                         ('mnt:[8]', 'mnt:[9]'), ('a' * 64, 'b' * 64),
                         ('193 176', '193 177'), (i.BOOT, 'different-boot')]:
            with self.subTest(old=old), self.assertRaises(ValueError):
                i.summarize(self.system.replace(old, new), self.ubuntu, self.docker)

    def test_stat_change_refuses(self):
        with self.assertRaises(ValueError):
            i.context(self.system.replace('11340', '11341', 1))

    def test_failed_duplicate_and_mislabeled_metadata_refuse(self):
        for raw in [self.system.replace('RESULT 0', 'RESULT 1', 1),
                    self.system.replace('FIELD stat-before', field('exe', 'another') + 'FIELD stat-before'),
                    self.system.replace('Name:\tsystemd-timesyn', 'Name:\tchronyd')]:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                i.context(raw)

    def test_exclusive_retention_and_exact_verification(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            out, archive = root / 'out', root / 'archive'
            out.mkdir()
            for name, text in [('system', self.system), ('ubuntu', self.ubuntu), ('docker', self.docker)]:
                (out / (name + '-root-processes.stdout.log')).write_text(text, encoding='utf-8')
            with mock.patch.object(i, 'OUT', out), mock.patch.object(i, 'ARCHIVE', archive), \
                    mock.patch.object(i.base, 'sources'), mock.patch('builtins.print'):
                i.retain()
                i.verify()
                with self.assertRaises(FileExistsError):
                    i.retain()
                index = json.loads((archive / 'index.json').read_bytes())
                entry = index['files']['observation/summary.json']
                obj = archive / 'objects' / entry['sha256']
                obj.write_bytes(b'corrupted')
                with self.assertRaisesRegex(ValueError, 'Changed identity evidence'):
                    i.verify()

    def test_scope_and_unsafe_digest_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory)
            index = {'schema': 'hc074-readonly-writer-identity-v1',
                     'source_capture_sha256': i.CAPTURE_SHA, 'original_windows': [300, 60, 10],
                     **dict.fromkeys(i.FALSE_FLAGS, False),
                     'files': {'unsafe': {'sha256': '../escape', 'bytes': 0}}}
            with mock.patch.object(i, 'ARCHIVE', archive):
                for change in ({'admission': True}, {'original_windows': [301, 60, 10]}, {}):
                    (archive / 'index.json').write_text(json.dumps(index | change), encoding='utf-8')
                    with self.assertRaises(ValueError):
                        i.verify()


if __name__ == '__main__':
    unittest.main()
