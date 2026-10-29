"""Read-only owner, lifetime and proposed retained-capacity audit. No test images."""
import argparse
import hashlib
from pathlib import Path
import re
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/"tools/authority-scratch-reuse-074"))
import review_reuse as previous

need,exact,load,sha=previous.need,previous.exact,previous.load,previous.sha
canonical,publish,BASE=previous.canonical,previous.publish,previous.BASE
authority=previous.previous.prior
section,ordered,witness=authority.section,authority.ordered,authority.witness
OUTPUT=BASE/"authority-scratch-owner-audit-20261022"
INPUTS=dict(previous.INPUTS,**{"authority-scratch-reuse-20261022":"b5a90f56551c11378422879ae861a05c8fa3c9bb6448e9dc8c1f255ee3f20159"})
PREFIX="tools/long-run-supervisor-074/src/"
FIXTURE=PREFIX+"diagnostic_authority_fixture.rs"
REGISTRY=PREFIX+"diagnostic_authority_registry.rs"
FIXED=PREFIX+"diagnostic_authority_fixed.rs"
SIGNED=PREFIX+"diagnostic_signed_authority.rs"
TRUST=PREFIX+"diagnostic_worker_trust_input.rs"
KERNEL=PREFIX+"diagnostic_worker_kernel_binding.rs"
CANDIDATE="tools/authority-scratch-reuse-074/candidate.rs"
PINS={
    "tools/long-run-supervisor-074/src/diagnostic_authority_registry.rs": "1e936fe855eedad6d0e5dacbd76d534a45cc8df11cd30f06f8e2ed77258bc53e",
    "tools/long-run-supervisor-074/src/diagnostic_authority_fixed.rs": "fd1e10a038ae046888c8ff3c76303e36e5ed49a46aef2025391f8aa009d554a1",
    "tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs": "c4f04ccb7b3d7bd17e13241b1cefc6657661e5560d1b1ce51f3445e71d5202ce",
    "tools/authority-scratch-reuse-074/candidate.rs": "858f79d15f14bec48ba5c00fb3f7e959931aa1b9f739a71585b09ec438d47b8f",
    "tools/long-run-supervisor-074/src/diagnostic_signed_authority.rs": "9fa67be28d71c843f5c10e0de50cc0307dffc8c7e58b69a4b24ca1dc15311990",
    "docs/testing/performance/0.74/authority-scratch-owner-audit-plan-20261022.md": "67bf0371be91dd6240663a677906a23d067e7d6a1ef792f0c25fb4b7df629fdd",
    "tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs": "153a71dc617694e2b275fecfa1dc09faafb85a57920794a3e7005858bfb732c0",
    "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs": "3b3c75f41dbd8a8db1f73d61af3b0c701a666c3ebeed326ae49012ed0c01fb6c",
    "tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs": "891fa2ec885cf3193f575afe4e4b2f4f70a885868d9868b1843a49753d6f8807"
}


def source_texts(root):
    texts={}
    for path,pin in PINS.items():
        text=canonical(Path(root)/path)
        exact(hashlib.sha256(text.encode()).hexdigest(),pin,"Owner audit source drift: "+path)
        texts[path]=text
    return texts


def constant(text,name):
    matches=re.findall(r"^pub const "+name+r": usize = (\d+);$",text,re.M)
    exact(len(matches),1,"One fixed bound required: "+name)
    return int(matches[0])


def retained_budget(generations,record,marker,wrapper):
    for value in (generations,record,marker,wrapper):
        need(type(value) is int and value>=0,"Nonnegative integer capacity inputs required")
    need(generations<=128 and record==2048 and marker==65 and wrapper==32,"Frozen budget/layout inputs required")
    leaves=generations*2
    buffers=generations*(record+marker)
    wrappers=leaves*wrapper
    return dict(generations=generations,leaves=leaves,successful_cache_capacity_bound=buffers,
        standalone_wrapper_bytes=wrappers,cache_plus_standalone_wrappers=buffers+wrappers)


