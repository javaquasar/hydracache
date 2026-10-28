import unittest
import json
import difflib
import tomllib
from pathlib import Path
from unittest.mock import patch
import run
import native


class Controls(unittest.TestCase):
    def test_both_contexts_bind_their_own_worker_entry_and_root(self):
        for attempt, suffix, entry in [(1, 'baseline', 'native.py'), (2, 'baseline-corrected', 'native_baseline.py'), (3, 'fixed', 'native_fixed.py'), (4, 'fixed-io-safe', 'native_io_safe.py')]:
            base, target = run.context(attempt)
            argv = run.native_command(attempt)
            self.assertEqual(argv[-1], 'session')
            self.assertEqual(argv[-2], '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-fragments-074/' + entry)
            with patch.object(native, 'ROOT', run.ROOT):
                module = native.configured(attempt, run.ROOT / 'tools/dbus-fragments-074' / entry)
            self.assertEqual(module.BASE, base)
            self.assertEqual(module.NATIVE.as_posix(), target)
            self.assertEqual(module.prepared(), base / 'prepare/result.json')
            self.assertEqual(module.session.__globals__['__file__'], str(run.ROOT / 'tools/dbus-fragments-074' / entry))
            argv, env, artifact, image_target, folder, action = module.command('fd-194-build', {})
            self.assertEqual(action, 'build')
            self.assertIsNone(artifact)
            self.assertIn('--locked', argv)
            self.assertIn('--offline', argv)
            self.assertEqual(folder.as_posix(), target + '/fd')
            self.assertEqual(env['CARGO_HOME'].replace('\\', '/'), target + '/cargo-home')
            self.assertTrue(base.as_posix().endswith('/' + suffix))

    def test_direct_rustix_edge_matches_the_sealed_parent(self):
        packet = run.ROOT / 'docs/testing/performance/0.74/local-runs/dbus-linux-fd-20261028'
        index = (packet / 'index.json').read_bytes()
        self.assertEqual(run.sha(index), '9de2f42856957da48aac3f87f1e8a1d00a6e7f2d259c2f36b978c0a06a4d5ce4')
        row = json.loads(index)['members']['capture/attempt-5/input/fd/Cargo.lock']
        raw = (packet / 'objects' / row['sha256']).read_bytes()
        self.assertEqual(run.sha(raw), row['sha256'])
        nodes = [n for n in tomllib.loads(raw.decode())['package'] if n['name'] == 'rustix']
        self.assertEqual([n['version'] for n in nodes], [run.RUSTIX_VERSION])
        self.assertEqual(run.RUSTIX_VERSION, '1.1.6')

    def test_truncation_patch_has_one_pinned_inverse(self):
        packet = run.ROOT / 'docs/testing/performance/0.74/local-runs/dbus-linux-fd-20261028'
        index = json.loads((packet / 'index.json').read_bytes())
        row = index['members']['capture/attempt-5/input/fd/zbus/src/runtime/io/unix.rs']
        raw = (packet / 'objects' / row['sha256']).read_bytes()
        self.assertEqual(run.sha(raw), run.LEAF_SHA)
        fixed = run.reject_ctrunc(raw)
        call = b'    if msg.bytes == 0 {'
        guard = fixed[fixed.index(b'    if msg.flags.contains'):fixed.index(call)]
        self.assertEqual(fixed.replace(guard, b'', 1), raw)
        self.assertEqual(fixed.count(b'ReturnFlags::CTRUNC'), 1)
        expected = ''.join(difflib.unified_diff(raw.decode().splitlines(True), fixed.decode().splitlines(True),
                                                  fromfile='a/src/runtime/io/unix.rs', tofile='b/src/runtime/io/unix.rs'))
        self.assertEqual((run.ROOT / 'tools/dbus-fragments-074/reject-ctrunc.patch').read_text(encoding='utf8'), expected)

        for changed in (raw + b'\n', fixed):
            with self.assertRaises(ValueError):
                run.reject_ctrunc(changed)
        projected, body = run.projection(fixed, '/owned')
        hook = b'\n    OBSERVED_TRUNC.with(|v| v.set(msg.flags.contains(rustix::net::ReturnFlags::CTRUNC)));'
        self.assertEqual(projected.count(hook), 1)
        self.assertIn(body, projected.replace(hook, b'', 1))
        self.assertIn(b'const FDS_MAX:usize=1024;', projected)

    def test_registration_adapter_changes_only_the_five_constructors(self):
        original = (run.ROOT / 'tools/dbus-fragments-074/tests.rs').read_bytes()
        changed = (run.ROOT / 'tools/dbus-fragments-074/tests_io_safe.rs').read_bytes()
        marker = b'\nfn registered(stream: UnixStream)'
        self.assertEqual(changed.count(marker), 1)
        body, helper = changed.split(marker)
        restored = body.replace(b'registered(a)', b'AsyncFd::new(a).unwrap()').replace(b'registered(b)', b'AsyncFd::new(b).unwrap()')
        self.assertEqual(restored, original)
        self.assertEqual(original.count(b'AsyncFd::new('), 5)
        self.assertIn(b'unsafe { AsyncFd::register(stream) }', helper)
        self.assertNotIn(b'allow(deprecated)', changed)

    def test_unknown_context_never_resolves(self):
        for value in (0, 5, True, '2', None):
            with self.assertRaises(ValueError):
                run.native_command(value)

    def test_complete_42_catalog_rejects_incomplete_or_foreign(self):
        old = ['old::' + str(n) for n in range(31)]
        extras = run.old.EXTRAS + [run.old.EXTRA_DROP] + run.EXTRAS
        names = old + extras
        def raw(values):
            return ('\n'.join(n + ': test' for n in values) + f'\n{len(values)} tests, 0 benchmarks\n').encode()
        self.assertEqual(run.old.inventory(raw(names), old, 'fd', extras), sorted(names))
        for changed in (names[:-1], names[:-1] + ['foreign'], names[:-1] + [names[0]], names + ['foreign']):
            with self.assertRaises(ValueError):
                run.old.inventory(raw(changed), old, 'fd', extras)


if __name__ == '__main__':
    unittest.main()
