"""Offline source/ownership audit; never opens a live authority or launches a probe."""
import argparse
import hashlib
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/sample-boot-inline-074"))
import review_inline as previous

need, load, sha, exact = previous.need, previous.load, previous.sha, previous.exact
canonical, publish, BASE = previous.canonical, previous.publish, previous.BASE
OUTPUT = BASE / "authority-read-audit-20261022"
INPUTS = dict(previous.INPUTS, **{
    "sample-boot-inline-20261022": "6bee3c2c239d2e38f9ad06a9098e756edc8dc248b6c48eb51f425b61b36b4ebf",
})
FIXTURE = "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs"
REGISTRY = "tools/long-run-supervisor-074/src/diagnostic_authority_registry.rs"
FIXED = "tools/long-run-supervisor-074/src/diagnostic_authority_fixed.rs"
OWNERS = "tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs"
PINS = {
    FIXTURE: "3b3c75f41dbd8a8db1f73d61af3b0c701a666c3ebeed326ae49012ed0c01fb6c",
    REGISTRY: "1e936fe855eedad6d0e5dacbd76d534a45cc8df11cd30f06f8e2ed77258bc53e",
    FIXED: "fd1e10a038ae046888c8ff3c76303e36e5ed49a46aef2025391f8aa009d554a1",
    OWNERS: "c4f04ccb7b3d7bd17e13241b1cefc6657661e5560d1b1ce51f3445e71d5202ce",
}


def source_texts(root):
    texts = {}
    for name, pin in PINS.items():
        text = canonical(Path(root) / name)
        need(hashlib.sha256(text.encode()).hexdigest() == pin, f"Whole authority source drift: {name}")
        texts[name] = text
    return texts


def section(text, begin, end):
    need(text.count(begin) == 1, f"Unique body start required: {begin}")
    tail = text.split(begin, 1)[1]
    need(end in tail, f"Body end required: {end}")
    return begin + tail.split(end, 1)[0]


def ordered(text, expressions):
    position = -1
    for expression in expressions:
        need(text.count(expression) == 1, f"Unique expression required: {expression}")
        next_position = text.index(expression)
        need(next_position > position, f"Expression order changed: {expression}")
        position = next_position


def witness(path, text, expression):
    need(text.count(expression) == 1, "Unique source witness required")
    return dict(path=path, line=text[:text.index(expression)].count("\n") + 1, expression=expression)


