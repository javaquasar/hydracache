from pathlib import Path
import unittest
import run


class ProjectionControls(unittest.TestCase):
    def test_exact_multiple_sections(self):
        a, b = Path('C:/held/builder.rs'), Path('C:/held/mod.rs')
        marker = lambda p: ('\\\\?\\' + str(p) + ':\n\n').encode()
        raw = marker(a) + b'fn a() {}\n\n' + marker(b) + b'fn b() {}\n'
        originals = {a: b'fn a(){}', b: b'fn b(){}'}
        self.assertEqual(set(run.formatted_sections(raw, originals)), set(originals))
        for altered in (raw.replace(marker(b), b''), raw + marker(a) + b'fn a(){}',
                        b'foreign\n' + raw, raw.replace(b'fn b', b'fn c')):
            with self.assertRaises(ValueError):
                run.formatted_sections(altered, originals)

    def test_child_cannot_become_parent(self):
        a, b = Path('C:/held/builder.rs'), Path('C:/held/mod.rs')
        marker = lambda p: ('\\\\?\\' + str(p) + ':\n\n').encode()
        raw = marker(a) + b'fn b(){}\n' + marker(b) + b'fn a(){}\n'
        with self.assertRaises(ValueError):
            run.formatted_sections(raw, {a: b'fn a(){}', b: b'fn b(){}'})

    def test_only_reviewed_call_may_gain_comma(self):
        path = Path('C:/held/mod.rs')
        marker = ('\\\\?\\' + str(path) + ':\n\n').encode()
        old = b'            write_match_rule_string_component(f, format_args!("arg{i}"), arg, &mut first_component)?;'
        new = b'            write_match_rule_string_component(\n                f,\n                format_args!("arg{i}"),\n                arg,\n                &mut first_component,\n            )?;'
        self.assertEqual(run.formatted_sections(marker + new, {path: old})[path], new)
        with self.assertRaises(ValueError):
            run.formatted_sections(marker + new.replace(b'arg,', b'foreign,'), {path: old})


if __name__ == '__main__':
    unittest.main()
