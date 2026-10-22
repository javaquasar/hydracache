"""Offline mutations of source witnesses and arithmetic, never live/probe execution."""
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import audit_owners as subject


class ScratchOwnerAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts=subject.analyze()
        cls.texts=subject.source_texts(subject.ROOT)

    def refuse(self,path,before,after):
        self.assertTrue(before in self.texts[path],"Mutation fixture not present")
        changed=dict(self.texts);changed[path]=changed[path].replace(before,after)
        with self.assertRaises(ValueError):subject.contracts(changed,32)

    def test_sealed_history_and_each_generation_budget(self):
        self.assertEqual(len(self.facts["input_packets"]),19)
        self.assertEqual(self.facts["input_members_verified"],1076)
        rows=self.facts["audit"]["budget_rows"]
        self.assertEqual(len(rows),129)
        for generation,row in enumerate(rows):
            self.assertEqual(row["leaves"],2*generation)
            self.assertEqual(row["successful_cache_capacity_bound"],generation*2113)
            self.assertEqual(row["standalone_wrapper_bytes"],64*generation)
            self.assertEqual(row["cache_plus_standalone_wrappers"],generation*2177)
        self.assertEqual(rows[-1]["successful_cache_capacity_bound"],270464)
        self.assertEqual(rows[-1]["cache_plus_standalone_wrappers"],278656)
        self.assertIsNone(self.facts["audit"]["cardinality"]["process_wide_registry_limit"])

    def test_budget_types_bounds_and_altered_constants_refuse(self):
        for args in ((True,2048,65,32),(1.0,2048,65,32),(-1,2048,65,32),(129,2048,65,32),
                     (1,2049,65,32),(1,2048,64,32),(1,2048,65,24)):
            with self.assertRaises(ValueError):subject.retained_budget(*args)
        for before,after in (("MAX_GENERATIONS: usize = 128;","MAX_GENERATIONS: usize = 129;"),
            ("MAX_RECORD_BYTES: usize = 2048;","MAX_RECORD_BYTES: usize = 4096;"),
            ("COMMIT_MARKER_BYTES: usize = 65;","COMMIT_MARKER_BYTES: usize = 64;")):
            self.refuse(subject.REGISTRY,before,after)

    def test_retained_bytes_entries_and_original_thread_fields_refuse_drift(self):
        for before,after in (("raw: Vec<u8>,","raw: &[u8],"),("record: Leaf,","record: Vec<u8>,"),
            ("marker: Leaf,","marker: Vec<u8>,"),("entries: Vec<Entry>,","entries: Vec<Leaf>,"),
            ("_original_thread: PhantomData<Rc<()>>", "_original_thread: PhantomData<()>")):
            self.refuse(subject.FIXTURE,before,after)
        self.refuse(subject.FIXTURE,"    raw: Vec<u8>,","    raw: Vec<u8>,\n    scratch: Scratch,")

    def test_original_observation_brackets_and_iteration_refuse_drift(self):
        for before,after in (("for entry in &self.entries {","for entry in &mut self.entries {"),
            ("entry.record.revalidate(","entry.record.cached("),
            ("self.directory.inventory()? != self.entries.len()","false"),
            ("self.directory.revalidate()?;","")):
            self.refuse(subject.FIXTURE,before,after)
        self.refuse(subject.SIGNED,"[Step::Journal, Step::Signed, Step::Journal]","[Step::Journal, Step::Signed, Step::Signed]")

    def test_all_five_original_refusal_sites_and_external_routes_remain(self):
        self.assertEqual(self.facts["audit"]["lifecycle"]["journal_failed_assignment_sites"],5)
        self.assertEqual(len(self.facts["audit"]["refusal_gaps"]),8)
        self.refuse(subject.FIXTURE,"self.failed = true;","self.failed = false;")
        self.refuse(subject.FIXED,"self.journal.latch_refusal();","")
        self.refuse(subject.TRUST,"self.authority.latch_refusal();","")

    def test_reopen_append_history_limit_and_retention_remain(self):
        for path,before,after in ((subject.FIXTURE,"let entries = (1..=count)","let entries = (1..count)"),
            (subject.FIXTURE,"history(&proposed)?;",""),
            (subject.FIXTURE,"self.entries.push(entry);","self.entries.clear();"),
            (subject.REGISTRY,"self.revision >= MAX_GENERATIONS as u64","self.revision > MAX_GENERATIONS as u64"),
            (subject.REGISTRY,"records.is_empty() || records.len() > MAX_GENERATIONS","records.is_empty()")):
            self.refuse(path,before,after)

    def test_outer_borrowed_owners_are_not_independent_registries(self):
        self.refuse(subject.FIXED,"journal: FixtureAuthorityRegistry,","journal: Vec<FixtureAuthorityRegistry>,")
        self.refuse(subject.SIGNED,"Fixed(&'authority mut FixedAuthorityRegistry)","Fixed(FixedAuthorityRegistry)")
        self.refuse(subject.TRUST,"authority: &'a mut FixedAuthorityRegistry,","authority: &'a FixedAuthorityRegistry,")
        self.refuse(subject.KERNEL,"authority: &'a mut FixedAuthorityRegistry,","authority: FixedAuthorityRegistry,")
        self.refuse(subject.KERNEL,"self.verify_policy(bytes, &context, authority)","self.verify_policy(bytes, &context, other)")

    def test_model_borrow_cache_eviction_and_successful_capacity_guard_remain(self):
        for before,after in (("owner: &'a Leaf,","owner: Leaf,"),("self.buffer = None;",""),
            ("named.2.capacity() == named.1.len as usize && named.2.capacity() <= budget","true")):
            self.refuse(subject.CANDIDATE,before,after)

    def test_source_and_exclusive_separate_output_guards(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"one.rs"
            with patch.dict(subject.PINS,{"one.rs":hashlib.sha256(b"owned();\n").hexdigest()},clear=True):
                path.write_bytes(b"owned();\r\n");subject.source_texts(directory)
                path.write_bytes(b"shared();\n")
                with self.assertRaises(ValueError):subject.source_texts(directory)
            output=Path(directory)/"facts.json";subject.publish(output,dict(first=True))
            with self.assertRaises(FileExistsError):subject.publish(output,dict(first=False))
        with self.assertRaises(ValueError):subject.require_output(subject.ROOT/subject.previous.OUTPUT/"facts.json")
        with patch.object(Path,"is_symlink",return_value=True),self.assertRaises(ValueError):
            subject.require_output(subject.ROOT/subject.OUTPUT/"facts.json")

    def test_no_measurement_or_admission_follows_static_budget(self):
        self.assertIn("do-not-integrate-borrowed-prototype",self.facts["decision"])
        self.assertEqual(self.facts["new_measurements"],0)
        self.assertEqual(self.facts["original_owner_profiles"],0)
        self.assertEqual(self.facts["original_windows_seconds"],[300,60,10])
        self.assertEqual([r["age_ns"] for r in self.facts["first_refusals"]],[10042390247,10010835739])
        for key in ("runtime_candidate_implemented","original_material_gate_passed","full_sigkill_matrix","reconnect_proven", "clocks_changed","services_changed","server","product"):
            self.assertIs(self.facts[key],False)


if __name__=="__main__":unittest.main(verbosity=2)
