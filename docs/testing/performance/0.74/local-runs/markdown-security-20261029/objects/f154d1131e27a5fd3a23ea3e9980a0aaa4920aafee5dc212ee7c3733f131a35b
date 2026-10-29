"""Caller-mode regressions: retain owned-case's outer 45s/capture/stray frame."""
from pathlib import Path
import json
import tempfile
import unittest
from unittest import mock

import common as c
import isolated as i
import native as n


class ProbeStop(Exception):
    pass


class Caller(unittest.TestCase):
    def test_case_command_cannot_be_mistaken_for_owned_case_internal_child(self):
        with tempfile.TemporaryDirectory(dir=c.ROOT/'target') as name:
            out = Path(name)
            path = out/'never-executed'
            path.write_bytes(b'inert')
            image = dict(path=str(path), sha256=c.sha(path.read_bytes()))
            seen = []
            def observe(folder, label, argv, env, deadline):
                seen.append(argv)
                raise ProbeStop()
            with mock.patch.object(i.b, 'invoke', side_effect=observe):
                with self.assertRaises(ProbeStop):
                    i.cases(image, ['caller'], out/'cases', {}, 100)
            # The original Rust selected_alone contract recognizes argv length 4.
            # The harmless libtest color option preserves selection and outer mode.
            self.assertEqual(seen, [[str(path),'--exact','caller','--test-threads=1','--color=never']])

    def test_integration_uses_original_unfiltered_harness_instead_of_internal_entry(self):
        with tempfile.TemporaryDirectory(dir=c.ROOT/'target') as name:
            out = Path(name)
            build = out/'builds/194'
            build.mkdir(parents=True)
            images = []
            for kind in ('lib','test'):
                path = build/kind
                path.write_bytes(b'inert '+kind.encode())
                images.append(dict(path=str(path), sha256=c.sha(path.read_bytes()),kind=[kind],name=kind))
            c.write(out,'images.json',images)
            selected = 'transport::dispatch_negotiated_private_bus_fault_matrix'
            catalogs = {str(build/'lib'):sorted(['case'+str(v) for v in range(1592)]+[selected]),
                        str(build/'test'):['owned_parent']}
            commands = []
            strategies = []
            def execute(folder, label, argv, env, deadline):
                commands.append(argv)
                child = folder/label
                child.mkdir()
                if '--list' in argv:
                    text = ''.join(v+': test\n' for v in catalogs[argv[0]])
                elif '--ignored' in argv:
                    names = ['bare-receiver','healthy','atomic-adapter','fragmented','lost-ack',
                             'foreign-sender','wrong-serial','disconnect','manager-error','no-capability']
                    text = ''.join('HC074-NEGOTIATED case='+v+' result=ok synthetic\n' for v in names)
                    text += 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1592 filtered out;\n'
                else:
                    text = ('test owned_parent ... ok\n'
                            'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n')
                (child/'stdout.log').write_bytes(text.encode())
                return child
            def cases(image, catalog, folder, env, deadline):
                strategies.append(image['kind'])
                return dict(passed=len(catalog)-int(image['kind']==['lib']),
                            ignored=int(image['kind']==['lib']), catalog=len(catalog))
            with mock.patch.object(n,'NATIVE',out), mock.patch.object(n,'environment',return_value=(None,{})), \
                 mock.patch.object(i.b,'invoke',side_effect=execute), mock.patch.object(i,'cases',side_effect=cases):
                i.run('194',out,100)
            self.assertEqual(strategies,[['lib']])
            self.assertIn([str(build/'test'),'--test-threads=1'],commands)


if __name__ == '__main__':
    unittest.main()
