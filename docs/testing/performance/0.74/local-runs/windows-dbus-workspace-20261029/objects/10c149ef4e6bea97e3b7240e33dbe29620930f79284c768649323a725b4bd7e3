"""Owned tiny repository integration. No actual project preparation or tests."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import contract as c
import native


@unittest.skipUnless(sys.platform.startswith('linux'), 'native Git behavior needs Linux')
class Native(unittest.TestCase):
    def test_offline_independent_complete_checkout_and_reuse_refusal(self):
        with tempfile.TemporaryDirectory(prefix='hc074-native-fixture-') as directory:
            root = Path(directory).resolve()
            source = root / 'source'
            source.mkdir()
            def git(*args):
                return native.git(source, *args)
            git('init', '--template=', '--initial-branch=main', '.')
            (source / 'plain.txt').write_bytes(b'payload\n')
            (source / 'run.sh').write_bytes(b'#!/bin/sh\nexit 0\n')
            (source / 'run.sh').chmod(0o755)
            (source / c.QUAL).parent.mkdir(parents=True)
            (source / c.QUAL).write_bytes((Path(__file__).resolve().parents[2] / c.QUAL).read_bytes())
            git('add', '.')
            git('-c', 'user.name=local fixture', '-c', 'user.email=fixture@invalid', 'commit', '-m', 'owned data only')
            commit = git('rev-parse', 'HEAD').decode().strip()
            tree = c.tree(git('ls-tree', '-r', '-l', '-z', commit))
            refs = c.refs(git('for-each-ref', '--format=%(objectname) %(refname)'))
            lane = root / 'transfer'
            lane.mkdir()
            bundle = lane / 'history.bundle'
            git('bundle', 'create', str(bundle), '--all')
            body = bundle.read_bytes()
            request = lane / 'request.json'
            request.write_text(json.dumps(dict(schema='hc074-native-frontier-transfer-v1', source=commit,
                tree=tree, refs=refs, bundle_sha256=c.sha(body), bundle_bytes=len(body), qualifier_sha256=c.QUAL_SHA)))
            digest = c.sha(request.read_bytes())
            with patch.object(native, 'BASE', root / 'owned-native'), patch.object(native, 'parents'):
                result = native.run(request, digest)
                self.assertFalse(result['tests_dispatched'])
                self.assertFalse(result['linux_frontier_complete'])
                self.assertFalse(result['package_pass'])
                repo = Path(result['repo'])
                self.assertEqual(native.git(repo, 'rev-parse', 'HEAD').decode().strip(), commit)
                self.assertFalse((repo / '.git/objects/info/alternates').exists())
                self.assertEqual((repo / 'plain.txt').read_bytes(), b'payload\n')
                with self.assertRaises(FileExistsError): native.run(request, digest)
                (repo / 'plain.txt').write_bytes(b'drift\n')
                with self.assertRaisesRegex(ValueError, 'bytes differ'):
                    native.validate_files(repo, tree, lambda: None)
                (repo / 'plain.txt').unlink()
                os.symlink(source / 'plain.txt', repo / 'plain.txt')
                with self.assertRaisesRegex(ValueError, 'nonregular'):
                    native.validate_files(repo, tree, lambda: None)
                self.assertEqual((source / 'plain.txt').read_bytes(), b'payload\n')
                self.assertFalse(git('status', '--porcelain'))


if __name__ == '__main__': unittest.main()
