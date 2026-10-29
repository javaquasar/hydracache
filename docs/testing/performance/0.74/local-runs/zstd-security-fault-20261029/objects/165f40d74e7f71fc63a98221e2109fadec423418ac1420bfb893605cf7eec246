"""Reader/runner controls. Synthetic rows here are not timing observations."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


def module(name):
    spec=importlib.util.spec_from_file_location(name,Path(__file__).with_name(name+".py"))
    result=importlib.util.module_from_spec(spec);spec.loader.exec_module(result);return result


reader=module("review_boot_ops")
runner=module("run_boot_ops")
ROOT=Path(__file__).resolve().parents[2]


def boot_rows():
    data=[]
    for sample in range(16):
        for slot in range(8):
            case=(sample+slot)%8 if sample<8 else (sample-8+8-slot)%8
            calls=0 if case<2 else 4096 if case in (2,3,6) else 1024
            data.append(dict(sample=sample,slot=slot,case=reader.CASES[case],iterations=1024,
                selected_calls=calls,selected_cpu_ns=calls,outer_cpu_ns=10000,
                server=False,product=False,native_gate_open=False,overhead_subtracted=False))
    return data


class Controls(unittest.TestCase):
    def comparison_fixture(self):
        packet=ROOT/"docs/testing/performance/0.74/local-runs/authority-open-name-20261021"
        names=packet/"Candidate-194-cpu.log"
        full=packet/"Candidate-194-inspect.log"
        owners=packet/"Candidate-194-owners.log"
        before=dict(helper_fixtures=reader.rows(names,"HC074-OPEN-NAME-FIXTURE "),
            cold_warm=reader.rows(names,"HC074-OPEN-NAME-LEAF "),
            owners_fixtures=reader.rows(owners,"HC074-AUTHORITY-OWNERS-FIXTURE "),
            helper_allocations=reader.rows(names,"HC074-OPEN-NAME-ALLOC "),
            directory_helper_allocations=[r["actual"] for r in reader.rows(names,"HC074-OPEN-NAME-ALLOC ") if r["case"]==5],
            owners=reader.rows(owners,"HC074-AUTHORITY-OWNERS "),
            full_counts=[dict(case=r["case"],sample=r["sample"],counts={k:v//1024 for k,v in r["counts"].items()})
                for r in reader.rows(full,"HC074-VALIDATION-LEAF-ALLOC ")],full_io=[])
        after=copy.deepcopy(before)
        for row in after["helper_allocations"]:
            if row["case"]==5:row["actual"]=dict(allocations=0,reallocations=0,requested_bytes=0)
        after["directory_helper_allocations"]=[dict(allocations=0,reallocations=0,requested_bytes=0)]*3
        for row in after["full_counts"]:
            if row["case"] in ("authority-1","authority-7"):
                row["counts"]["allocations"]-=2;row["counts"]["requested_bytes"]-=8
        for row in after["owners"]:
            factor=2 if row["phase"]=="full" else 1 if row["phase"]=="directory" else 0
            row["counts"]["allocations"]-=factor*1024;row["counts"]["requested_bytes"]-=factor*4096
        return before,after

    def test_deterministic_candidate_target_only_in_synthetic_comparison_control(self):
        before,after=self.comparison_fixture()
        facts=reader.compare(before,after)
        self.assertIs(facts["allocation_reduction_verified"],True)
        self.assertIs(facts["cpu_speedup_claim"],False)
        self.assertIs(facts["cpu_nonregression_proven"],False)

    def test_candidate_missing_reduction_or_unrelated_semantic_drift_refused(self):
        before,after=self.comparison_fixture()
        for mutate in (lambda d:d["full_counts"][0]["counts"].update(allocations=1),
                       lambda d:d["full_io"].append(dict(invented=True)),
                       lambda d:d["cold_warm"][0].update(cold=False),
                       lambda d:d["owners"][0]["counts"].update(allocations=999),
                       lambda d:d["helper_allocations"][-1]["actual"].update(allocations=1024)):
            changed=copy.deepcopy(after);mutate(changed)
            with self.assertRaises(ValueError):reader.compare(before,changed)

    def test_boot_complete_matrix_and_empty_control(self):
        facts=reader.boot_facts(boot_rows())
        self.assertEqual(set(facts),set(reader.CASES))
        self.assertEqual(facts["empty"]["selected_ns_per_window"]["count"],16)
        self.assertIsNone(facts["actual"]["selected_ns_per_window"])

    def test_boot_missing_duplicate_reordered_or_changed_calls_refused(self):
        for change in (lambda d:d.pop(),lambda d:d.reverse(),lambda d:d.__setitem__(1,d[0]),
                       lambda d:d[2].update(selected_calls=1)):
            data=boot_rows();change(data)
            with self.assertRaises(ValueError):reader.boot_facts(data)

    def test_numeric_gate_negative_bool_and_impossible_cpu_refused(self):
        for change in (dict(server=0),dict(native_gate_open=True),dict(selected_cpu_ns=-1),
                       dict(outer_cpu_ns=True),dict(sample=False),dict(selected_cpu_ns=10001)):
            data=boot_rows();data[0].update(change)
            with self.subTest(change=change),self.assertRaises(ValueError):reader.boot_facts(data)

    def test_original_authority_leaf_schedule_and_window_counts(self):
        path=ROOT/"docs/testing/performance/0.74/local-runs/authority-leaf-cpu-20261021/Capture-194-cpu.log"
        data=reader.rows(path,"HC074-LEAF-CPU ")
        self.assertEqual(len(reader.timed_facts(data,"leaf")),40)
        for change in (dict(calls=1),dict(selected_clock_reads=1),dict(sample=20),dict(position=10)):
            changed=copy.deepcopy(data);changed[0].update(change)
            with self.assertRaises(ValueError):reader.timed_facts(changed,"leaf")

    def test_original_authority_phase_schedule_and_no_component_subtraction(self):
        path=ROOT/"docs/testing/performance/0.74/local-runs/authority-cpu-20261021/Capture-194-cpu.log"
        data=reader.rows(path,"HC074-AUTHORITY-CPU ")
        self.assertEqual(len(reader.timed_facts(data,"phases")),14)
        for change in (lambda d:d.pop(),lambda d:d[0].update(phase="invented"),lambda d:d[1].update(sample=0,phase=d[0]["phase"])):
            changed=copy.deepcopy(data);change(changed)
            with self.assertRaises(ValueError):reader.timed_facts(changed,"phases")

    def test_count_types_shape_and_statistical_outliers_preserved(self):
        self.assertEqual(reader.stats([1,2,1000000])["maximum"],1000000)
        for value in (-1,True,0.5,2**128):
            with self.assertRaises(ValueError):reader.number(value)
        with self.assertRaises(ValueError):reader.counts(dict(allocations=0))
        with self.assertRaises(ValueError):reader.stats([])

    def test_image_reader_requires_successful_unique_artifact_and_exact_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            packet=Path(directory)
            root=packet/"repo";root.mkdir();(root/"input").write_text("frozen")
            receipt=dict(failure=None,steps=[dict(name=n,exit=0) for n in runner.CHECKS],inputs=[dict(path="input",sha256=reader.sha(root/"input"))])
            (packet/"Baseline-check-194-exit.json").write_text(json.dumps(receipt))
            artifact=dict(reason="compiler-artifact",target=dict(name="hydracache_long_run_supervisor_074"),profile=dict(test=True),executable="/fixed/image")
            log=packet/"Baseline-check-194-build.log";log.write_text(json.dumps(artifact)+"\n")
            with patch.object(runner,"PACKET",packet),patch.object(runner,"ROOT",root):
                self.assertEqual(runner.image("Baseline","194"),"/fixed/image")
                log.write_text((json.dumps(artifact)+"\n")*2)
                with self.assertRaisesRegex(RuntimeError,"Ambiguous"):runner.image("Baseline","194")
                (root/"input").write_text("drift")
                with self.assertRaisesRegex(RuntimeError,"drift"):runner.image("Baseline","194")

    def test_read_only_context_uses_fixed_images_and_no_service_or_clock_mutation(self):
        # Inspect the wrapper call surface, not its live WSL state from Windows.
        source=Path(runner.__file__).read_text()
        for forbidden in ("sudo", "ssh ", "systemctl", "date -s", "clock_settime", "hwclock"):
            self.assertNotIn(forbidden,source)
        self.assertEqual(len(runner.JOBS),6)
        self.assertIn("1200s",source)
        self.assertIn("Never resume",source)

    def test_writer_is_exclusive_and_bounds_count_cpu_errors(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"facts.json"
            runner.capture.write_new(path,dict(native_gate_open=False))
            with self.assertRaises(FileExistsError):runner.capture.write_new(path,{})
            with self.assertRaises(ValueError):reader.scope(dict(product=1))

    def session(self, fail=False):
        with tempfile.TemporaryDirectory() as directory:
            packet=Path(directory)
            checked=dict(failure=None,steps=[dict(name=n,exit=0) for n in ("tests","check","clippy")],inputs=[])
            (packet/"BaselineCanary-check-xtask-exit.json").write_text(json.dumps(checked))
            context=dict(source="frozen",dirty="",uid=1000,boot="original",pid=42,start_ticks="10")
            calls=[]
            def child(argv,env,log,deadline):
                calls.append(argv)
                log.write_text("unit fixture only, not timing evidence")
                return dict(exit=101 if fail else 0,containment_failure=None,log_sha256="fixture")
            with patch.object(runner,"PACKET",packet),patch.object(runner,"image",return_value="/fixed/image"),\
                 patch.object(runner,"context",return_value=context),patch.object(runner.capture,"sha",return_value=runner.QUALIFICATION),\
                 patch.object(runner.capture,"capture",side_effect=child),patch("sys.argv",["run_boot_ops.py","--source","frozen","--stage","Baseline"]):
                code=runner.main()
                result=reader.load(packet/"Baseline-result.json")
                self.assertEqual(reader.load(packet/"Baseline-postflight.json"),context)
                self.assertEqual(code,1 if fail else 0)
                self.assertEqual(len(calls),1 if fail else 12)
                self.assertIs(result["complete"],not fail)
                self.assertIs(result["native_gate_open"],False)
                with self.assertRaises(RuntimeError):runner.main()

    def test_single_parent_full_schedule_has_postflight_and_cannot_resume(self):
        self.session()

    def test_child_refusal_stops_all_later_jobs_but_keeps_postflight(self):
        self.session(fail=True)


if __name__=="__main__":
    unittest.main(verbosity=2)
