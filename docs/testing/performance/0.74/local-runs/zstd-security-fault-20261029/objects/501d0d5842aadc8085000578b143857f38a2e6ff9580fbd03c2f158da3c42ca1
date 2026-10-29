import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import check as c
import retain as r


class Retention(unittest.TestCase):
    def test_exact_lane_changed_sources_commands_logs_and_scope(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = {'one.rs': 'fixed'}
            result = dict(lane='host', failure=None, steps=['one'], server=False, product=False, original_attempt=False)
            row = dict(argv=['fixed'], exit=0)
            for suffix in ('stdout', 'stderr'):
                file = root / ('one.' + suffix + '.log')
                file.write_bytes(b'first')
                row[suffix + '_sha256'] = c.sha(file)
            for name, value in [('result', result), ('one', row), ('source-before', source), ('source-after', source)]:
                c.publish(root / (name + '.json'), value)
            with patch.object(c, 'commands', return_value=[('one', ['fixed'])]):
                r.verify(root, 'host', source)
                for name, value, key, bad in [('result', result, 'failure', 'refused'), ('result', result, 'steps', []),
                    ('result', result, 'original_attempt', True), ('one', row, 'argv', ['changed']),
                    ('one', row, 'exit', 101), ('one', row, 'stderr_sha256', 'changed'),
                    ('source-after', source, 'one.rs', 'drift')]:
                    file = root / (name + '.json')
                    file.write_text(json.dumps(dict(value, **{key: bad})))
                    with self.assertRaises(ValueError): r.verify(root, 'host', source)
                    file.write_text(json.dumps(value))
                (root / 'one.stdout.log').write_bytes(b'changed')
                with self.assertRaises(ValueError): r.verify(root, 'host', source)

    def test_existing_packet_refuses_before_external_command(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r, 'PACKET', Path(tmp)), \
             patch.object(c.logger.subprocess, 'run') as child:
            with self.assertRaises(ValueError): r.run('unused')
            child.assert_not_called()


if __name__ == '__main__': unittest.main()