def contracts(texts,wrapper):
    fixture,registry,fixed,signed,trust,kernel,candidate=(texts[p] for p in (FIXTURE,REGISTRY,FIXED,SIGNED,TRUST,KERNEL,CANDIDATE))
    generations=constant(registry,"MAX_GENERATIONS");record=constant(registry,"MAX_RECORD_BYTES");marker=constant(registry,"COMMIT_MARKER_BYTES")
    exact((generations,record,marker),(128,2048,65),"Frozen journal limits changed")
    # The earlier complete-body audit still checks every fresh observation.
    authority.contracts({p:texts[p] for p in authority.PINS})
    leaf=section(fixture,"struct Leaf {","impl Leaf {")
    entry=section(fixture,"struct Entry {","impl Entry {")
    journal=section(fixture,"pub struct FixtureAuthorityRegistry {","pub fn initialize_fixture_registry(")
    for text,expressions in ((leaf,["file: File,","stamp: Stamp,","raw: Vec<u8>,","name: String,"]),
        (entry,["record: Leaf,","marker: Leaf,"]),
        (journal,["directory: Directory,","entries: Vec<Entry>,","records: Vec<Record>,","failed: bool,","_original_thread: PhantomData<Rc<()>>,"])):
        ordered(text,expressions)
    need("Scratch" not in leaf+entry+journal and "RefCell" not in leaf+entry+journal,"No embedded scratch or interior-mutability candidate was admitted")
    opened=section(fixture,"fn open_directory(","impl FixtureAuthorityRegistry {")
    ordered(opened,["directory.revalidate()?;","let count = directory.inventory()?;","if count == 0 {",
        "let entries = (1..=count)","Entry::open(&directory.leaf, revision, directory.uid, directory.gid)",
        ".collect::<Result<Vec<_>, _>>()?;","let records = history(",".bind(trust, host)?;","let mut read = FixtureAuthorityRegistry {","read.revalidate()?;"])
    revalidate=section(fixture,"    fn revalidate(&mut self)","    /// Reinspect")
    ordered(revalidate,["if self.failed {","let result = (|| {","for entry in &self.entries {","entry.record.revalidate(","entry.marker.revalidate(","if result.is_err() {","self.failed = true;","        result\n"])
    exact(revalidate.count("self.directory.revalidate()?;"),2,"Directory brackets changed")
    exact(revalidate.count("self.directory.inventory()? != self.entries.len()"),2,"Inventory brackets changed")
    published=section(fixture,"    fn publish(\n","fn name(")
    ordered(published,["let bytes = record.bytes()?;","let mut proposed = self", "history(&proposed)?;",
        "let mut file = create_new(","fail(fault, PublicationStage::DurableCommit)?;","let entry = Entry::open(",
        "if entry.record.raw != bytes {","self.entries.push(entry);","self.records.push(record);"])
    exact(published.count("self.revalidate()?;"),2,"Publication brackets changed")
    need(published.index("self.revalidate()?;")<published.index("let bytes = record.bytes()?;") and
         published.rindex("self.revalidate()?;")>published.index("self.records.push(record);"),"Publication bracket order changed")
    next_revision=section(registry,"    fn next_revision(","fn key_hex(")
    ordered(next_revision,["if self.revision >= MAX_GENERATIONS as u64 {","return Err(AuthorityError::Budget);","Ok(self.revision + 1)"])
    history=section(registry,"fn history(","/// Strict bounded byte inspection")
    need("records.is_empty() || records.len() > MAX_GENERATIONS" in history,"History bound missing")
    for forbidden in ("self.entries.clear(","self.entries.remove(","self.entries.pop(","self.entries.retain(","self.entries =", "impl Drop for FixtureAuthorityRegistry", "impl Clone for FixtureAuthorityRegistry"):
        need(forbidden not in fixture,"Original retained-entry lifecycle changed: "+forbidden)
    exact(fixture.count("self.entries.push(entry);"),1,"Append path changed")
    failed_sections=[
        section(fixture,"    pub(crate) fn latch_refusal(","    pub fn is_failed("),
        revalidate,
        section(fixture,"    pub fn inspect(\n","    pub fn refuse("),
        section(fixture,"    fn change(\n","    fn publish(\n"),
        published,
    ]
    for part in failed_sections:exact(part.count("self.failed = true;"),1,"Journal refusal site changed")
    exact(fixture.count("self.failed = true;"),5,"Unattributed journal refusal site")
    need("self.journal.latch_refusal();" in fixed,"Fixed refusal no longer reaches original journal")
    ordered(section(fixed,"pub struct FixedAuthorityRegistry {","/// Explicit initialization"),["journal: FixtureAuthorityRegistry,"])
    need("FileExt::try_lock_exclusive(&leaf)?;" in fixture and "FileExt::try_lock_exclusive(&leaf)?;" in fixed,"Cooperative directory lock missing")
    need("let _ = FileExt::unlock(&self.leaf);" in fixture,"Original directory drop changed")
    borrowed=section(signed,"enum Journal<'authority> {","impl Journal<'_> {")
    ordered(borrowed,["Fixed(&'authority mut FixedAuthorityRegistry)","Fixture(&'authority mut FixtureAuthorityRegistry)"])
    policy=section(trust,"pub struct WorkerPolicyInput<'s, 'a> {","impl WorkerPolicyInput<'_, '_> {")
    ordered(policy,["input: &'s mut WorkerTrustInput,","authority: &'a mut FixedAuthorityRegistry,","policy: CheckedWorkerPolicy,"])
    verify_kernel=section(kernel,"    pub fn verify_kernel_policy<'s, 'a, 'p>(","        let context =")
    need("authority: &'a mut FixedAuthorityRegistry," in verify_kernel,"Kernel policy no longer borrows the same journal")
    need("let mut policy = match self.verify_policy(bytes, &context, authority)" in kernel,"Kernel join replaced the authority owner")
    need("self.authority.latch_refusal();" in trust,"External trust refusal no longer reaches original journal")
    need("const BRACKET: [Step; 3] = [Step::Journal, Step::Signed, Step::Journal];" in signed,"Signed bracket changed")
    ordered(section(candidate,"pub(super) struct Scratch<'a> {","impl<'a> Scratch<'a> {"),["owner: &'a Leaf,","buffer: Option<Vec<u8>>,"])
    need("let result = self.validate(dir, uid, gid, budget);\n        if result.is_err() {\n            self.buffer = None;" in candidate,"Leaf-local error eviction changed")
    need("named.2.capacity() == named.1.len as usize && named.2.capacity() <= budget" in candidate,"Successful retained-capacity guard missing")
    witness_pairs=[(FIXTURE,"    entries: Vec<Entry>,"),(FIXTURE,"            for entry in &self.entries {"),
        (FIXTURE,"            self.entries.push(entry);"),(FIXTURE,"    pub(crate) fn latch_refusal(&mut self) {"),
        (FIXED,"pub struct FixedAuthorityRegistry {\n    journal: FixtureAuthorityRegistry,"),(TRUST,"\n    authority: &'a mut FixedAuthorityRegistry,"),
        (SIGNED,"enum Journal<'authority> {"),(KERNEL,"        authority: &'a mut FixedAuthorityRegistry,"),
        (CANDIDATE,"    owner: &'a Leaf,"),(CANDIDATE,"            self.buffer = None;")]
    return dict(fixed_budgets=dict(generations=generations,record=record,marker=marker),
        cardinality=dict(entries_per_journal_max=128,leaves_per_entry=2,leaves_per_journal_max=256,
            process_wide_registry_limit=None,exclusive_borrow_not_process_cardinality=True),
        lifecycle=dict(open_rebuilds_all_entries=True,append_retains_previous_entries=True,refusal_retains_original_entries=True,
            original_thread_affinity=True,fixed_wraps_shared_engine=True,higher_layers_borrow_original_journal=True,
            journal_failed_assignment_sites=5,directory_brackets=2,inventory_brackets=2),
        budget_rows=[retained_budget(g,record,marker,wrapper) for g in range(generations+1)],
        budget_scope="proposed successful cached capacity plus measured standalone wrappers per journal, not actual runtime memory or transient peak",
        original_expected_bytes_files_stamps="keep independently owned; no replacement or eviction",
        self_borrowing_sidecar="not directly embeddable in its own Leaf; append/Vec relocation forbids retaining unpinned borrowed sidecars",
        mutable_integration="owned storage would require a separate mutable-receiver/split-borrow candidate, not interior-mutability by default",
        refusal_gaps=["directory-or-inventory-before-first-leaf","record-before-marker","marker-after-record",
            "directory-or-inventory-after-last-leaf","inspect-binding","change-transition","publish-error","external-latch"],
        alternatives=[dict(placement="borrowed-sidecar",disposition="model-only; cannot embed self borrow or outlive append"),
            dict(placement="owned-per-leaf",disposition="next isolated model candidate; measure layout and clear all caches on journal refusal"),
            dict(placement="per-round",disposition="new allocations each round; does not inherit cross-round savings"),
            dict(placement="shared-rebound",disposition="different ownership/capacity hypothesis; not validated by this prototype")],
        witnesses=[witness(path,texts[path],expression) for path,expression in witness_pairs])


