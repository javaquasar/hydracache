import unittest
import eligibility


class Shapes(unittest.TestCase):
    def test_full_and_short_flags_are_not_folded(self):
        for revision, kind in [('a' * 40, 'full40'), ('abcd', 'short'), ('HEAD', 'other_revision')]:
            for flags, suffix in [([], 'default_replacements'), (['--no-replace-objects'], 'no_replace')]:
                self.assertEqual(eligibility.command_shape(['git', *flags, 'show', revision + ':path']),
                                 kind + '_' + suffix)

    def test_unknown_flags_missing_paths_and_injection_are_not_admitted(self):
        for args in [['git', '-c', 'x=y', 'show', 'a' * 40 + ':p'],
                     ['git', 'show', 'a' * 40 + ':'],
                     ['git', 'show', 'a' * 40 + ':a\nb'],
                     ['git', 'show', 'a' * 40 + ':a\0b'],
                     ['git', 'show', '--format=%H']]:
            self.assertEqual(eligibility.command_shape(args), 'other_show_shape')

    def test_invalid_argv_refuses(self):
        for args in [None, [], ['git', None], 'git show']:
            with self.assertRaises(ValueError):
                eligibility.command_shape(args)

    def test_six_manual_anchors_exist_but_none_is_admitted(self):
        sites = eligibility.reviewed_sites()
        self.assertEqual(len(sites), 6)
        self.assertEqual(len({site['path'] for site in sites}), 6)
        self.assertTrue(all(not site['admitted'] and site['line'] > 0 and site['excerpt'] for site in sites))


if __name__ == '__main__':
    unittest.main()
