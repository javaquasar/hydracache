"""Read-only actual journal integration boundary audit; no test-image execution."""
import argparse
import hashlib
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/"tools/authority-operator-targeted-074"))
import review_targeted as previous
sys.path.insert(0,str(ROOT/"tools/offline-authority-scratch-owner-audit-074"))
import audit_owners as owners
need,exact,load,canonical,publish,BASE=previous.need,previous.exact,previous.load,previous.canonical,previous.publish,previous.BASE
section,ordered,witness=owners.section,owners.ordered,owners.witness
OUTPUT=BASE/"authority-integration-audit-20261022"
INPUTS=dict(previous.INPUTS,**{"authority-operator-targeted-20261022":"bd81081959409bf27b188871aa7493f0a42e50cdea658800f45db7c35229d475"})
PREFIX="tools/long-run-supervisor-074/"
CONTROL=PREFIX+"tests/support/authority_owners_controls.rs"
FIXTURE_TESTS=PREFIX+"src/diagnostic_authority_fixture_tests.rs"
FIXED_TESTS=PREFIX+"src/diagnostic_authority_fixed_tests.rs"
PINS=dict(owners.PINS)
PINS.update({
    "docs/testing/performance/0.74/authority-integration-audit-plan-20261022.md": "f19e7beeb409e8e5f01a38a2e79d71f53d75cd29e42e5426833a0be9f39adbee",
    "crates/xtask/tests/authority_owners_074.rs": "e74849596ae04f15c155d06ff16a60b36d0ad9678268730fcbcb85b1b8e085a8",
    "crates/xtask/tests/authority_owned_journal_074.rs": "6f289745d058a479dd15d1007826c22949b6f0b664a196181864636f8cf83c46",
    "tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs": "c4f04ccb7b3d7bd17e13241b1cefc6657661e5560d1b1ce51f3445e71d5202ce",
    "tools/long-run-supervisor-074/src/diagnostic_authority_fixture_tests.rs": "09af700f81af7c1647ca76dfedf14ce51a18434723c49b9cdf441f7a019a6b4f",
    "crates/xtask/tests/authority_read_baseline_074.rs": "060a59d39bb545cbcc99bd1ea5de3f35b329ceb0c7723b468b0ba5dc2b73d258",
    "crates/xtask/tests/authority_scratch_reuse_074.rs": "1500f6ae469047ffef3905bce1174cb4a29627a6ef1587558f3cb78b51a06798",
    "tools/long-run-supervisor-074/src/diagnostic_authority_fixed_tests.rs": "848a31bc3ff546d66a22ae4920ebd894e192061b9dda11b5482b858c68bf9354",
    "crates/xtask/tests/authority_operator_order_074.rs": "96a035e001afad881c19b24e246871553ce934675f6115c544e1aedb2438249a"
})
# Additional exact source identities are filled below, not discovered from a
# changed tree during review. Existing runtime and model pins remain independent.
PINS.update({
    "crates/xtask/tests/authority_breakdown_074.rs": "5293953c992b8cf7760c4f9fc25b57d9e6f4de7310eb28bfca81e382fe8d855a",
    "crates/xtask/tests/authority_leaf_cpu_074.rs": "d4d6f4aa339e24bd43b1776929b8661ea03dd4a0bf49c22691d85b26ed049f41",
    "crates/xtask/tests/authority_fd_path_074.rs": "8eec87b856ced811211db39a0b0c3b7b321d645e7877d862466395c7f7b198ff",
    "crates/xtask/tests/authority_name_capacity_074.rs": "4419ec9993fda528563e237c5f2688f9c53488a2753d3b210b15400b2e6f3180",
    "crates/xtask/tests/authority_expected_name_074.rs": "f7d483f3607316d5b4e01d4773d4093ef0a07ed39a7e7e04bc35237015221b4f",
    "crates/xtask/tests/authority_inventory_074.rs": "5e3059b971b0a445a9bc3c39a1c9bf65f06a392b47c8c54ef8f36628cedc9b7a",
    "crates/xtask/tests/authority_cpu_074.rs": "1e8e28624175d25dd08939a9b821e64130749dd221ebef6d8a7d95cf6a8c92b7",
    "crates/xtask/tests/authority_open_name_074.rs": "de8949147040b65c3495062a71257c20fa718a67239a4e8230da613de510673b",
    "crates/xtask/tests/authority_leaf_name_074.rs": "3ad157fd1dd31b63e1ce797321ba4fc71d56fed536508ff21ed8c95ec91ecadf"
})
BOUNDARIES={
    "crates/xtask/tests/authority_breakdown_074.rs":["diagnostic_authority_fixture.rs"],
    "crates/xtask/tests/authority_cpu_074.rs":["diagnostic_authority_fixture.rs"],
    "crates/xtask/tests/authority_expected_name_074.rs":["diagnostic_authority_fixture.rs"],
    "crates/xtask/tests/authority_fd_path_074.rs":["diagnostic_authority_fixture.rs"],
    "crates/xtask/tests/authority_inventory_074.rs":["diagnostic_authority_fixture.rs"],
    "crates/xtask/tests/authority_leaf_cpu_074.rs":["diagnostic_authority_fixture.rs"],
    "crates/xtask/tests/authority_leaf_name_074.rs":["diagnostic_authority_fixture.rs"],
    "crates/xtask/tests/authority_name_capacity_074.rs":["diagnostic_authority_fixture.rs"],
    "crates/xtask/tests/authority_open_name_074.rs":["diagnostic_authority_fixture.rs"],
    "crates/xtask/tests/authority_read_baseline_074.rs":["diff","--exit-code","tools/long-run-supervisor-074/src"],
    "crates/xtask/tests/authority_scratch_reuse_074.rs":["diff","--exit-code","tools/long-run-supervisor-074/src"],
    "crates/xtask/tests/authority_owned_journal_074.rs":["diff","--exit-code","tools/long-run-supervisor-074/src"],
    "crates/xtask/tests/authority_operator_order_074.rs":["let change = actual","self.revalidate()?;",".and_then(transition)","diff","--exit-code"],
    "crates/xtask/tests/authority_owners_074.rs":["restored_control(&controls, helper)","implementation(runtime.clone(), owner)","assert_eq!(restored, baseline)"],
}


