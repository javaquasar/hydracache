import contextlib
import copy
import importlib.util
import io
from pathlib import Path
import tomllib
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SPEC = importlib.util.spec_from_file_location('security_successor_audit',HERE/'audit.py')
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)


class SecurityInputs(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.before = {n:gate.frozen(ROOT,gate.BASE,n) for n in gate.PINS}
        cls.current = {n:gate.read_inside(ROOT,n) for n in gate.PINS}
        cls.old_graph = tomllib.loads(cls.before['Cargo.lock'].decode())
        cls.new_graph = tomllib.loads(cls.current['Cargo.lock'].decode())
        cls.old_manifest = tomllib.loads(cls.before['Cargo.toml'].decode())
        cls.new_manifest = tomllib.loads(cls.current['Cargo.toml'].decode())

    def test_actual_three_nodes_do_not_admit_package_or_projection(self):
        value = gate.security_inputs(self.before,self.current)
        self.assertEqual(value['locked_node_changes'],['rustls','rustls-webpki','tokio'])
        for key in ('production_projection_installed','historical_checks_updated',
                    'roots_candidate_accepted','policy_pass','package_pass','release_admitted'):
            self.assertIs(value[key],False)

    def test_only_crlf_conversion_is_permitted(self):
        converted = {n:gate.canonical(raw).replace(b'\n',b'\r\n') for n,raw in self.current.items()}
        self.assertTrue(gate.security_inputs(self.before,converted)['approved_security_input_delta_verified'])

    def test_stale_versions_refuse(self):
        with self.assertRaisesRegex(ValueError,'approved current pin differs'):
            gate.security_inputs(self.before,self.before)

    def test_wrong_path_and_added_path_refuse(self):
        current = dict(self.current)
        current['Cargo.toml'],current['Cargo.lock'] = current['Cargo.lock'],current['Cargo.toml']
        with self.assertRaisesRegex(ValueError,'pin differs'):
            gate.security_inputs(self.before,current)
        current = dict(self.current,extra=b'not approved')
        with self.assertRaisesRegex(ValueError,'allowlist'):
            gate.security_inputs(self.before,current)

    def test_current_body_not_only_version_is_pinned(self):
        for name in self.current:
            with self.subTest(name=name):
                current = dict(self.current)
                current[name] += b'\n# unreviewed\n'
                with self.assertRaisesRegex(ValueError,'approved current pin differs'):
                    gate.security_inputs(self.before,current)

    def test_no_successful_current_observation_is_cached(self):
        gate.security_inputs(self.before,self.current)
        bad = dict(self.current)
        bad['Cargo.lock'] += b' '
        with self.assertRaisesRegex(ValueError,'pin differs'):
            gate.security_inputs(self.before,bad)
        gate.security_inputs(self.before,self.current)

    def test_wrong_checksum_refuses_even_without_outer_hash_guard(self):
        graph = copy.deepcopy(self.new_graph)
        next(p for p in graph['package'] if p['name'] == 'tokio')['checksum'] = '0'*64
        with self.assertRaisesRegex(ValueError,'version/checksum'):
            gate.validate_graph(self.old_graph,graph)

    def test_extra_dependency_edge_refuses_even_without_outer_hash_guard(self):
        graph = copy.deepcopy(self.new_graph)
        next(p for p in graph['package'] if p['name'] == 'tokio')['dependencies'].append('unreviewed')
        with self.assertRaisesRegex(ValueError,'unapproved graph delta'):
            gate.validate_graph(self.old_graph,graph)

    def test_added_removed_and_duplicate_nodes_refuse(self):
        for mode in ('added','removed','duplicate'):
            with self.subTest(mode=mode):
                graph = copy.deepcopy(self.new_graph)
                if mode == 'added':
                    graph['package'].append(dict(name='unreviewed',version='1.0.0'))
                elif mode == 'removed':
                    graph['package'].pop(0)
                else:
                    graph['package'].append(copy.deepcopy(graph['package'][0]))
                with self.assertRaises(ValueError):
                    gate.validate_graph(self.old_graph,graph)

    def test_pending_root_catalog_cannot_borrow_security_approval(self):
        graph = copy.deepcopy(self.new_graph)
        next(p for p in graph['package'] if p['name'] == 'webpki-roots' and p['version'] == '1.0.8')['version'] = '1.0.10'
        with self.assertRaisesRegex(ValueError,'unapproved graph delta'):
            gate.validate_graph(self.old_graph,graph)

    def test_msrv_features_and_floor_changes_refuse(self):
        for mode in ('msrv','feature','floor'):
            with self.subTest(mode=mode):
                manifest = copy.deepcopy(self.new_manifest)
                if mode == 'msrv':
                    manifest['workspace']['package']['rust-version'] = '1.89'
                elif mode == 'feature':
                    manifest['workspace']['dependencies']['tokio']['features'].append('test-util')
                else:
                    manifest['workspace']['dependencies']['rustls'] = '0.23.41'
                with self.assertRaises(ValueError):
                    gate.validate_manifest(self.old_manifest,manifest)

    def test_git_inputs_are_fixed_before_any_command(self):
        with patch.object(gate.subprocess,'check_output') as call:
            for ref,name in ((gate.BASE,'other'),('HEAD','Cargo.lock')):
                with self.assertRaisesRegex(ValueError,'fixed Git input allowlist'):
                    gate.frozen(ROOT,ref,name)
            call.assert_not_called()


class Catalog(unittest.TestCase):
    def setUp(self):
        self.data = {'a.rs':b'changed','b.rs':b'added','Cargo.lock':b'new'}
        self.syntax = dict(executed_input_code=False,compiler_expansion=False,
            sources=[dict(source=n,sha256=gate.sha(self.data[n]),sites=[]) for n in ('a.rs','b.rs')])
        self.old = dict(sources={'a.rs':gate.sha(b'old')},auxiliary={'Cargo.lock':gate.sha(b'old lock')},
            targets=['unittests src/lib.rs'])

    def test_complete_changed_added_and_auxiliary_lists_are_not_clearance(self):
        calls = []
        def read(name):
            calls.append(name)
            return self.data[name]
        result = gate.catalog_delta(self.old,self.syntax,['unittests src/lib.rs','tests/new.rs'],read)
        self.assertEqual(calls,['a.rs','b.rs','Cargo.lock'])
        self.assertEqual(result['added_sources'],['b.rs'])
        self.assertEqual(set(result['changed_sources']),{'a.rs'})
        self.assertEqual(set(result['changed_auxiliary']),{'Cargo.lock'})
        self.assertEqual(result['added_targets'],['tests/new.rs'])
        for key in ('package_dispatched','current_effect_review_complete','package_pass','original_execution_or_release_admitted'):
            self.assertIs(result[key],False)

    def test_missing_source_and_target_are_not_success(self):
        self.old['sources']['missing.rs'] = '0'*64
        self.old['targets'].append('tests/missing.rs')
        result = gate.catalog_delta(self.old,self.syntax,['unittests src/lib.rs'],self.data.__getitem__)
        self.assertEqual(result['missing_sources'],['missing.rs'])
        self.assertEqual(result['missing_targets'],['tests/missing.rs'])
        self.assertFalse(result['current_effect_review_complete'])

    def test_duplicate_source_refuses_before_io(self):
        self.syntax['sources'].append(self.syntax['sources'][0])
        with self.assertRaisesRegex(ValueError,'duplicate syntax source'):
            gate.catalog_delta(self.old,self.syntax,[],lambda _:self.fail('unexpected IO'))

    def test_stale_syntax_or_duplicate_target_refuses(self):
        self.syntax['sources'][0]['sha256'] = '0'*64
        with self.assertRaisesRegex(ValueError,'fresh source differs'):
            gate.catalog_delta(self.old,self.syntax,[],self.data.__getitem__)
        self.syntax['sources'][0]['sha256'] = gate.sha(self.data['a.rs'])
        with self.assertRaisesRegex(ValueError,'duplicate target'):
            gate.catalog_delta(self.old,self.syntax,['x','x'],self.data.__getitem__)

    def test_wrong_envelope_and_duplicate_json_refuse(self):
        with self.assertRaisesRegex(ValueError,'syntax envelope differs'):
            gate.catalog_delta(self.old,{'exit':0},[],lambda _:self.fail('unexpected IO'))
        with self.assertRaisesRegex(ValueError,'duplicate JSON key'):
            gate.decode(b'{"sources":[],"sources":[]}')

    def test_no_dispatch_or_threshold_cli(self):
        with contextlib.redirect_stderr(io.StringIO()):
            for extra in ('--execute','--filter','--timeout'):
                with self.assertRaises(SystemExit):
                    gate.arguments(['input.json',extra])

    def test_parent_and_absolute_reads_refuse(self):
        for name in ('../outside',str(ROOT/'Cargo.toml')):
            with self.assertRaisesRegex(ValueError,'nonrelative audit input'):
                gate.read_inside(ROOT,name)

    def test_sealed_v6_receipt_membership_is_not_refreshed(self):
        receipt = gate.load_v6(ROOT)
        self.assertEqual((len(receipt['sources']),len(receipt['auxiliary']),len(receipt['targets'])),(368,45,145))

    def test_new_heartbeat_edge_is_not_inherited_clearance(self):
        old = gate.load_v6(ROOT)
        result = gate.heartbeat_inputs(lambda n:gate.read_inside(ROOT,n),old['auxiliary'])
        self.assertEqual(result['outside_previous_auxiliary'],['scripts/ci/watchdog_linux_group.py'])
        self.assertFalse(result['transitive_python_review_complete'])
        self.assertFalse(result['effect_clearance'])

    def test_heartbeat_import_and_companion_mutations_refuse(self):
        inputs = {n:gate.read_inside(ROOT,n) for n in
                  ['scripts/ci/run-with-heartbeat.py',*gate.HEARTBEAT_COMPANIONS]}
        for name in inputs:
            with self.subTest(name=name):
                changed = dict(inputs)
                if name.endswith('run-with-heartbeat.py'):
                    changed[name] = changed[name].replace(b'import LinuxGroup',b'import LinuxGroup as Other')
                else:
                    changed[name] += b'\n# changed\n'
                with self.assertRaises(ValueError):
                    gate.heartbeat_inputs(changed.__getitem__,{})


if __name__ == '__main__':
    unittest.main()
