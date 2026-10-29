import unittest
import json
import difflib
import run
import native


class Controls(unittest.TestCase):
    def setUp(self):
        self.old = ['old::' + str(n) for n in range(31)]

    @staticmethod
    def raw(names):
        return ('\n'.join(n + ': test' for n in names) + f'\n{len(names)} tests, 0 benchmarks\n').encode()

    def test_baseline_and_fd_catalogs_are_exact(self):
        for variant, names in [('baseline', self.old), ('fd', self.old + run.EXTRAS)]:
            self.assertEqual(run.inventory(self.raw(names), self.old, variant), sorted(names))

    def test_incomplete_foreign_duplicate_or_cross_variant_refuses(self):
        names = self.old + run.EXTRAS
        for changed in (names[:-1], names + ['foreign'], names[:-1] + ['foreign'], names[:-1] + [names[0]], self.old):
            with self.assertRaises(ValueError):
                run.inventory(self.raw(changed), self.old, 'fd')
        with self.assertRaises(ValueError):
            run.inventory(self.raw(names), self.old, 'baseline')

    def test_invalid_baseline_variant_and_summary_refuse(self):
        names = self.old + run.EXTRAS
        for old, variant, raw in [(self.old[:-1] + [self.old[0]], 'fd', self.raw(names)),
                                  (self.old, 'foreign', self.raw(names)),
                                  (self.old, 'fd', self.raw(names).replace(b'37 tests', b'0 tests')),
                                  (self.old, 'fd', b'0 tests, 0 benchmarks\n')]:
            with self.assertRaises(ValueError):
                run.inventory(raw, old, variant)

    def test_every_dispatch_selector_reaches_its_matching_context(self):
        for attempt in (1, 2, 3, 4, 5):
            argv = run.native_command(attempt)
            self.assertEqual(argv[:10], ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--cd', '/', '--exec', '/usr/bin/python3', '-B'])
            self.assertEqual(argv[10], '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/dbus-linux-074/native.py')
            base, root, mode = native.context(argv[11])
            self.assertEqual(mode, 'session')
            suffix = '' if attempt == 1 else '-v' + str(attempt)
            self.assertTrue(base.as_posix().endswith('/target/dbus-linux' + suffix + '-20261028'))
            self.assertEqual(root.as_posix(), '/home/artur/.cache/hydracache-074-dbus-linux/20261028-a' + str(attempt))
            worker = 'worker' + suffix
            self.assertEqual(native.context(worker), (base, root, 'worker'))

    def test_extended_lifetime_catalog_requires_the_new_identity(self):
        names = self.old + run.EXTRAS + [run.EXTRA_DROP]
        extras = run.EXTRAS + [run.EXTRA_DROP]
        self.assertEqual(run.inventory(self.raw(names), self.old, 'fd', extras), sorted(names))
        with self.assertRaises(ValueError):
            run.inventory(self.raw(names[:-1]), self.old, 'fd', extras)
        with self.assertRaises(ValueError):
            run.inventory(self.raw(names), self.old, 'fd')

    def test_exact_cloexec_leaf_and_inverse_source_guard(self):
        packet = run.ROOT / 'docs/testing/performance/0.74/local-runs/dbus-read-flows-20261028'
        index = (packet / 'index.json').read_bytes()
        self.assertEqual(run.sha(index), '4be475b3f92e348e8ccde151356d73607e0b609ebcacf406c4e13fc586131fb2')
        row = json.loads(index)['members']['capture/v3/fixture/zbus/src/runtime/io/unix.rs']
        raw = (packet / 'objects' / row['sha256']).read_bytes()
        self.assertEqual(run.sha(raw), row['sha256'])
        patched = run.patch_recvmsg(raw)
        patch = ''.join(difflib.unified_diff(raw.decode('utf8').splitlines(True), patched.decode('utf8').splitlines(True),
                                               fromfile='a/src/runtime/io/unix.rs', tofile='b/src/runtime/io/unix.rs'))
        self.assertEqual((run.ROOT / 'tools/dbus-linux-074/linux-recvmsg-cloexec.patch').read_text(encoding='utf8'), patch)
        self.assertEqual(patched.count(run.RECVMSG_AFTER), 1)
        self.assertEqual(patched.replace(run.RECVMSG_AFTER, run.RECVMSG_BEFORE, 1), raw)
        for bad in (raw + b'\n', raw.replace(b'RecvFlags::empty()', b'RecvFlags::CMSG_CLOEXEC')):
            with self.assertRaises(ValueError):
                run.patch_recvmsg(bad)

    def test_unknown_dispatch_and_native_context_refuse(self):
        for value in (0, 6, True, '3', None):
            with self.assertRaises(ValueError):
                run.native_command(value)
        for value in ('session-v6', 'worker-v6', 'session/../../', '', 'prepare'):
            with self.assertRaises(ValueError):
                native.context(value)


if __name__ == '__main__':
    unittest.main()
