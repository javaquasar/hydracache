//! Fixed patch allowlist only; not a semantic checker or a Linux admission gate.
use proc_macro2::TokenStream;

pub const SOURCE: &str = "33c54b0cda8c330b115009cdf092ad002077cedf";
pub const FIXTURE: &str = "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs";
pub const OBSERVER: &str =
    "tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs";

fn replace(source: &str, old: &str, new: &str, count: usize) -> String {
    assert_eq!(
        source.matches(old).count(),
        count,
        "fixed patch occurrence: {old}"
    );
    source.replace(old, new)
}
fn section<'a>(source: &'a str, begin: &str, end: &str) -> &'a str {
    assert_eq!(source.matches(begin).count(), 1, "unique section start");
    let start = source.find(begin).unwrap();
    let rest = &source[start..];
    assert_eq!(rest.matches(end).count(), 1, "unique section end");
    &rest[..rest.find(end).unwrap()]
}

const SELECT: &str = "match slot.take() {
            Some(mut raw) if raw.capacity() == before.len() as usize => {
                raw.clear();
                raw
            }
            _ => Vec::with_capacity(before.len() as usize),
        }";
const RETAIN: &str =
    "        if named.2.capacity() == named.1.len as usize && named.2.capacity() <= budget {
            self.scratch = Some(named.2);
        }
";
const WRAPPER: &str = "        let result = self.validate(dir, uid, gid, budget);
        if result.is_err() {
            self.evict_scratch();
        }
        result
    }
";
const EVICT: &str = "    fn evict_scratch(&mut self) {
        self.scratch = None;
    }
";
const JOURNAL_EVICT: &str = "    pub(crate) fn latch_refusal(&mut self) {
        self.failed = true;
        self.evict_scratch();
    }
    fn evict_scratch(&mut self) {
        for entry in &mut self.entries {
            entry.record.evict_scratch();
            entry.marker.evict_scratch();
        }
    }
";

pub fn candidate(baseline: &str) -> String {
    let read = section(baseline, "    fn read(\n", "    fn revalidate(\n");
    let original = section(
        baseline,
        "    fn revalidate(\n        &self,",
        "\n}\nstruct Entry",
    );
    let mut reusable = replace(read, "fn read(", "fn read_reusing(", 1);
    reusable = replace(
        &reusable,
        "        budget: usize,",
        "        budget: usize,\n        slot: &mut Option<Vec<u8>>,",
        1,
    );
    reusable = replace(
        &reusable,
        "Vec::with_capacity(before.len() as usize)",
        SELECT,
        1,
    );
    let mut validate = replace(original, "fn revalidate(", "fn validate(", 1);
    validate = replace(&validate, "        &self,", "        &mut self,", 1);
    validate = replace(
        &validate,
        "Self::read(dir, &self.name, uid, gid, budget)?",
        "Self::read_reusing(dir, &self.name, uid, gid, budget, &mut self.scratch)?",
        1,
    );
    validate = replace(
        &validate,
        "        Ok(())",
        &format!("{RETAIN}        Ok(())"),
        1,
    );
    let signature = original.split("        if Stamp").next().unwrap();
    let signature = replace(signature, "        &self,", "        &mut self,", 1);
    let mut output = replace(
        baseline,
        &format!("{read}{original}"),
        &format!("{read}{reusable}{signature}{WRAPPER}{validate}\n{EVICT}"),
        1,
    );
    output = replace(
        &output,
        "    name: String,\n}",
        "    name: String,\n    scratch: Option<Vec<u8>>,\n}",
        1,
    );
    output = replace(
        &output,
        "            raw,\n            name,",
        "            raw,\n            name,\n            scratch: None,",
        1,
    );
    output = replace(
        &output,
        "for entry in &self.entries {",
        "for entry in &mut self.entries {",
        1,
    );
    output = replace(&output, "self.failed = true;", "self.latch_refusal();", 5);
    replace(
        &output,
        "    pub(crate) fn latch_refusal(&mut self) {\n        self.latch_refusal();\n    }\n",
        JOURNAL_EVICT,
        1,
    )
}

pub fn observer(baseline: &str) -> String {
    let read = section(
        baseline,
        "fn leaf_read<O: Observer>(",
        "fn leaf_revalidate<O: Observer>(",
    );
    let original = section(
        baseline,
        "fn leaf_revalidate<O: Observer>(",
        "#[derive(Clone, Copy)]\npub(crate) enum Phase",
    );
    let mut reusable = replace(read, "fn leaf_read<", "fn leaf_read_reusing<", 1);
    reusable = replace(
        &reusable,
        "    budget: usize,",
        "    budget: usize,\n    slot: &mut Option<Vec<u8>>,",
        1,
    );
    reusable = replace(
        &reusable,
        "Vec::with_capacity(before.len() as usize)",
        SELECT,
        1,
    );
    let mut validate = replace(original, "fn leaf_revalidate<", "fn leaf_validate<", 1);
    validate = replace(&validate, "    leaf: &Leaf,", "    leaf: &mut Leaf,", 1);
    validate = replace(
        &validate,
        "leaf_read(observer, dir, &leaf.name, uid, gid, budget)?",
        "leaf_read_reusing(observer, dir, &leaf.name, uid, gid, budget, &mut leaf.scratch,)?",
        1,
    );
    validate = replace(
        &validate,
        "    Ok(())",
        &format!("{}    Ok(())", RETAIN.replace("self.", "leaf.")),
        1,
    );
    let signature = original.split("    if Stamp").next().unwrap();
    let signature = replace(signature, "    leaf: &Leaf,", "    leaf: &mut Leaf,", 1);
    let wrapper = WRAPPER
        .replace(
            "self.validate(dir, uid, gid, budget)",
            "leaf_validate(observer, leaf, dir, uid, gid, budget)",
        )
        .replace("self.evict_scratch()", "leaf.evict_scratch()");
    let mut output = replace(
        baseline,
        &format!("{read}{original}"),
        &format!("{read}{reusable}{signature}{wrapper}\n{validate}"),
        1,
    );
    output = replace(
        &output,
        "for entry in &self.registry.entries {",
        "for entry in &mut self.registry.entries {",
        1,
    );
    output = replace(
        &output,
        "(&entry.record, MAX_RECORD_BYTES)",
        "(&mut entry.record, MAX_RECORD_BYTES)",
        1,
    );
    output = replace(
        &output,
        "(&entry.marker, COMMIT_MARKER_BYTES)",
        "(&mut entry.marker, COMMIT_MARKER_BYTES)",
        1,
    );
    output = replace(
        &output,
        "let leaf = Leaf::open",
        "let mut leaf = Leaf::open",
        1,
    );
    replace(
        &output,
        "                &leaf,",
        "                &mut leaf,",
        1,
    )
}

pub fn accepts(expected: &str, actual: &str) -> bool {
    syn::parse_file(actual).is_ok()
        && expected.parse::<TokenStream>().unwrap().to_string()
            == actual
                .parse::<TokenStream>()
                .map(|tokens| tokens.to_string())
                .unwrap_or_default()
}