def contracts(texts):
    fixture, registry, fixed, owners = (texts[name] for name in (FIXTURE, REGISTRY, FIXED, OWNERS))
    read = section(fixture, "    fn read(\n", "    fn revalidate(\n")
    fresh = section(fixture, "    fn revalidate(\n", "struct Entry {")
    latch = section(fixture, "    fn revalidate(&mut self)", "    /// Reinspect")
    stamp = section(fixture, "struct Stamp {", "struct Leaf {")
    regular = section(fixture, "fn regular(", "fn open_at(")
    opener = section(fixture, "fn open_at(", "fn create_new(")
    read_steps = [
        "let mut file = open_at(dir, name, false)?;",
        "let before = file.metadata()?;",
        "regular(&before, uid, gid, 0o400)?;",
        "if before.dev() != dir.metadata()?.dev() || before.len() > budget as u64 {\n            return Err(AuthorityError::Budget);\n        }",
        "let stamp = Stamp::of(&before);",
        "let mut raw = Vec::with_capacity(before.len() as usize);",
        "(&mut file).take(budget as u64 + 1).read_to_end(&mut raw)?;",
        "if Stamp::of(&file.metadata()?) != stamp || raw.len() as u64 != stamp.len {\n            return Err(AuthorityError::Invalid);\n        }",
        "Ok((file, stamp, raw))",
    ]
    ordered(read, read_steps)
    for expression, count in (("file.metadata()", 2), ("dir.metadata()", 1), ("open_at(", 1),
                              ("Vec::with_capacity(", 1), (".take(", 1), (".read_to_end(", 1)):
        exact(read.count(expression), count, "Read observation count drift")
    ordered(fresh, [
        "if Stamp::of(&self.file.metadata()?) != self.stamp {\n            return Err(AuthorityError::Invalid);\n        }",
        "let named = Self::read(dir, &self.name, uid, gid, budget)?;",
        "if named.1 != self.stamp || named.2 != self.raw {\n            return Err(AuthorityError::Invalid);\n        }",
        "Ok(())",
    ])
    for forbidden in ("drop(", "self.raw =", "self.raw.clear", "self.raw.clone", "read_exact", "set_len"):
        need(forbidden not in read + fresh, "Fresh read ownership/EOF shortcut forbidden")
    for expression in ("!m.is_file()", "m.uid() != uid", "m.gid() != gid", "m.mode() & 0o7777 != mode", "m.nlink() != 1"):
        need(expression in regular, "Regular file predicate missing")
    fields = ["dev", "ino", "uid", "gid", "mode", "links", "len", "mtime", "mtime_ns", "ctime", "ctime_ns"]
    declaration = stamp.split("\n}", 1)[0]
    for field in fields:
        need(f"    {field}:" in declaration, "Stamp field missing")
    for expression in ("dev: m.dev()", "ino: m.ino()", "uid: m.uid()", "gid: m.gid()", "mode: m.mode()",
                       "links: m.nlink()", "len: m.len()", "mtime: m.mtime()", "mtime_ns: m.mtime_nsec()",
                       "ctime: m.ctime()", "ctime_ns: m.ctime_nsec()"):
        need(expression in stamp, "Stamp observation changed")
    for flag in ("O_RDONLY", "O_NOFOLLOW", "O_CLOEXEC", "O_NONBLOCK"):
        need("libc::" + flag in opener, "Read-only/nonblocking opener changed")
    ordered(latch, [
        "if self.failed {\n            return Err(AuthorityError::Refused);\n        }",
        "let result = (|| {", "for entry in &self.entries {",
        "entry.record.revalidate(", "entry.marker.revalidate(",
        "if result.is_err() {\n            self.failed = true;\n        }", "        result\n",
    ])
    exact(latch.count("self.directory.revalidate()?;"), 2, "Directory brackets changed")
    exact(latch.count("self.directory.inventory()? != self.entries.len()"), 2, "Inventory brackets changed")
    for expression in ("pub const MAX_GENERATIONS: usize = 128;", "pub const MAX_RECORD_BYTES: usize = 2048;",
                       "pub const COMMIT_MARKER_BYTES: usize = 65;"):
        need(registry.count(expression) == 1, "Fixed authority budget drift")
    ordered(section(fixture, "impl Entry {", "struct Directory {"), [
        'Leaf::open(dir, name(revision, "json"), uid, gid, MAX_RECORD_BYTES)?;',
        'Leaf::open(dir, name(revision, "commit"), uid, gid, COMMIT_MARKER_BYTES)?;',
        "if marker.raw != marker_bytes(&record.raw)",
    ])
    need("self.journal.inspect(trust, host)" in fixed, "Fixed backend must use the shared journal inspect")
    inspect = section(fixture, "    pub fn inspect(\n", "    pub fn refuse(")
    ordered(inspect, ["self.revalidate()?;", "record.bind(trust, host)?;"])
    need("authority_owners_leaf_read_and_revalidate_match_bytes_stamps_and_refusals" in owners,
         "Existing live differential source missing")
    need("// Keep the fresh File (tuple field zero) alive through both comparisons." in fresh,
         "Original descriptor lifetime contract missing")
    return dict(
        read_body_sha256=hashlib.sha256(read.encode()).hexdigest(),
        revalidate_body_sha256=hashlib.sha256(fresh.encode()).hexdigest(),
        read_steps=[witness(FIXTURE, fixture, expression) for expression in read_steps],
        stamp_fields=fields, fixed_budgets=dict(record=2048, marker=65, generations=128),
        owned_roles=[
            dict(owner="retained-leaf", bytes="Leaf.raw", file="Leaf.file", lifetime="retained original Leaf owner",
                 purpose="expected bytes and original descriptor/stamp for future fresh comparisons", disposition="keep-owned"),
            dict(owner="fresh-revalidation", bytes="named.2", file="named.0", lifetime="through stamp and byte comparisons",
                 purpose="independent newly opened bytes and stamp", disposition="baseline-required-before-transient-buffer-proposal"),
        ],
        explicit_constructor_expressions_per_read=1, measured_allocation_requests=None,
        measured_reallocations=None, measured_wall_ns=None,
        read_limit="budget + 1; post-stamp and exact original length still required",
        caller_refusal="shared journal latches errors; Leaf::read itself is not a sticky registry",
        trace_control_source_only=True, fresh_descriptor_retained_through_comparisons=True,
        proposal="separate transient revalidation scratch from retained Leaf.raw only after its own bounded-read baseline",
        rejected_shortcuts=["reuse cached bytes as fresh evidence", "overwrite expected Leaf.raw during comparison",
                            "read_exact only the old length", "replace budget+1 with old metadata length",
                            "drop the fresh descriptor before comparisons", "remove metadata/ownership or caller latching"],
    )


def analyze(root=ROOT):
    root = Path(root)
    packets = [previous.previous.previous.prior.verify_packet(root / BASE / name, pin) for name, pin in INPUTS.items()]
    exact(sum(row["members"] for row in packets), 928, "Prior member total drift")
    need(canonical(root / "tools/sample-boot-inline-074/review_inline.py") == canonical(root / BASE /
         "sample-boot-inline-20261022/method/tools/sample-boot-inline-074/review_inline.py"), "Previous reader drift")
    old = previous.analyze(root)
    exact(old, load(root / BASE / "sample-boot-inline-20261022/facts.json"), "Previous first comparison changed")
    exact(old["decision"], "reject-for-adoption", "Rejected boot hypothesis must stay closed")
    return dict(schema="offline-authority-read-audit-074-v1", starting_tree="3f48a7c9",
        input_packets=packets, input_members_verified=928, source_pins=PINS, audit=contracts(source_texts(root)),
        original_windows_seconds=old["original_windows_seconds"], first_refusals=old["first_refusals"],
        qualification_sha256=old["qualification_sha256"], previous_boot_decision=old["decision"],
        new_measurements=0, original_owner_profiles=0, runtime_candidate_implemented=False,
        original_material_gate_passed=False, full_sigkill_matrix=False, reconnect_proven=False,
        clocks_changed=False, services_changed=False, server=False, product=False)


def require_output(path):
    need(Path(path).resolve() == (ROOT / OUTPUT / "facts.json").resolve(), "Separate authority audit facts required")
    for node in (Path(path), *Path(path).parents):
        need(not node.is_symlink() and not getattr(node, "is_junction", lambda: False)(), "Linked output")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--packet-sha256")
    args = parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:
        need(args.verify, "Sealed review is read-only")
        previous.previous.previous.prior.verify_packet(args.output.parent, args.packet_sha256)
    publish(args.output, analyze(), args.verify)
    print("Fresh authority ownership audited: 16 prior seals/928 members; no new measurements or runtime candidate")
