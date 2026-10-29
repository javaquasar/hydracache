import importlib.util
import json
from pathlib import Path
import re
import unittest
import ast

spec=importlib.util.spec_from_file_location('native_fault_preparation',Path(__file__).with_name('fault_prepare.py'))
p=importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


def source():
    return (p.parent.BASE/'input/fd'/p.PKG/p.TEST).read_bytes()


class Controls(unittest.TestCase):
    def test_existing_nine_cases_and_negative_guards_remain_with_native_positive_io(self):
        before=source()
        after=p.project(before)
        block=lambda raw:raw.decode().split('for case in [',1)[1].split(']',1)[0]
        old=re.findall(r'"([a-z-]+)"',block(before))
        new=re.findall(r'"([a-z-]+)"',block(after))
        self.assertEqual([n for n in new if n!='atomic-adapter'],old)
        self.assertEqual(len(new),10)
        for marker in (b'writer_shape(fd.as_raw_fd()), Err(ExchangeError::Request)',
                       b'libc::F_GETFL',b'libc::O_WRONLY',b'Duration::from_secs(60)',
                       b'Duration::from_secs(3)',b'Duration::from_secs(2)'):
            self.assertEqual(before.count(marker),after.count(marker),marker)
        self.assertIn(b'case == "atomic-adapter"',after)
        self.assertIn(b'bus.connect(true, false)',after)

    def test_real_flag_fault_restores_owned_descriptor_before_resuming_same_panic(self):
        after=p.project(source())
        self.assertIn(b'libc::F_SETFD, 0',after)
        restore=after.index(b'libc::F_SETFD, original_flags')
        resume=after.index(b'std::panic::resume_unwind(error)')
        self.assertLess(restore,resume)
        self.assertIn(b'original_flags, libc::FD_CLOEXEC',after)
        self.assertIn(b'guard_refused=true restored=true',after)
        with self.assertRaises(ValueError):p.project(source()+b'\n')

    def test_original_native_failure_is_real_not_relabelled_as_success(self):
        result=json.loads((p.parent.BASE/'private-bus/native-result.json').read_bytes())
        self.assertFalse(result['passed'])
        self.assertTrue(result['source_unchanged'])
        actual=json.loads((p.parent.BASE/'private-bus/fd-194-test/result.json').read_bytes())
        self.assertEqual(actual['exit'],101)
        self.assertEqual(actual['summary'],[['FAILED','0','1','0','0','1592']])

    def test_exact_cohort_driver_renders_without_running_archived_coordinator(self):
        path=Path(__file__).with_name('fault_unit_native.py')
        raw=path.read_text().replace('/mnt/c/Workspace/prj/jq/cashe/hydracache-074/',
                                    p.ROOT.as_posix()+'/')
        tree=ast.parse(raw)
        self.assertIsInstance(tree.body[-1],ast.Expr)
        self.assertEqual(tree.body[-1].value.func.id,'exec')
        tree.body.pop()
        self.assertFalse(any(isinstance(n,ast.Call) and isinstance(n.func,ast.Name) and n.func.id=='exec'
                             for n in ast.walk(tree)))
        scope={}
        exec(compile(tree,'inert-fixed-driver-render','exec'),scope)
        rendered=scope['text']
        ast.parse(rendered)
        self.assertIn("['--include-ignored', '--exact', *listing['selected']]",rendered)
        self.assertIn("[('ok', '65', '0', '0', '0', '1528')]",rendered)
        self.assertIn("need(len(selected)==len(set(selected))==64",rendered)
        self.assertIn("for action in ('list', 'test', 'clippy'):",rendered)
        self.assertIn("catalog_count=1593, passes_per_compiler=65",rendered)


if __name__=='__main__':
    unittest.main()
