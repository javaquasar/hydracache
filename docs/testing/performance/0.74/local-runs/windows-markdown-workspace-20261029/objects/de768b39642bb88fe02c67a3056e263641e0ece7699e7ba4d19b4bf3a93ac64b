"""Offline controls only; never invoke a privileged Rust fixture."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
import runner as base
import run_material_live as live
import review_material_live as review


class SessionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.output = Path(self.tmp.name)/"packet"
        self.pins = dict(source="a"*40,bundle=[],images={"194":"b","188":"c"})
        self.context = dict(**self.pins,tracked_dirty="",uid=0,fixtures=[],boot="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",qualification_sha256=base.QUALIFICATION)
        self.calls = []

    def observe(self):
        return copy.deepcopy(self.context)

    def execute(self,lane,log,deadline):
        self.calls.append(lane)
        return dict(exit=101)

    def test_first_refusals_retained_one_call_each_and_no_resume(self):
        result = live.run_session(self.output,self.pins,self.observe,self.execute)
        self.assertEqual(self.calls,["194","188"])
        self.assertEqual([r["exit"] for r in result["profiles"]],[101,101])
        self.assertFalse(result["schedule_completed"])
        self.assertEqual(base.read_json(self.output/"intent.json")["original_windows_seconds"],[300,60,10])
        with self.assertRaises(FileExistsError):
            live.run_session(self.output,self.pins,self.observe,self.execute)
        self.assertEqual(len(self.calls),2)

    def test_context_drift_stops_everything(self):
        def observe():
            current=self.observe()
            if self.calls: current["boot"]="bbbbbbbb-bbbb-cccc-dddd-eeeeeeeeeeee"
            return current
        result=live.run_session(self.output,self.pins,observe,self.execute)
        self.assertEqual(self.calls,["194"])
        self.assertIsNotNone(result["failure"])

    def test_preflight_refuses_without_dispatch(self):
        def prepare(): raise RuntimeError("preflight")
        result=live.run_session(self.output,self.pins,self.observe,self.execute,prepare=prepare)
        self.assertEqual(self.calls,[])
        self.assertIn("preflight",result["failure"])
        self.assertTrue((self.output/"postflight.json").exists())

    def test_clean_environment_only_healthy_opt_in(self):
        env=live.child_env(Path("/repo"))
        self.assertEqual(env["HC074_CONSUMING_HELD_LIVE_CASE"],"healthy")
        self.assertEqual(env["HC074_HELD_MATERIAL_LIVE"],"original-once-v1")
        self.assertNotIn("LD_PRELOAD",env)

    def test_success_needs_both_integer_zero_receipts(self):
        def execute(lane,log,deadline): return dict(exit=False if lane=="194" else 0)
        result=live.run_session(self.output,self.pins,self.observe,execute)
        self.assertFalse(result["schedule_completed"])

    def test_budget_refuses_without_dispatch(self):
        times=iter([0,7500])
        result=live.run_session(self.output,self.pins,self.observe,self.execute,clock=lambda:next(times,7500))
        self.assertEqual(self.calls,[])
        self.assertIn("budget",result["failure"])

    def test_postflight_failure_never_becomes_success(self):
        def observe():
            if len(self.calls)==2: raise RuntimeError("postflight")
            return self.observe()
        result=live.run_session(self.output,self.pins,observe,self.execute)
        self.assertFalse(result["schedule_completed"])
        self.assertIn("postflight",result["failure"])


class ReaderTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.log=Path(self.tmp.name)/"profile.log"
        packet=Path(__file__).resolve().parents[2]/"docs/testing/performance/0.74/local-runs/directory-preparation-impact-20261021"
        self.rows=review.prior.rows(packet/"194-Profile-0.log","HC074-HELD-PHASE ")
        self.original=review.prior.rows(packet/"194-Profile-0.log","HC074-HELD-LIVE-ORIGINAL ")[0]
        self.metadata=base.read_json(packet/"metadata.json")

    def encode(self,failed=False):
        rows=copy.deepcopy(self.rows)
        extra=copy.deepcopy(rows[-1]);extra["phase"]="projection-materialization";extra["returned"]=not failed
        rows.append(extra)
        original=copy.deepcopy(self.original);original["returned"]=not failed
        text="".join("HC074-HELD-PHASE "+json.dumps(r)+"\n" for r in rows)
        for stage in review.STAGES[:1 if failed else 5]: text+="HC074-HELD-MATERIAL stage="+stage+"\n"
        text+="HC074-HELD-LIVE-ORIGINAL "+json.dumps(original)+"\n"
        if not failed: text+=review.FOOTER+"\n"
        self.log.write_text(text,encoding="utf-8")
        return dict(lane="194",exit=101 if failed else 0,containment_failure=None,log_sha256=base.sha(self.log),bytes=self.log.stat().st_size,wall_seconds=7.0)

    def test_complete_and_first_refused_prefix_are_distinct(self):
        for failed in (False,True):
            receipt=self.encode(failed)
            facts=review.profile_facts(self.log,receipt,self.metadata)
            self.assertIs(facts["harness_passed"],not failed)
            self.assertEqual(len(facts["stages"]),1 if failed else 5)

    def test_scope_order_raw_exit_and_cleanup_mutations_refuse(self):
        for mutation in ("exit","raw","stage","footer","phase","scope","duplicate","boot","age"):
            receipt=self.encode();text=self.log.read_text()
            if mutation=="exit":receipt["exit"]=False
            if mutation=="raw":receipt["log_sha256"]="0"*64
            if mutation=="stage":text=text.replace("stage=projection-return","stage=material-return")
            if mutation=="footer":text=text.replace(review.FOOTER,"")
            if mutation=="phase":text=text.replace('"phase": "projection-materialization"','"phase": "other"')
            if mutation=="scope":text=text.replace('"deadline_changed": false','"deadline_changed": true')
            if mutation=="duplicate":text+="HC074-HELD-MATERIAL stage=projection-enter\n"
            if mutation in ("boot","age"):
                lines=text.splitlines()
                for i,line in enumerate(lines):
                    if line.startswith("HC074-HELD-LIVE-ORIGINAL "):
                        row=json.loads(line.split(" ",1)[1])
                        if mutation=="boot":row["causal"]["last"]["now_boot"]="foreign"
                        else:row["causal"]["last"]["age_ns"]=10_000_000_000
                        lines[i]="HC074-HELD-LIVE-ORIGINAL "+json.dumps(row)
                text="\n".join(lines)+"\n"
            if mutation not in ("raw","exit"):
                self.log.write_text(text,encoding="utf-8");receipt.update(log_sha256=base.sha(self.log),bytes=self.log.stat().st_size)
            with self.subTest(mutation=mutation),self.assertRaises(ValueError):
                review.profile_facts(self.log,receipt,self.metadata)


if __name__=="__main__": unittest.main(verbosity=2)