def analyze(root=ROOT):
    root=Path(root)
    packets=[previous.core.prior.verify_packet(root/BASE/name,pin) for name,pin in INPUTS.items()]
    exact(sum(p["members"] for p in packets),1076,"Prior sealed history drift")
    exact(canonical(root/"tools/authority-scratch-reuse-074/review_reuse.py"),canonical(root/BASE/"authority-scratch-reuse-20261022/method/tools/authority-scratch-reuse-074/review_reuse.py"),"Frozen scratch reviewer drift")
    old=previous.analyze(root)
    exact(old,load(root/BASE/"authority-scratch-reuse-20261022/facts.json"),"First scratch observations changed")
    exact(old["decision"],"retain-unpromoted-prototype","No runtime adoption authorized")
    sizes=[lane["candidate"]["rows"]["LIFETIME"][0]["size"] for lane in old["lanes"]]
    exact(sizes,[32,32],"Standalone wrapper observation changed")
    return dict(schema="offline-authority-scratch-owner-audit-074-v1",starting_tree="fa9edb88",
        input_packets=packets,input_members_verified=1076,source_pins=PINS,audit=contracts(source_texts(root),sizes[0]),
        decision="do-not-integrate-borrowed-prototype; preregister owned-storage journal model with whole-journal eviction",
        new_measurements=0,runtime_candidate_implemented=False,original_owner_profiles=0,
        original_windows_seconds=old["original_windows_seconds"],first_refusals=old["first_refusals"],qualification_sha256=old["qualification_sha256"],
        original_material_gate_passed=False,full_sigkill_matrix=False,reconnect_proven=False,
        clocks_changed=False,services_changed=False,server=False,product=False)


def require_output(path):
    need(Path(path).resolve()==(ROOT/OUTPUT/"facts.json").resolve(),"Separate owner audit facts required")
    for node in (Path(path),*Path(path).parents):
        need(not node.is_symlink() and not getattr(node,"is_junction",lambda:False)(),"Linked output")


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--verify",action="store_true");parser.add_argument("--packet-sha256");args=parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:
        need(args.verify,"Sealed verification is read-only");previous.core.prior.verify_packet(args.output.parent,args.packet_sha256)
    publish(args.output,analyze(),args.verify)
    print("Owner audit verified: 19 prior seals/1076 members; no new measurements or runtime integration")
