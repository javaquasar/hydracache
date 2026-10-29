import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import check as c
import retain as r


class Retention(unittest.TestCase):
    def test_lane_changed_sources_commands_and_scope_refuse(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); sources = {'one': 'fixed'}
            result = dict(lane='host', failure=None, steps=['one'], server=False, product=False, original_attempt=False)
            row = dict(argv=['fixed'], exit=0)
            for suffix in ('stdout', 'stderr'):
                file = root / ('one.' + suffix + '.log'); file.write_bytes(b'first'); row[suffix + '_sha256'] = c.sha(file)
            for name, data in [('result', result), ('source-before', sources), ('source-after', sources), ('one', row)]:
                c.publish(root / (name + '.json'), data)
            with patch.object(c, 'commands', return_value=[('one', ['fixed'])]):
                r.verify_lane(root, 'host', sources)
                for name, original, key, bad in [('result', result, 'failure', 'failed'), ('result', result, 'steps', []),
                    ('result', result, 'server', True), ('source-after', sources, 'one', 'drift'),
                    ('one', row, 'exit', 1), ('one', row, 'argv', ['other']), ('one', row, 'stdout_sha256', 'drift')]:
                    file = root / (name + '.json'); file.write_text(json.dumps(dict(original, **{key: bad})))
                    with self.assertRaises(ValueError): r.verify_lane(root, 'host', sources)
                    file.write_text(json.dumps(original))

    def test_existing_packet_refuses_before_commands(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r, 'PACKET', Path(tmp)), patch.object(c.logger.subprocess, 'run') as run:
            with self.assertRaises(ValueError): r.run('unused')
            run.assert_not_called()

    def test_success_status_cannot_replace_a_sealed_source_refusal(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            c.publish(root / 'red.json', dict(exit=101)); c.publish(root / 'bootstrap-red.json', dict(exit=101))
            c.publish(root / 'probe.json', dict(exit=0, source_sha256='fixed'))
            with self.assertRaisesRegex(ValueError, 'sealed-source refusal'):
                r.verify_negative(root, {'tools/held-bind-alias-074/probe.py': 'fixed'})


if __name__ == '__main__': unittest.main()
