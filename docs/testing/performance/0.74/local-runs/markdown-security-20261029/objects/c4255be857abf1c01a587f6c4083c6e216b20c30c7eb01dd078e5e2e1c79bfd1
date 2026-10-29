import copy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import check as c
import retain as r


class Retention(unittest.TestCase):
    def test_incomplete_changed_log_source_scope_and_command_refuse(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            folder = root / 'host'
            folder.mkdir()
            steps = [('one', ['fixed'])]
            source = {'one.rs': 'source'}
            result = dict(lane='host', failure=None, steps=['one'], original_attempt=False, product=False, server=False)
            row = dict(exit=0, argv=['fixed'])
            for suffix in ('stdout', 'stderr'):
                file = folder / ('one.' + suffix + '.log')
                file.write_bytes(b'first')
                row[suffix + '_sha256'] = c.sha(file)
            with patch.object(c, 'commands', return_value=steps):
                with self.assertRaises(FileNotFoundError): r.verify_lane(root, 'host', source)
                for name, data in [('result', result), ('source-before', source),
                                   ('source-after', source), ('one', row)]: c.publish(folder / (name + '.json'), data)
                r.verify_lane(root, 'host', source)
                for name, original, key, bad in (
                    ('result', result, 'steps', []), ('result', result, 'server', True),
                    ('result', result, 'failure', 'first refused'), ('source-after', source, 'one.rs', 'drift'),
                    ('one', row, 'argv', ['changed']), ('one', row, 'exit', 101),
                    ('one', row, 'stdout_sha256', 'wrong')):
                    changed = copy.deepcopy(original)
                    changed[key] = bad
                    # These are synthetic mutation controls, not retained real receipts.
                    file = folder / (name + '.json')
                    file.write_text(__import__('json').dumps(changed))
                    with self.assertRaises(ValueError): r.verify_lane(root, 'host', source)
                    file.write_text(__import__('json').dumps(original))
                (folder / 'one.stdout.log').write_bytes(b'changed')
                with self.assertRaises(ValueError): r.verify_lane(root, 'host', source)

    def test_existing_packet_refuses_before_any_git_call(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r, 'PACKET', Path(tmp)), \
             patch.object(r.subprocess, 'run') as run:
            with self.assertRaises(ValueError): r.run('unused')
            run.assert_not_called()


if __name__ == '__main__': unittest.main()