def source_texts(root):
    texts={}
    for name,pin in PINS.items():
        text=canonical(Path(root)/name)
        exact(hashlib.sha256(text.encode()).hexdigest(),pin,"Integration audit source drift: "+name)
        texts[name]=text
    return texts


def contracts(texts):
    # Reuse the earlier complete-body/fresh-read/ownership source contract, not
    # a loosened transcription. Its wrapper number is a frozen historical input,
    # not a new layout measurement; no budget/layout result is promoted here.
    historical=owners.contracts(texts,32)
    fixture,registry,fixed,signed,trust=(texts[p] for p in (owners.FIXTURE,owners.REGISTRY,owners.FIXED,owners.SIGNED,owners.TRUST))
    init=section(fixture,"fn initialize_directory(\n","pub fn open_fixture_registry(")
    ordered(init,["entries: vec![]","records: vec![]","failed: false","read.directory.inventory()? != 0",
        "read.directory.temporary.sync_all()?;","let record = Record::from_checked(policy)?;","read.publish(record, None)?;"])
    change=section(fixture,"    fn change(\n","    fn publish(\n")
    ordered(change,["self.revalidate()?;","let next = self",".and_then(transition);","Ok(next) => self.publish(next, fault)","Err(error) => {","self.failed = true;"])
    publication=section(fixture,"    fn publish(\n","fn name(")
    ordered(publication,["if self.failed","let bytes = record.bytes()?;","history(&proposed)?;",
        'let mut file = create_new(&self.directory.leaf, &name(revision, "json"))?;',
        "fail(fault, PublicationStage::RecordCreated)?;","write_seal(&mut file, &bytes)?;",
        "fail(fault, PublicationStage::RecordSynced)?;",
        'let mut marker = create_new(&self.directory.leaf, &name(revision, "commit"))?;',
        "fail(fault, PublicationStage::MarkerCreated)?;","write_seal(&mut marker, &marker_bytes(&bytes))?;",
        "fail(fault, PublicationStage::DurableCommit)?;","let entry = Entry::open(",
        "if entry.record.raw != bytes","self.entries.push(entry);","self.records.push(record);"])
    exact(publication.count("self.directory.leaf.sync_all()?;"),3,"Three existing directory sync sites must survive")
    exact(publication.count("self.revalidate()?;"),2,"Publication round count changed")
    need(publication.index("self.revalidate()?;")<publication.index("let bytes = record.bytes()?;"),"Opening publication validation moved")
    need(publication.rindex("self.revalidate()?;")>publication.index("self.records.push(record);"),"Closing publication validation moved")
    for start,end,after in (("let mut file = create_new(","fail(fault, PublicationStage::RecordCreated)","regular("),
                           ("let mut marker = create_new(","fail(fault, PublicationStage::MarkerCreated)","regular("),
                           ("write_seal(&mut marker,","fail(fault, PublicationStage::DurableCommit)","write_seal(")):
        part=section(publication,start,end)
        need(part.index("self.directory.leaf.sync_all()?;")>part.index(after),"Directory sync boundary moved")
    seal=section(fixture,"fn write_seal(","fn write_bytes(")
    ordered(seal,["write_bytes(file, bytes)?;","file.set_permissions(fs::Permissions::from_mode(0o400))?;","file.sync_all()?;","Ok(())"])
    advance=section(registry,"    fn advance(&self,","    fn next_revision(")
    ordered(advance,["let mut next = Self::from_checked(policy)?;","self.state != State::Refused",
        "next.policy_epoch <= self.policy_epoch","next.machine_id != self.machine_id","next.issuer_key_hex != self.issuer_key_hex",
        "next.policy_sha256 == self.policy_sha256","return Err(AuthorityError::Transition);","next.revision = self.next_revision()?;",
        "next.previous_sha256 = Some(sha256_hex(&self.bytes()?));"])
    control=texts[CONTROL]
    for expression in ("    leaf: &Leaf,","for entry in &self.registry.entries {","(&entry.record, MAX_RECORD_BYTES)",
        "(&entry.marker, COMMIT_MARKER_BYTES)","leaf.revalidate(&dir.leaf, dir.uid, dir.gid, budget)?;"):
        need(expression in control,"Original observer receiver boundary lost: "+expression)
    boundary=[]
    for path,expressions in BOUNDARIES.items():
        for expression in expressions:need(expression in texts[path],"Historical/current guard boundary lost: "+path)
        kind="historical-normalization-chain" if len(expressions)==1 else "historical-observer-and-current-token-comparison" if path.endswith("authority_owners_074.rs") else "current-runtime-frozen-identity"
        boundary.append(dict(path=path,kind=kind,sha256=PINS[path]))
    for path,names in ((FIXTURE_TESTS,["authority_fixture_close_reopen_refusal_and_explicit_higher_epoch_are_persistent",
            "authority_fixture_logical_faults_retain_partial_tail_or_committed_refusal_without_replay",
            "authority_fixture_owner_drop_unlocks_with_inherited_description_still_open",
            "authority_fixture_partial_write_errors_preserve_bytes_and_do_not_complete_a_marker"]),
        (FIXED_TESTS,["authority_fixed_modeled_writer_persists_and_exclusively_locks_shared_engine",
            "authority_fixed_modeled_publication_faults_do_not_repair_partial_tail"])):
        for name in names:need("fn "+name+"(" in texts[path],"Existing real-filesystem semantic test disappeared")
    sites=[("external","    pub(crate) fn latch_refusal(","    pub fn is_failed("),
        ("validation","    fn revalidate(&mut self)","    /// Reinspect"),
        ("binding","    pub fn inspect(\n","    pub fn refuse("),
        ("transition","    fn change(\n","    fn publish(\n"),("publication","    fn publish(\n","fn name(")]
    failures=[]
    for name,start,end in sites:
        part=section(fixture,start,end);exact(part.count("self.failed = true;"),1,"Unattributed failure path")
        offset=fixture.index(part)+part.index("self.failed = true;")
        failures.append(dict(site=name,path=owners.FIXTURE,line=fixture[:offset].count("\n")+1,
            required_candidate_action="latch owner and evict every record/marker scratch; retain original entries"))
    exact(fixture.count("self.failed = true;"),5,"Unaccounted journal failure assignment")
    return dict(failure_sites=failures,lifecycle=historical["lifecycle"],guard_boundaries=boundary,
        observer=dict(path=CONTROL,receiver="&Leaf",entry_loop="immutable",current_candidate_equivalence=False),
        publication=dict(checkpoints=["RecordCreated","RecordSynced","MarkerCreated","DurableCommit"],
            directory_sync_sites=3,seal_calls=2,seal_order=["write_all","permissions_0400","file_sync_all"],entry_after_durable_commit=True),
        patch_scope=["Leaf optional owned scratch and cold constructor; no self borrow or interior mutability",
            "fresh bounded read storage substitution only; retain original File/stamp/raw and fresh File lifetime",
            "explicit mutable entry iteration with record-before-marker and unchanged outer brackets",
            "centralize five failure paths and evict all temporary buffers, including external latches",
            "keep actual Record transitions and publication IO/sync/checkpoint order unchanged",
            "version observer separately and distinguish frozen historical replay from current candidate guards"],
        blockers=["exact runtime patch not yet implemented or compiled","historical replay/current candidate guard separation not implemented",
            "real filesystem cache eviction and publication fault semantics not yet exercised on candidate",
            "prechecked model does not cover real Record policy/key/host/chain validation"],
        source_witnesses=[witness(owners.FIXTURE,fixture,"        self.failed = true;\n    }"),
            witness(owners.FIXTURE,fixture,"            fail(fault, PublicationStage::DurableCommit)?;"),
            witness(owners.REGISTRY,registry,"            || next.issuer_key_hex != self.issuer_key_hex"),
            witness(CONTROL,control,"    leaf: &Leaf,")])


