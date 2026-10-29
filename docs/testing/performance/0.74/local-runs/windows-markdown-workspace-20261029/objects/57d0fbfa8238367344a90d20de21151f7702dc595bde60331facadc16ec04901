"""Synthetic evidence refusal controls, without source reads or compilation."""
import copy
from pathlib import Path
import tempfile
import unittest
import retain as r


class Controls(unittest.TestCase):
    def fixture(self, root):
        folder = root / '194'; folder.mkdir()
        sources = {'synthetic': '74'}
        for name in ('source-before', 'source-after'): r.c.publish(folder / (name + '.json'), sources)
        result = dict(failure=None, steps=['material', 'projection', 'check', 'clippy'],
            original_attempt=False, product=False, server=False)
        r.c.publish(folder / 'result.json', result)
        for name in result['steps']:
            row = dict(exit=0)
            for suffix in ('stdout', 'stderr'):
                file = folder / (name + '.' + suffix + '.log'); file.write_bytes(b'synthetic')
                row[suffix + '_sha256'] = r.c.sha(file)
            r.c.publish(folder / (name + '.json'), row)
        return sources

    def test_complete_lane_requires_exact_source_scope_steps_and_every_log(self):
        for fault in ('none', 'source', 'log', 'failure', 'step', 'scope'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as directory:
                root = Path(directory); sources = self.fixture(root)
                if fault == 'source': sources = {'synthetic': '75'}
                elif fault == 'log': (root / '194/clippy.stdout.log').write_bytes(b'changed')
                elif fault in ('failure', 'step', 'scope'):
                    file = root / '194/result.json'; value = copy.deepcopy(r.read(file))
                    if fault == 'failure': value['failure'] = 'first'
                    elif fault == 'step': value['steps'].pop()
                    else: value['original_attempt'] = True
                    file.write_text(r.json.dumps(value))
                if fault == 'none': r.verify_lane(root, '194', sources)
                else:
                    with self.assertRaises(ValueError): r.verify_lane(root, '194', sources)


if __name__ == '__main__': unittest.main(verbosity=2)
