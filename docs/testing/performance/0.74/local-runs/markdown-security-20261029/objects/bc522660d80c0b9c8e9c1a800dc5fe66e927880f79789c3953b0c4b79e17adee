import hashlib
from pathlib import Path
import unittest

import profile_session as p


class ProfileTests(unittest.TestCase):
    def template(self):
        return Path(__file__).with_name('holder-session.sh').read_bytes()

    def test_exact_safety_frame_and_one_case_only(self):
        raw = self.template()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), p.TEMPLATE_SHA256)
        value = p.render(raw).decode().replace('\r\n', '\n')
        old = raw.decode().replace('\r\n', '\n')
        for name in ('preflight', 'inputs', 'finish'):
            def body(text):
                return text.split(name + '() {\n')[1].split('\n}\n')[0]
            expected = body(old).replace('holder-session.sh', 'profile-session.sh').replace(
                'HC074_NATIVE_HOLDER exit=', 'HC074_NATIVE_PROFILE exit=')
            self.assertEqual(body(value), expected)
        self.assertIn('mkdir profile-once\n', value)
        self.assertIn('HC074_CONSUMING_HELD_LIVE_CASE=healthy HC074_CONSUMING_HELD_COST=1', value)
        self.assertEqual(value.count('nice -n 10 "$root/libtest"'), 1)
        self.assertNotIn('for case in healthy', value)
        self.assertTrue(value.endswith('exit "$result"\n'))

    def test_template_drift_is_not_silently_rebased(self):
        for raw in (b'', self.template() + b'\n', self.template().replace(b'1200s', b'1201s')):
            with self.assertRaises(ValueError):
                p.render(raw)


if __name__ == '__main__':
    unittest.main()
