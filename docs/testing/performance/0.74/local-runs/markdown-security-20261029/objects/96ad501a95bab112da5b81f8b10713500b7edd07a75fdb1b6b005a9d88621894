"""Offline source-contract mutations; never executes the inspected Rust tests."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import audit_integration as subject


class IntegrationAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts=subject.analyze();cls.texts=subject.source_texts(subject.ROOT)

    def mutate(self,path,before,after):
        texts=dict(self.texts);self.assertIn(before,texts[path]);texts[path]=texts[path].replace(before,after)
        with self.assertRaises(ValueError):subject.contracts(texts)

    def test_scope_and_sealed_history_are_distinct(self):
        self.assertEqual((len(self.facts['input_packets']),self.facts['input_members_verified']),(23,1363))
        self.assertIs(self.facts['targeted_model_gate'],True)
        self.assertIs(self.facts['prior_broad_isolation_gate'],False)
        self.assertEqual(self.facts['model_images_executed'],0)
        self.assertEqual(len(self.facts['audit']['blockers']),4)

    def test_all_failure_assignments_are_attributed(self):
        self.assertEqual([r['site'] for r in self.facts['audit']['failure_sites']],['external','validation','binding','transition','publication'])
        path=subject.owners.FIXTURE
        self.mutate(path,'self.failed = true;','self.failed = false;')
        texts=dict(self.texts);texts[path]+='\nfn hidden() { self.failed = true; }\n'
        with self.assertRaises(ValueError):subject.contracts(texts)

    def test_external_fixed_signed_and_trust_routes_cannot_disappear(self):
        for path,before in ((subject.owners.FIXED,'self.journal.latch_refusal();'),
            (subject.owners.TRUST,'self.authority.latch_refusal();'),
            (subject.owners.SIGNED,'const BRACKET: [Step; 3] = [Step::Journal, Step::Signed, Step::Journal];')):
            self.mutate(path,before,'/* missing original refusal route */')

    def test_constructor_originals_and_drop_are_independent(self):
        for before,after in [('file: File,','file: u64,'),('raw: Vec<u8>,','raw: (),'),
            ('let _ = FileExt::unlock(&self.leaf);',''),('read.directory.temporary.sync_all()?;','')]:
            self.mutate(subject.owners.FIXTURE,before,after)

    def test_reopen_and_append_cannot_adopt_or_skip_history(self):
        for before,after in [('let entries = (1..=count)','let entries = (count..=count)'),
            ('read.revalidate()?;',''),('self.entries.push(entry);','self.entries.clear();')]:
            self.mutate(subject.owners.FIXTURE,before,after)

    def test_fresh_read_and_outer_rounds_remain_complete(self):
        for before,after in [('regular(&before, uid, gid, 0o400)?;',''),('.take(budget as u64 + 1)','.take(budget as u64)'),
            ('named.1 != self.stamp || named.2 != self.raw','false'),('for entry in &self.entries {','for entry in &self.entries[..0] {')]:
            self.mutate(subject.owners.FIXTURE,before,after)

    def test_change_priority_is_not_inherited_from_legacy_model(self):
        before='        self.revalidate()?;\n        let next = self'
        self.mutate(subject.owners.FIXTURE,before,'        let next = self')

    def test_publication_sync_and_checkpoint_order_refuse_shortcuts(self):
        for before,after in [('self.directory.leaf.sync_all()?;',''),('file.sync_all()?;',''),
            ('write_bytes(file, bytes)?;',''),('fail(fault, PublicationStage::DurableCommit)?;',''),
            ('file.set_permissions(fs::Permissions::from_mode(0o400))?;','')]:
            self.mutate(subject.owners.FIXTURE,before,after)

    def test_actual_advance_requires_more_than_prechecked_model(self):
        for before in ('next.machine_id != self.machine_id','next.issuer_key_hex != self.issuer_key_hex',
            'next.policy_sha256 == self.policy_sha256','next.policy_epoch <= self.policy_epoch'):
            self.mutate(subject.owners.REGISTRY,before,'false')

    def test_observer_and_current_guard_boundaries_are_explicit(self):
        self.mutate(subject.CONTROL,'    leaf: &Leaf,','    leaf: &mut Leaf,')
        self.mutate(subject.CONTROL,'for entry in &self.registry.entries {','for entry in &mut self.registry.entries {')
        for path in subject.BOUNDARIES:
            before=subject.BOUNDARIES[path][0]
            self.mutate(path,before,'changed-boundary')

    def test_source_output_and_no_clobber_guards(self):
        with patch.object(subject,'canonical',return_value='drift'),self.assertRaises(ValueError):subject.source_texts(subject.ROOT)
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)/'facts.json';subject.publish(output,dict(first=True))
            with self.assertRaises(FileExistsError):subject.publish(output,dict(first=False))
        with self.assertRaises(ValueError):subject.require_output(subject.ROOT/subject.previous.OUTPUT/'facts.json')

    def test_deadlines_first_refusals_and_real_gates_do_not_move(self):
        self.assertEqual(self.facts['new_measurements'],0)
        self.assertEqual(self.facts['original_windows_seconds'],[300,60,10])
        self.assertEqual([r['age_ns'] for r in self.facts['first_refusals']],[10042390247,10010835739])
        for key in ('runtime_candidate_implemented','original_material_gate_passed','full_sigkill_matrix','reconnect_proven','clocks_changed','services_changed','server','product'):
            self.assertIs(self.facts[key],False)


if __name__=='__main__':unittest.main(verbosity=2)