def analyze(root=ROOT):
    root=Path(root)
    packets=[previous.core.prior.verify_packet(root/BASE/name,pin) for name,pin in INPUTS.items()]
    exact(sum(p["members"] for p in packets),1363,"Prior history drift")
    old=previous.analyze(root);exact(old,load(root/previous.OUTPUT/"facts.json"),"Clean targeted evidence changed")
    exact(old["targeted_isolation_gate_passed"],True,"Model isolation prerequisite is red")
    exact(old["prior_targeted_isolation_gate_passed"],False,"Do not promote earlier broad cohort")
    return dict(schema="offline-authority-integration-audit-074-v1",starting_tree="405fd026",
        input_packets=packets,input_members_verified=1363,source_pins=PINS,audit=contracts(source_texts(root)),
        decision="prepare exact owned-storage patch with separate historical and current guards; no runtime admission",
        targeted_model_gate=True,prior_broad_isolation_gate=False,new_measurements=0,model_images_executed=0,
        runtime_candidate_implemented=False,original_owner_profiles=0,original_windows_seconds=old["original_windows_seconds"],
        first_refusals=old["first_refusals"],qualification_sha256=old["qualification_sha256"],original_material_gate_passed=False,
        full_sigkill_matrix=False,reconnect_proven=False,clocks_changed=False,services_changed=False,server=False,product=False)


def require_output(path):
    need(Path(path).resolve()==(ROOT/OUTPUT/"facts.json").resolve(),"Separate integration audit output required")
    for node in (Path(path),*Path(path).parents):need(not node.is_symlink() and not getattr(node,"is_junction",lambda:False)(),"Linked output")


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--verify",action="store_true");parser.add_argument("--packet-sha256");args=parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:need(args.verify,"Sealed audit is read-only");previous.core.prior.verify_packet(args.output.parent,args.packet_sha256)
    publish(args.output,analyze(),args.verify)
    print("Actual integration boundary verified: five refusal sites; runtime and real-fault gates remain closed")
