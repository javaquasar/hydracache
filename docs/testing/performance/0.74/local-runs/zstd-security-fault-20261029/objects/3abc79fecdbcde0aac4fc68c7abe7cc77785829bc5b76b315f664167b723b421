"""Source mutation controls only, never a live authority or filesystem fault run."""
import ast
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import audit_read as subject


class AuthorityReadAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.texts = subject.source_texts(subject.ROOT)
        cls.facts = subject.analyze()

    def changed(self, path, before, after):
        texts = dict(self.texts)
        self.assertIn(before, texts[path])
        texts[path] = texts[path].replace(before, after, 1)
        with self.assertRaises(ValueError):
            subject.contracts(texts)

    def test_retained_and_temporary_owners_are_distinct_without_cost_claim(self):
        audit = self.facts["audit"]
        self.assertEqual(len(audit["owned_roles"]), 2)
        self.assertEqual(audit["owned_roles"][0]["disposition"], "keep-owned")
        self.assertEqual(audit["owned_roles"][1]["disposition"], "baseline-required-before-transient-buffer-proposal")
        self.assertEqual(audit["explicit_constructor_expressions_per_read"], 1)
        for key in ("measured_allocation_requests", "measured_reallocations", "measured_wall_ns"):
            self.assertIsNone(audit[key])

    def test_open_before_regular_parent_and_bounded_read_order_is_fixed(self):
        for before, after in [
            ("let mut file = open_at(dir, name, false)?;", "let mut file = self.file.try_clone()?;"),
            ("regular(&before, uid, gid, 0o400)?;", ""),
            ("let before = file.metadata()?;", "let before = cached_metadata;"),
            ("dir.metadata()?.dev()", "before.dev()"),
            ("let stamp = Stamp::of(&before);\n        let mut raw", "let mut raw"),
        ]:
            with self.subTest(before=before):
                self.changed(subject.FIXTURE, before, after)
        texts = dict(self.texts)
        old = "let stamp = Stamp::of(&before);\n        let mut raw = Vec::with_capacity(before.len() as usize);"
        new = "let mut raw = Vec::with_capacity(before.len() as usize);\n        let stamp = Stamp::of(&before);"
        texts[subject.FIXTURE] = texts[subject.FIXTURE].replace(old, new)
        with self.assertRaises(ValueError):
            subject.contracts(texts)

    def test_eof_bound_length_stamp_and_error_propagation_shortcuts_refuse(self):
        for before, after in [
            ("before.len() > budget as u64", "before.len() >= budget as u64"),
            (".take(budget as u64 + 1)", ".take(before.len())"),
            (".read_to_end(&mut raw)?", ".read_exact(&mut raw)?"),
            (".read_to_end(&mut raw)?", ".read_to_end(&mut raw).unwrap_or(0)"),
            ("Stamp::of(&file.metadata()?) != stamp", "Stamp::of(&file.metadata()?) == stamp"),
            ("raw.len() as u64 != stamp.len", "false"),
            ("Ok((file, stamp, raw))", "Ok((file, stamp, Vec::new()))"),
        ]:
            with self.subTest(before=before):
                self.changed(subject.FIXTURE, before, after)

    def test_fresh_comparison_must_not_reuse_mutate_or_drop_the_original_evidence(self):
        for before, after in [
            ("let named = Self::read(dir, &self.name, uid, gid, budget)?;", "let named = (self.file.try_clone()?, self.stamp.clone(), self.raw.clone());"),
            ("named.1 != self.stamp || named.2 != self.raw", "named.2 != self.raw"),
            ("named.1 != self.stamp || named.2 != self.raw", "named.1 != self.stamp || self.raw != self.raw"),
            ("        if named.1", "        drop(named.0);\n        if named.1"),
            ("        if named.1", "        self.raw = named.2;\n        if named.1"),
            ("Stamp::of(&self.file.metadata()?) != self.stamp", "false"),
        ]:
            with self.subTest(before=before):
                self.changed(subject.FIXTURE, before, after)

    def test_regular_file_flags_and_all_stamp_fields_are_preserved(self):
        for before in ("!m.is_file()", "m.uid() != uid", "m.gid() != gid", "m.mode() & 0o7777 != mode", "m.nlink() != 1"):
            self.changed(subject.FIXTURE, before, "false")
        flags = "libc::O_RDONLY\n        | libc::O_NOFOLLOW\n        | libc::O_CLOEXEC\n        | libc::O_NONBLOCK"
        for before in ("O_RDONLY", "O_NOFOLLOW", "O_CLOEXEC", "O_NONBLOCK"):
            self.changed(subject.FIXTURE, flags, flags.replace("libc::" + before, "0"))
        for field in self.facts["audit"]["stamp_fields"]:
            self.changed(subject.FIXTURE, "    " + field + ":", "    missing_" + field + ":")

    def test_registry_latching_fixed_route_and_original_budgets_are_preserved(self):
        for before, after in [("            self.failed = true;", "            self.failed = false;"),
                              ("entry.record.revalidate(", "entry.record.cached_check("),
                              ("entry.marker.revalidate(", "entry.marker.cached_check(")]:
            self.changed(subject.FIXTURE, before, after)
        self.changed(subject.FIXED, "self.journal.inspect(trust, host)", "self.journal.view()")
        for before in ("MAX_GENERATIONS: usize = 128", "MAX_RECORD_BYTES: usize = 2048", "COMMIT_MARKER_BYTES: usize = 65"):
            self.changed(subject.REGISTRY, before, before + "0")

    def test_whole_source_pin_refuses_unrelated_changes_but_accepts_line_endings(self):
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "one.rs"
            pin = subject.hashlib.sha256(b"fresh();\n").hexdigest()
            with patch.dict(subject.PINS, {"one.rs": pin}, clear=True):
                file.write_bytes(b"fresh();\r\n")
                subject.source_texts(directory)
                file.write_bytes(b"cached();\n")
                with self.assertRaises(ValueError):
                    subject.source_texts(directory)

    def test_old_packets_boot_rejection_windows_refusals_and_gates_remain(self):
        self.assertEqual(len(self.facts["input_packets"]), 16)
        self.assertEqual(self.facts["input_members_verified"], 928)
        self.assertEqual(self.facts["previous_boot_decision"], "reject-for-adoption")
        self.assertEqual(self.facts["original_windows_seconds"], [300, 60, 10])
        self.assertEqual([r["age_ns"] for r in self.facts["first_refusals"]], [10042390247, 10010835739])
        self.assertEqual(self.facts["new_measurements"], 0)
        self.assertEqual(self.facts["original_owner_profiles"], 0)
        for key in ("runtime_candidate_implemented", "original_material_gate_passed", "full_sigkill_matrix",
                    "reconnect_proven", "clocks_changed", "services_changed", "server", "product"):
            self.assertIs(self.facts[key], False)

    def test_unique_complete_witnesses_and_bodies_not_substring_presence_alone(self):
        text = self.texts[subject.FIXTURE]
        for step in self.facts["audit"]["read_steps"]:
            self.assertEqual(step["path"], subject.FIXTURE)
            self.assertEqual(text[:text.index(step["expression"])].count("\n") + 1, step["line"])
        for changed in (text.replace("    fn read(\n", "    fn read_again(\n"), text + "\n    fn read(\n"):
            texts = dict(self.texts)
            texts[subject.FIXTURE] = changed
            with self.assertRaises(ValueError):
                subject.contracts(texts)

    def test_auditor_has_no_launch_network_clock_or_dynamic_execution_capability(self):
        tree = ast.parse((Path(subject.__file__)).read_text(encoding="utf-8"))
        names = {node.id for node in ast.walk(tree) if isinstance(node, ast.Name)}
        names |= {node.attr for node in ast.walk(tree) if isinstance(node, ast.Attribute)}
        for name in ("subprocess", "Popen", "socket", "system", "sleep", "time_ns", "perf_counter", "eval", "exec"):
            self.assertNotIn(name, names)
        compile(tree, str(subject.__file__), "exec")

    def test_output_is_separate_exclusive_and_read_only_verify_refuses_drift(self):
        subject.require_output(subject.ROOT / subject.OUTPUT / "facts.json")
        for packet in ("sample-boot-inline-20261022", "held-material-live-20261021"):
            with self.assertRaises(ValueError):
                subject.require_output(subject.ROOT / subject.BASE / packet / "facts.json")
        with patch.object(Path, "is_symlink", return_value=True), self.assertRaises(ValueError):
            subject.require_output(subject.ROOT / subject.OUTPUT / "facts.json")
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "facts.json"
            subject.publish(file, dict(first=True))
            with self.assertRaises(FileExistsError):
                subject.publish(file, dict(first=False))
            with self.assertRaises(ValueError):
                subject.publish(file, dict(first=False), verify=True)


if __name__ == "__main__":
    unittest.main(verbosity=2)
