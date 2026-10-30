//! Source-bound, platform-scoped line coverage. The historical global ratchet is unchanged.
use crate::imap_extended_receipt_075::{artifact, Artifact};
use crate::imap_qualification_075::{digest, hex, read_relative};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use syn::{spanned::Spanned, visit::Visit};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub const BASELINE: &str = "98d7321b1cda568c386cbfdef1993c7e4dcebe38";
const MAX: usize = 8 * 1024 * 1024;
const MAX_RAW: usize = 128 * 1024 * 1024;
const MAX_RAW_TOTAL: u64 = 6 * 1024 * 1024 * 1024;
const MAX_OBJECTS: usize = 512;
pub const CARGO_CONFIG: &[&str] = &[
    "profile.dev.opt-level=0",
    "profile.test.opt-level=0",
    r#"profile.dev.package."sha2@0.11.0".opt-level=3"#,
    r#"profile.test.package."sha2@0.11.0".opt-level=3"#,
];
pub fn object_total(total: u64, bytes: u64) -> Result<u64> {
    let total = total
        .checked_add(bytes)
        .ok_or("compiler archive byte overflow")?;
    if bytes == 0 || bytes > 512 * 1024 * 1024 || total > 8 * 1024 * 1024 * 1024 {
        return Err("compiler archive byte bound".into());
    }
    Ok(total)
}
pub fn object_count(count: usize) -> Result<()> {
    if count > MAX_OBJECTS {
        return Err("compiler archive object count bound".into());
    }
    Ok(())
}
const SCOPES: &[&str] = &[
    "crates/xtask/src/",
    "crates/hydracache-cluster-testkit/src/",
    "tests/java-imap-benchmark/src/main/java/",
    "tests/java-imap-hazelcast-075/src/main/java/",
    "tests/java-imap-provenance-075/src/main/java/",
    "tests/rust-imap-contract-consumer-075/src/",
];
pub type Lines = BTreeMap<String, BTreeMap<u32, u64>>;
/// Snapshot counters without resetting them, then atomically publish the complete buffer.
pub fn flush_profile() -> Result<()> {
    #[cfg(feature = "imap-075-coverage")]
    {
        unsafe extern "C" {
            fn __llvm_profile_get_size_for_buffer() -> u64;
            fn __llvm_profile_write_buffer(buffer: *mut std::ffi::c_char) -> std::ffi::c_int;
        }
        // SAFETY: LLVM reports the exact size required by write_buffer for this executable.
        let size = usize::try_from(unsafe { __llvm_profile_get_size_for_buffer() })?;
        if size == 0 || size > MAX_RAW {
            return Err("LLVM checkpoint buffer byte bound".into());
        }
        let mut bytes = vec![0u8; size];
        // SAFETY: bytes owns the full reported writable allocation. Only this worker
        // thread performs snapshots, and the runtime neither retains nor frees it.
        if unsafe { __llvm_profile_write_buffer(bytes.as_mut_ptr().cast()) } != 0 {
            return Err("LLVM buffer serialization failed".into());
        }
        let pattern = PathBuf::from(
            std::env::var_os("LLVM_PROFILE_FILE")
                .ok_or("explicit LLVM checkpoint path required")?,
        );
        if !pattern.is_absolute()
            || pattern.file_name().and_then(|s| s.to_str()) != Some("%p.profraw")
        {
            return Err("fixed PID-specific LLVM checkpoint pattern required".into());
        }
        persist_profile(
            pattern.parent().ok_or("LLVM profile parent")?,
            std::process::id(),
            &bytes,
        )?;
    }
    Ok(())
}
/// Publication only; the independent LLVM reader still validates the actual profile bytes.
pub fn persist_profile(directory: &Path, pid: u32, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    if !directory.is_absolute() || pid == 0 || bytes.is_empty() || bytes.len() > MAX_RAW {
        return Err("profile publication path/PID/byte bound".into());
    }
    let staging = directory
        .parent()
        .ok_or("profile staging parent")?
        .join("profile-staging");
    fs::create_dir_all(&staging)?;
    let mut file = tempfile::Builder::new()
        .prefix("partial-")
        .tempfile_in(staging)?;
    file.write_all(bytes)?;
    file.flush()?;
    file.persist(directory.join(format!("{pid}.profraw")))?;
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    pub added: BTreeSet<u32>,
    pub has_executable: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CfgRange {
    pub source: String,
    pub first: u32,
    pub last: u32,
    pub predicate: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceResult {
    pub source: String,
    pub disposition: String,
    pub executable_added: usize,
    pub covered_added: usize,
    pub uncovered: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub sources: Vec<SourceResult>,
    pub executable_added: usize,
    pub covered_added: usize,
    pub compiled_out: Vec<CfgRange>,
}

/// Git emits exact added ranges; comments affect source hashes, never the executable denominator.
pub fn added_lines(diff: &[u8]) -> Result<BTreeMap<String, BTreeSet<u32>>> {
    if diff.len() > MAX {
        return Err("coverage diff byte bound".into());
    }
    let diff = std::str::from_utf8(diff)?;
    let mut result: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    let mut current = None;
    let mut work = 0;
    let mut line_work = 0;
    for line in diff.lines() {
        work += 1;
        if work > 200000 {
            return Err("coverage diff work bound".into());
        }
        if let Some(path) = line.strip_prefix("+++ b/") {
            if path.contains('\\')
                || path.contains(':')
                || path.starts_with('"')
                || path.chars().any(char::is_control)
                || Path::new(path)
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err("foreign diff source path".into());
            }
            current = Some(path.to_owned());
            result.entry(path.into()).or_default();
        } else if line == "+++ /dev/null" {
            current = None;
        } else if line.starts_with("@@ ") {
            let field = line
                .split_whitespace()
                .nth(2)
                .ok_or("coverage diff added range missing")?;
            let field = field
                .strip_prefix('+')
                .ok_or("coverage diff added range malformed")?;
            let (first, count) = field.split_once(',').unwrap_or((field, "1"));
            let first: u32 = first.parse()?;
            let count: u32 = count.parse()?;
            if count > 100000 || first.checked_add(count).is_none() {
                return Err("coverage diff line/work bound".into());
            }
            line_work += count as usize;
            if line_work > 1000000 {
                return Err("coverage aggregate changed-line bound".into());
            }
            if let Some(path) = &current {
                let lines = result.get_mut(path).ok_or("coverage diff source state")?;
                for nr in first..first + count {
                    if nr == 0 {
                        return Err("coverage line zero".into());
                    }
                    lines.insert(nr);
                }
            }
        }
    }
    Ok(result)
}
fn scoped(path: &str) -> bool {
    SCOPES.iter().any(|scope| path.starts_with(scope))
        && (path.ends_with(".rs") || path.ends_with(".java"))
}
fn executable_source(path: &str, bytes: &[u8]) -> Result<bool> {
    let text = std::str::from_utf8(bytes)?;
    if path.ends_with(".java") {
        // Type bodies produce source-file entries even if all record accessors are filtered.
        // A comment-only compilation unit has no runtime code.
        let mut block = false;
        let mut quote = None;
        let mut escape = false;
        let mut line = false;
        let data = text.as_bytes();
        let mut i = 0;
        while i < data.len() {
            let c = data[i];
            let next = data.get(i + 1).copied();
            if line {
                if c == b'\n' {
                    line = false;
                }
            } else if block {
                if c == b'*' && next == Some(b'/') {
                    block = false;
                    i += 1;
                }
            } else if let Some(q) = quote {
                if escape {
                    escape = false;
                } else if c == b'\\' {
                    escape = true;
                } else if c == q {
                    quote = None;
                }
            } else if c == b'/' && next == Some(b'/') {
                line = true;
                i += 1;
            } else if c == b'/' && next == Some(b'*') {
                block = true;
                i += 1;
            } else if c == b'"' || c == b'\'' {
                quote = Some(c);
            } else if c == b'{' {
                return Ok(true);
            }
            i += 1;
        }
        return Ok(false);
    }
    struct Functions(bool);
    impl<'a> Visit<'a> for Functions {
        fn visit_item_fn(&mut self, _: &'a syn::ItemFn) {
            self.0 = true;
        }
        fn visit_impl_item_fn(&mut self, _: &'a syn::ImplItemFn) {
            self.0 = true;
        }
        fn visit_trait_item_fn(&mut self, node: &'a syn::TraitItemFn) {
            self.0 |= node.default.is_some();
        }
        fn visit_expr_closure(&mut self, _: &'a syn::ExprClosure) {
            self.0 = true;
        }
    }
    let mut functions = Functions(false);
    functions.visit_file(&syn::parse_file(text)?);
    Ok(functions.0)
}
pub fn source_snapshot(root: &Path) -> Result<Vec<Source>> {
    source_snapshot_since(root, BASELINE)
}
/// The production capture always supplies BASELINE; fixtures use an isolated real Git history.
pub fn source_snapshot_since(root: &Path, baseline: &str) -> Result<Vec<Source>> {
    if !hex(baseline, 40) {
        return Err("coverage baseline commit identity required".into());
    }
    fn git(root: &Path, args: &[&str], paths: &[String]) -> Result<Vec<u8>> {
        let mut command = Command::new("git");
        command.current_dir(root).args(args);
        if !paths.is_empty() {
            command.arg("--").args(paths);
        }
        let result = command.output()?;
        if !result.status.success() || result.stdout.len() > MAX {
            return Err(format!(
                "coverage scoped Git input failed/bounded: status {}, {} output bytes; {}",
                result.status,
                result.stdout.len(),
                String::from_utf8_lossy(&result.stderr)
                    .chars()
                    .take(512)
                    .collect::<String>()
            )
            .into());
        }
        Ok(result.stdout)
    }
    let patterns = SCOPES
        .iter()
        .flat_map(|scope| {
            [
                format!(":(glob){scope}**/*.rs"),
                format!(":(glob){scope}**/*.java"),
            ]
        })
        .collect::<Vec<_>>();
    let parents = git(root, &["rev-list", "--parents", "-n", "1", "HEAD"], &[])?;
    let parents = std::str::from_utf8(&parents)?
        .split_whitespace()
        .collect::<Vec<_>>();
    let mut own_tip = "HEAD";
    if parents.len() == 3 {
        let first = git(root, &["merge-base", baseline, parents[1]], &[])?;
        let second = git(root, &["merge-base", baseline, parents[2]], &[])?;
        let first_owned = std::str::from_utf8(&first)?.trim() == baseline;
        let second_owned = std::str::from_utf8(&second)?.trim() == baseline;
        if first_owned && !second_owned {
            own_tip = parents[1];
        } else if second_owned && !first_owned {
            own_tip = parents[2];
        }
    }
    // A PR merge may contain a much newer 0.74 tree. Select 0.75's changed files
    // before requesting patches, while still binding every final tracked source.
    // Changed lines in shared files are conservatively retained in full.
    let names = git(
        root,
        &[
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-renames",
            "--name-only",
            "-z",
            baseline,
            own_tip,
        ],
        &patterns,
    )?;
    let paths = std::str::from_utf8(&names)?
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(|p| format!(":(literal){p}"))
        .collect::<Vec<_>>();
    if paths.len() > 2048 {
        return Err("coverage changed-source work bound".into());
    }
    let changed = if paths.is_empty() {
        BTreeMap::new()
    } else {
        added_lines(&git(
            root,
            &[
                "-c",
                "core.quotepath=false",
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--no-renames",
                "--unified=0",
                baseline,
                "HEAD",
            ],
            &paths,
        )?)?
    };
    let mut result = Vec::new();
    for path in std::str::from_utf8(&git(
        root,
        &["-c", "core.quotepath=false", "ls-files", "-z"],
        &patterns,
    )?)?
    .split('\0')
    .filter(|p| !p.is_empty())
    {
        if !scoped(path) {
            continue;
        }
        if result.len() >= 2048 {
            return Err("coverage source work bound".into());
        }
        let bytes = read_relative(root, path, MAX)?;
        result.push(Source {
            path: path.into(),
            sha256: digest(&bytes),
            bytes: bytes.len() as u64,
            added: changed.get(path).cloned().unwrap_or_default(),
            has_executable: executable_source(path, &bytes)?,
        });
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    if result.is_empty()
        || !result
            .iter()
            .any(|s| s.path.starts_with("crates/xtask/src/") && !s.added.is_empty())
    {
        return Err("coverage scope omits changed xtask sources".into());
    }
    Ok(result)
}

/// Normalizes only spelling, never traversal or a foreign root/basename.
pub fn normalize_source(root: &str, path: &str, known: &BTreeSet<String>) -> Result<String> {
    fn spelling(path: &str) -> Result<String> {
        if path.is_empty() || path.len() > 4096 || path.chars().any(char::is_control) {
            return Err("coverage path byte/control bound".into());
        }
        let path = path.replace('\\', "/");
        let path = path.strip_prefix("//?/").unwrap_or(&path).to_owned();
        let path = if let Some(unc) = path.strip_prefix("UNC/") {
            format!("//{unc}")
        } else {
            path
        };
        if path.split('/').any(|p| p == ".." || p == ".") {
            return Err("coverage traversal refused".into());
        }
        Ok(path.trim_end_matches('/').to_owned())
    }
    let root = spelling(root)?;
    let path = spelling(path)?;
    let windows = root.as_bytes().get(1) == Some(&b':') || root.starts_with("//");
    let absolute = path.starts_with('/') || path.as_bytes().get(1) == Some(&b':');
    let candidate = if absolute {
        let prefix = format!("{root}/");
        if path.len() <= prefix.len()
            || !path.get(..prefix.len()).is_some_and(|actual| {
                if windows {
                    actual.eq_ignore_ascii_case(&prefix)
                } else {
                    actual == prefix
                }
            })
        {
            return Err("foreign absolute coverage source".into());
        }
        &path[prefix.len()..]
    } else {
        path.as_str()
    };
    if candidate.contains(':')
        || candidate.starts_with('/')
        || candidate.split('/').any(str::is_empty)
    {
        return Err("foreign coverage source spelling".into());
    }
    if known.contains(candidate) {
        return Ok(candidate.into());
    }
    if windows {
        let matches = known
            .iter()
            .filter(|p| p.eq_ignore_ascii_case(candidate))
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            return Ok(matches[0].clone());
        }
    }
    Err("coverage source outside the declared tracked scope".into())
}
/// LLVM's LCOV line entries are authority for executability; branch coverage is not inferred.
pub fn parse_lcov(bytes: &[u8], root: &str, known: &BTreeSet<String>) -> Result<Lines> {
    if bytes.is_empty() || bytes.len() > 32 * 1024 * 1024 {
        return Err("LLVM LCOV byte/empty bound".into());
    }
    let mut result: Lines = BTreeMap::new();
    let mut current = None;
    let mut lines = BTreeMap::new();
    let mut work = 0;
    for line in std::str::from_utf8(bytes)?.lines() {
        work += 1;
        if work > 1000000 {
            return Err("LLVM LCOV work bound".into());
        }
        if let Some(path) = line.strip_prefix("SF:") {
            if current.is_some() {
                return Err("nested LCOV source record".into());
            }
            current = Some(normalize_source(root, path, known)?);
        } else if let Some(value) = line.strip_prefix("DA:") {
            if current.is_none() {
                return Err("LCOV line without source".into());
            }
            let mut fields = value.split(',');
            let nr: u32 = fields.next().ok_or("LCOV line number")?.parse()?;
            let count: u64 = fields.next().ok_or("LCOV line count")?.parse()?;
            if nr == 0 || nr > 1000000 || lines.insert(nr, count).is_some() {
                return Err("duplicate/invalid LLVM line".into());
            }
        } else if line == "end_of_record" {
            let source = current.take().ok_or("LCOV terminal without source")?;
            let target = result.entry(source).or_default();
            for (nr, count) in std::mem::take(&mut lines) {
                let total = target.entry(nr).or_insert(0);
                *total = (*total)
                    .checked_add(count)
                    .ok_or("LLVM coverage count overflow")?;
            }
        } else if !(line.is_empty()
            || [
                "TN:", "FN:", "FNDA:", "FNF:", "FNH:", "LF:", "LH:", "BRDA:", "BRF:", "BRH:",
            ]
            .iter()
            .any(|prefix| line.starts_with(prefix)))
        {
            return Err("unsupported LCOV record".into());
        }
    }
    if current.is_some() || result.is_empty() {
        return Err("incomplete LLVM LCOV source records".into());
    }
    Ok(result)
}
pub fn java_lines(
    value: &Value,
    module: &str,
    expected_session: &str,
    root: &str,
    known: &BTreeSet<String>,
) -> Result<Lines> {
    if value["schema"] != "imap075-java-coverage-v1" {
        return Err("future/unknown Java coverage projection".into());
    }
    if ![
        "java-imap-benchmark",
        "java-imap-hazelcast-075",
        "java-imap-provenance-075",
    ]
    .contains(&module)
    {
        return Err("foreign Java module".into());
    }
    let sessions = value["sessions"]
        .as_array()
        .ok_or("Java coverage session list missing")?;
    if sessions.len() != 1 || sessions[0]["id"] != expected_session || expected_session.is_empty() {
        return Err("missing/stale/merged Java coverage sessions".into());
    }
    let sources = value["sources"]
        .as_object()
        .filter(|s| !s.is_empty() && s.len() <= 512)
        .ok_or("Java coverage sources empty/bounded")?;
    let mut result = Lines::new();
    for (path, entries) in sources {
        let path = normalize_source(root, &format!("tests/{module}/src/main/java/{path}"), known)?;
        let entries = entries
            .as_object()
            .filter(|e| e.len() <= 100000)
            .ok_or("Java line work bound")?;
        let mut lines = BTreeMap::new();
        for (line, count) in entries {
            let nr: u32 = line.parse()?;
            let count = count.as_u64().ok_or("Java count missing")?;
            if nr == 0 || nr > 1000000 || lines.insert(nr, count).is_some() {
                return Err("invalid Java coverage line".into());
            }
        }
        if result.insert(path, lines).is_some() {
            return Err("duplicate Java coverage source".into());
        }
    }
    Ok(result)
}
pub fn summarize(
    sources: &[Source],
    lines: &Lines,
    os: &str,
    compiled_out: Vec<CfgRange>,
) -> Result<Summary> {
    if !["windows", "linux"].contains(&os) || sources.is_empty() || sources.len() > 2048 {
        return Err("coverage platform/source profile unsupported".into());
    }
    let mut known = BTreeSet::new();
    for source in sources {
        if !known.insert(source.path.clone())
            || !hex(&source.sha256, 64)
            || source.bytes > MAX as u64
        {
            return Err("duplicate/invalid coverage source binding".into());
        }
    }
    if lines.keys().any(|key| !known.contains(key)) {
        return Err("foreign source component in coverage".into());
    }
    let mut results = Vec::new();
    let mut executable_added = 0;
    let mut covered_added = 0;
    for source in sources.iter().filter(|s| !s.added.is_empty()) {
        let other_platform = match source.path.as_str() {
            "crates/xtask/src/imap_proof_process_windows_075.rs" => os != "windows",
            "crates/xtask/src/imap_proof_process_linux_075.rs" => os != "linux",
            _ => false,
        };
        let mut result = SourceResult {
            source: source.path.clone(),
            disposition: "compiled".into(),
            executable_added: 0,
            covered_added: 0,
            uncovered: vec![],
        };
        if other_platform {
            if lines.contains_key(&source.path) {
                return Err("compiled-out platform source present in coverage".into());
            }
            result.disposition = "compiled_out_platform".into();
        } else if let Some(counts) = lines.get(&source.path) {
            if counts.is_empty() && source.has_executable && source.path.ends_with(".rs") {
                return Err("empty executable Rust source report".into());
            }
            for nr in &source.added {
                if let Some(count) = counts.get(nr) {
                    result.executable_added += 1;
                    if *count > 0 {
                        result.covered_added += 1;
                    } else {
                        result.uncovered.push(*nr);
                    }
                }
            }
        } else if source.has_executable {
            return Err(format!("missing compiled source component: {}", source.path).into());
        } else {
            result.disposition = "no_runtime_executable_items".into();
        }
        executable_added += result.executable_added;
        covered_added += result.covered_added;
        results.push(result);
    }
    if executable_added == 0 {
        return Err("coverage executable changed-line set is empty".into());
    }
    Ok(Summary {
        sources: results,
        executable_added,
        covered_added,
        compiled_out,
    })
}
pub fn require_complete(summary: &Summary) -> Result<()> {
    if summary.executable_added == 0
        || summary.executable_added != summary.covered_added
        || summary.sources.iter().any(|s| !s.uncovered.is_empty())
    {
        let missing = summary
            .sources
            .iter()
            .flat_map(|s| s.uncovered.iter().map(|n| format!("{}:{n}", s.source)))
            .take(24)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("uncovered changed executable lines: {missing}").into());
    }
    Ok(())
}

/// False cfg bodies are disclosed separately; the LLVM line denominator remains untouched.
pub fn compiled_out_ranges(
    root: &Path,
    sources: &[Source],
    contexts: &BTreeMap<String, Vec<crate::imap_sbom_075::CargoTarget>>,
) -> Result<Vec<CfgRange>> {
    struct Visitor<'a> {
        source: &'a Source,
        contexts: &'a [crate::imap_sbom_075::CargoTarget],
        ranges: Vec<CfgRange>,
        failure: Option<String>,
    }
    impl Visitor<'_> {
        fn record(&mut self, attrs: &[syn::Attribute], span: proc_macro2::Span) -> bool {
            for attr in attrs.iter().filter(|a| a.path().is_ident("cfg")) {
                let predicate = format!(
                    "cfg({})",
                    match &attr.meta {
                        syn::Meta::List(list) => list.tokens.to_string(),
                        _ => {
                            self.failure = Some("invalid cfg attribute".into());
                            return false;
                        }
                    }
                );
                let mut any = false;
                for context in self.contexts {
                    match context.applies(&predicate) {
                        Ok(value) => any |= value,
                        Err(e) => {
                            self.failure = Some(e.to_string());
                            return false;
                        }
                    }
                }
                if !any {
                    let first = span.start().line as u32;
                    let last = span.end().line as u32;
                    if first == 0 || last < first {
                        self.failure = Some("cfg source span locations unavailable".into());
                        return false;
                    }
                    if self.source.added.range(first..=last).next().is_some() {
                        self.ranges.push(CfgRange {
                            source: self.source.path.clone(),
                            first,
                            last,
                            predicate,
                        });
                    }
                    return false;
                }
            }
            true
        }
    }
    impl<'a> Visit<'a> for Visitor<'_> {
        fn visit_item_fn(&mut self, node: &'a syn::ItemFn) {
            if self.record(&node.attrs, node.span()) {
                syn::visit::visit_item_fn(self, node);
            }
        }
        fn visit_impl_item_fn(&mut self, node: &'a syn::ImplItemFn) {
            if self.record(&node.attrs, node.span()) {
                syn::visit::visit_impl_item_fn(self, node);
            }
        }
        fn visit_trait_item_fn(&mut self, node: &'a syn::TraitItemFn) {
            if self.record(&node.attrs, node.span()) {
                syn::visit::visit_trait_item_fn(self, node);
            }
        }
        fn visit_item_mod(&mut self, node: &'a syn::ItemMod) {
            if self.record(&node.attrs, node.span()) {
                syn::visit::visit_item_mod(self, node);
            }
        }
        fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
            if self.record(&node.attrs, node.span()) {
                syn::visit::visit_item_impl(self, node);
            }
        }
        fn visit_item_foreign_mod(&mut self, node: &'a syn::ItemForeignMod) {
            if self.record(&node.attrs, node.span()) {
                syn::visit::visit_item_foreign_mod(self, node);
            }
        }
        fn visit_expr_block(&mut self, node: &'a syn::ExprBlock) {
            if self.record(&node.attrs, node.span()) {
                syn::visit::visit_expr_block(self, node);
            }
        }
    }
    let mut result = Vec::new();
    for source in sources
        .iter()
        .filter(|s| s.path.ends_with(".rs") && !s.added.is_empty())
    {
        let contexts = contexts
            .get(&source.path)
            .ok_or("source cfg compilation contexts missing")?;
        let mut visitor = Visitor {
            source,
            contexts,
            ranges: vec![],
            failure: None,
        };
        visitor.visit_file(&syn::parse_file(std::str::from_utf8(&read_relative(
            root,
            &source.path,
            MAX,
        )?)?)?);
        if let Some(error) = visitor.failure {
            return Err(error.into());
        }
        result.extend(visitor.ranges);
    }
    Ok(result)
}

const RUNNER_TARGET: &str = "target/imap-075/coverage-runner";
const RUST_FLAGS: &str = "-C instrument-coverage -C debuginfo=0";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub name: String,
    pub sha256: String,
    pub bytes: u64,
    pub version: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: String,
    pub baseline: String,
    pub os: String,
    pub arch: String,
    pub rust_flags: String,
    pub cargo_config: Vec<String>,
    pub feature: String,
    pub target: crate::imap_sbom_075::CargoTarget,
    pub tools: Vec<Tool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    pub schema: String,
    pub source_sha: String,
    pub run_id: String,
    pub production_enabled: bool,
    pub profile: Profile,
    pub sources: Vec<Source>,
    pub producer: Artifact,
    pub bootstrap: CompilerRecord,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledObject {
    pub executable: String,
    pub archived: Artifact,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerRecord {
    pub stream: Artifact,
    pub objects: Vec<CompiledObject>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JavaSession {
    pub schema: String,
    pub module: String,
    pub id: String,
    pub source_sha: String,
    pub exec: Artifact,
    pub xml: Artifact,
    pub jar: Artifact,
    pub build: Artifact,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureReceipt {
    pub schema: String,
    pub control: Artifact,
    pub controller: crate::imap_soak_075::WorkerIdentity,
    pub cargo_streams: Vec<CompilerRecord>,
    pub objects: Vec<Artifact>,
    pub java: Vec<JavaSession>,
    pub preparation: Artifact,
    pub java_tools: Vec<Artifact>,
    pub completed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Analysis {
    pub schema: String,
    pub source_sha: String,
    pub run_id: String,
    pub profile: Profile,
    pub sources: Vec<Source>,
    pub raw_profiles: Vec<Artifact>,
    pub rust_report: Artifact,
    pub java_reports: Vec<Artifact>,
    pub summary: Summary,
    pub qualification: bool,
}

fn suffix(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}
fn runner(root: &Path) -> PathBuf {
    root.join(RUNNER_TARGET).join("debug").join(suffix("xtask"))
}
fn relative(root: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(root)?
        .to_string_lossy()
        .replace('\\', "/"))
}
fn bound(root: &Path, reference: &Artifact, maximum: usize) -> Result<PathBuf> {
    let path = root.join(&reference.path);
    if artifact(root, &path, maximum)? != *reference {
        return Err("coverage artifact bytes substituted".into());
    }
    Ok(path)
}
fn load<T: serde::de::DeserializeOwned>(root: &Path, path: &str) -> Result<T> {
    Ok(serde_json::from_slice(&read_relative(root, path, MAX)?)?)
}
fn store_new(root: &Path, path: &Path, value: &impl Serialize) -> Result<Artifact> {
    use std::io::Write;
    let bytes = serde_json::to_vec_pretty(value)?;
    if bytes.len() > MAX {
        return Err("coverage retained JSON bound".into());
    }
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    artifact(root, path, MAX)
}
fn output(root: &Path, path: &Path) -> Result<PathBuf> {
    crate::imap_pre074_075::new_output(root, path)
}
fn tool_paths() -> Result<BTreeMap<String, PathBuf>> {
    let mut command = Command::new("rustc");
    command.args(["--print", "sysroot"]);
    let observed = crate::imap_proof_process_075::execute(command, &[], Default::default())?;
    let sysroot = PathBuf::from(String::from_utf8(observed.stdout)?.trim());
    let target = crate::imap_sbom_075::CargoTarget::current()?;
    let bin = sysroot.join("lib/rustlib").join(&target.triple).join("bin");
    let java = crate::imap_api_differential::java_executable().canonicalize()?;
    let rustc = sysroot.join("bin").join(suffix("rustc"));
    let cargo = sysroot.join("bin").join(suffix("cargo"));
    Ok([
        ("llvm-cov".into(), bin.join(suffix("llvm-cov"))),
        ("llvm-profdata".into(), bin.join(suffix("llvm-profdata"))),
        ("java".into(), java),
        ("rustc".into(), rustc),
        ("cargo".into(), cargo),
    ]
    .into_iter()
    .collect())
}
fn fingerprint(path: &Path, args: &[&str]) -> Result<String> {
    let mut command = Command::new(path);
    command.args(args);
    let result = crate::imap_proof_process_075::execute(
        command,
        &[],
        crate::imap_proof_process_075::ProcessBounds {
            stdout_bytes: 8192,
            stderr_bytes: 8192,
            ..Default::default()
        },
    )?;
    let mut bytes = result.stdout;
    bytes.extend(result.stderr);
    Ok(String::from_utf8(bytes)?.trim().to_owned())
}
pub fn current_profile() -> Result<Profile> {
    validate_policy(&serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/coverage-profile.json"
    ))?)?;
    if !["windows", "linux"].contains(&std::env::consts::OS) || std::env::consts::ARCH != "x86_64" {
        return Err("coverage platform is outside the admitted profile".into());
    }
    let mut tools = Vec::new();
    for (name, path) in tool_paths()? {
        let args = if name == "java" {
            vec![
                "-Xms16m",
                "-Xmx64m",
                "-XX:ActiveProcessorCount=2",
                "--version",
            ]
        } else if name == "rustc" {
            vec!["-vV"]
        } else {
            vec!["--version"]
        };
        let version = fingerprint(&path, &args)?;
        let metadata = fs::metadata(&path)?;
        tools.push(Tool {
            name,
            version,
            sha256: crate::imap_daemon_probe_075::binary_digest(&path)?,
            bytes: metadata.len(),
        });
    }
    let rust = tools
        .iter()
        .find(|t| t.name == "rustc")
        .ok_or("coverage rustc missing")?;
    if !["release: 1.88.0", "release: 1.94.0"]
        .iter()
        .any(|release| rust.version.lines().any(|line| line == *release))
    {
        return Err("coverage Rust toolchain is outside the admitted 0.75 matrix".into());
    }
    let java = tools
        .iter()
        .find(|t| t.name == "java")
        .ok_or("coverage JDK missing")?
        .version
        .clone();
    if !["17.", "21."].iter().any(|v| {
        java.lines().next().is_some_and(|l| {
            l.starts_with(&format!("java {v}")) || l.starts_with(&format!("openjdk {v}"))
        })
    }) {
        return Err("coverage JDK is outside the admitted 0.75 matrix".into());
    }
    let java_path = crate::imap_api_differential::java_executable().canonicalize()?;
    let java_home = java_path
        .parent()
        .and_then(Path::parent)
        .ok_or("coverage JDK layout")?;
    for (name, relative) in [
        ("jdk-release", "release"),
        ("jdk-modules", "lib/modules"),
        (
            "jdk-jvm",
            if cfg!(windows) {
                "bin/server/jvm.dll"
            } else {
                "lib/server/libjvm.so"
            },
        ),
    ] {
        let file = java_home.join(relative);
        tools.push(Tool {
            name: name.into(),
            sha256: crate::imap_daemon_probe_075::binary_digest(&file.canonicalize()?)?,
            bytes: fs::metadata(file)?.len(),
            version: java.clone(),
        });
    }
    for input in crate::imap_sbom_capture_075::rust_materials(
        &crate::imap_sbom_075::CargoTarget::current()?.triple,
    )? {
        tools.push(Tool {
            name: format!(
                "rust-distribution:{}",
                input["path"].as_str().ok_or("Rust input path")?
            ),
            sha256: input["sha256"].as_str().ok_or("Rust input hash")?.into(),
            bytes: input["bytes"].as_u64().ok_or("Rust input size")?,
            version: String::new(),
        });
    }
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Profile {
        schema: "imap075-coverage-profile-v2".into(),
        baseline: BASELINE.into(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        rust_flags: RUST_FLAGS.into(),
        cargo_config: CARGO_CONFIG.iter().map(|c| (*c).to_owned()).collect(),
        feature: "imap-075-coverage".into(),
        target: crate::imap_sbom_075::CargoTarget::current()?,
        tools,
    })
}
pub fn validate_policy(value: &Value) -> Result<()> {
    let expected = serde_json::json!({"schema":"imap075-coverage-policy-v2","baseline":BASELINE,"production_enabled":false,"rust":["1.88.0","1.94.0"],"java":[17,21],"platforms":["linux-x86_64","windows-x86_64"],"rust_flags":["-C","instrument-coverage","-C","debuginfo=0"],"cargo_config":CARGO_CONFIG,"scopes":SCOPES,"require_all_changed_executable_lines":true,"claim_branch_coverage":false,"claim_production_correctness":false,"historical_global_ratchet":"unchanged"});
    if *value != expected {
        return Err("coverage policy differs from fixed source/cfg/line contract".into());
    }
    Ok(())
}
pub fn validate_control(
    control: &Control,
    source: &str,
    profile: &Profile,
    sources: &[Source],
) -> Result<()> {
    if control.schema != "imap075-coverage-control-v2"
        || !hex(source, 40)
        || control.source_sha != source
        || control.production_enabled
        || control.run_id.len() != 32
        || !hex(&control.run_id, 32)
        || control.profile != *profile
        || control.sources != sources
    {
        return Err("stale/foreign/empty coverage source/tool/cfg profile".into());
    }
    Ok(())
}
fn instrumentation(command: &mut Command, root: &Path, raw: &Path) {
    for configuration in CARGO_CONFIG {
        command.args(["--config", configuration]);
    }
    for key in [
        "RUSTFLAGS",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_TARGET",
    ] {
        command.env_remove(key);
    }
    command
        .env(
            "CARGO_ENCODED_RUSTFLAGS",
            ["-C", "instrument-coverage", "-C", "debuginfo=0"].join("\x1f"),
        )
        .env("CARGO_TARGET_DIR", root.join(RUNNER_TARGET))
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_PROFILE_TEST_DEBUG", "0")
        .env("CARGO_PROFILE_DEV_OPT_LEVEL", "0")
        .env("CARGO_PROFILE_TEST_OPT_LEVEL", "0")
        .env("CARGO_PROFILE_DEV_DEBUG_ASSERTIONS", "true")
        .env("CARGO_PROFILE_TEST_DEBUG_ASSERTIONS", "true")
        .env("CARGO_PROFILE_DEV_OVERFLOW_CHECKS", "true")
        .env("CARGO_PROFILE_TEST_OVERFLOW_CHECKS", "true")
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_BUILD_JOBS", "2")
        .env("RUST_TEST_THREADS", "1")
        .env("LLVM_PROFILE_FILE", raw.join("%p.profraw"));
}
pub fn runner_command(root: &Path, raw: &Path) -> Command {
    let mut command = Command::new("cargo");
    command.current_dir(root).args([
        "build",
        "-p",
        "xtask",
        "--bin",
        "xtask",
        "--features",
        "imap-075-coverage",
        "--message-format=json-render-diagnostics",
        "--locked",
    ]);
    instrumentation(&mut command, root, raw);
    command
}
pub trait CoverageDriver {
    fn execute(
        &mut self,
        stage: crate::imap_build_process_075::BuildStage,
        command: Command,
    ) -> Result<crate::imap_build_process_075::BuildReport>;
}
struct ManagedDriver(crate::imap_build_process_075::BuildController);
impl ManagedDriver {
    fn new(root: &Path) -> Result<Self> {
        use crate::imap_build_process_075::{remaining_budget, BuildBounds, BuildController};
        let remaining = remaining_budget()?;
        Ok(Self(BuildController::with_budget(
            root,
            remaining,
            BuildBounds {
                phase_timeout: remaining,
                ..Default::default()
            },
            Default::default(),
        )?))
    }
}
impl CoverageDriver for ManagedDriver {
    fn execute(
        &mut self,
        stage: crate::imap_build_process_075::BuildStage,
        command: Command,
    ) -> Result<crate::imap_build_process_075::BuildReport> {
        Ok(self.0.run(stage, command)?)
    }
}
/// The live controller is a copy; Cargo may rebuild the runner target while it executes.
pub fn execute_pipeline(
    root: &Path,
    path: &Path,
    source_sha: String,
    profile: Profile,
    sources: Vec<Source>,
    driver: &mut impl CoverageDriver,
) -> Result<PathBuf> {
    use crate::imap_build_process_075::BuildStage;
    let directory = output(root, path)?;
    for name in [
        "controller",
        "objects",
        "raw",
        "raw/controller",
        "raw/tooling",
        "raw/testkit",
        "raw/consumer",
        "raw/analysis",
        "raw/bootstrap",
    ] {
        fs::create_dir(directory.join(name))?;
    }
    let bootstrap_report = driver.execute(
        BuildStage::CoverageRunner,
        runner_command(root, &directory.join("raw/bootstrap")),
    )?;
    let bootstrap =
        snapshot_compiler_record(root, &directory, report_output(root, &bootstrap_report)?)?;
    let copied = directory.join("controller").join(suffix("xtask"));
    fs::copy(runner(root), &copied)?;
    if crate::imap_ci_075::source_state(root)? != source_sha {
        return Err("source changed while preparing instrumented runner".into());
    }
    let nonce = format!(
        "{}:{}:{:?}",
        relative(root, &directory)?,
        std::process::id(),
        std::time::SystemTime::now()
    );
    let run_id = digest(nonce.as_bytes())[..32].to_owned();
    let control = Control {
        schema: "imap075-coverage-control-v2".into(),
        source_sha,
        run_id,
        production_enabled: false,
        profile,
        sources,
        producer: artifact(root, &copied, 512 * 1024 * 1024)?,
        bootstrap,
    };
    let control_path = directory.join("control.json");
    store_new(root, &control_path, &control)?;
    for (mode, raw) in [
        ("imap-coverage-run", "raw/controller"),
        ("imap-coverage-analyze", "raw/analysis"),
    ] {
        let mut command = Command::new(&copied);
        command
            .current_dir(root)
            .args([mode, "--release", "0.75", "--control"])
            .arg(relative(root, &control_path)?)
            .env("LLVM_PROFILE_FILE", directory.join(raw).join("%p.profraw"));
        for variable in [
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_TARGET_DIR",
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
        ] {
            command.env_remove(variable);
        }
        driver.execute(BuildStage::CoverageExecution, command)?;
    }
    Ok(directory.join("capture.json"))
}
fn read_control(root: &Path, path: &str) -> Result<(Control, PathBuf)> {
    if !path.starts_with("target/imap-075/")
        || Path::new(path).file_name().and_then(|s| s.to_str()) != Some("control.json")
    {
        return Err("fixed owned coverage control path required".into());
    }
    let control: Control = load(root, path)?;
    let directory = root
        .join(path)
        .parent()
        .ok_or("coverage control parent")?
        .to_path_buf();
    validate_control(
        &control,
        &crate::imap_ci_075::source_state(root)?,
        &current_profile()?,
        &source_snapshot(root)?,
    )?;
    if control.producer.path != relative(root, &directory.join("controller").join(suffix("xtask")))?
    {
        return Err("foreign coverage controller".into());
    }
    bound(root, &control.producer, 512 * 1024 * 1024)?;
    Ok((control, directory))
}
fn identity() -> Result<crate::imap_soak_075::WorkerIdentity> {
    Ok(crate::imap_soak_075::WorkerIdentity {
        pid: std::process::id(),
        platform_birth: crate::imap_soak_075::process_identity(std::process::id())?,
        incarnation: format!("coverage-{}", std::process::id()),
    })
}
fn dead(identity: &crate::imap_soak_075::WorkerIdentity) -> Result<()> {
    if identity.pid == 0
        || identity.platform_birth.is_empty()
        || identity.platform_birth.len() > 256
        || identity.incarnation != format!("coverage-{}", identity.pid)
    {
        return Err("invalid coverage controller identity".into());
    }
    match crate::imap_soak_075::process_identity(identity.pid) {
        Ok(current) if current == identity.platform_birth => {
            Err("coverage controller still lives; raw profile incomplete".into())
        }
        Ok(_) => Ok(()),
        Err(error) => {
            if let Some(io) = error.downcast_ref::<std::io::Error>() {
                if matches!(
                    io.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidInput
                ) {
                    return Ok(());
                }
            }
            #[cfg(windows)]
            {
                if let Some(io) = error.downcast_ref::<std::io::Error>() {
                    if io.raw_os_error() == Some(87) {
                        return Ok(());
                    }
                }
            }
            Err(error)
        }
    }
}
fn report_output(
    root: &Path,
    report: &crate::imap_build_process_075::BuildReport,
) -> Result<Artifact> {
    artifact(root, Path::new(&report.stdout_path), MAX)
}
/// Cargo's executable field omits host build scripts and loaded proc-macro libraries.
pub fn declared_objects(root: &Path, bytes: &[u8]) -> Result<BTreeSet<String>> {
    if bytes.len() > MAX {
        return Err("compiler object stream byte bound".into());
    }
    let mut paths = BTreeSet::new();
    for line in bytes
        .split(|b| *b == b'\n')
        .filter(|l| l.first() == Some(&b'{'))
    {
        let row: Value = serde_json::from_slice(line)?;
        if row["reason"] == "compiler-artifact" {
            let mut candidates = Vec::new();
            if let Some(executable) = row["executable"].as_str() {
                candidates.push(executable);
            }
            let contains = |field: &str, value: &str| {
                row["target"][field]
                    .as_array()
                    .is_some_and(|items| items.iter().any(|item| item == value))
            };
            let script = contains("kind", "custom-build");
            let dynamic = contains("kind", "proc-macro")
                || contains("crate_types", "dylib")
                || contains("crate_types", "cdylib");
            if script || dynamic {
                let files = row["filenames"]
                    .as_array()
                    .ok_or("compiler object filenames missing")?;
                let selected = files
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|name| {
                        let path = Path::new(name);
                        if script {
                            path.file_name().and_then(|s| s.to_str())
                                == Some(&suffix("build-script-build"))
                        } else {
                            matches!(
                                path.extension().and_then(|s| s.to_str()),
                                Some("dll" | "so" | "dylib")
                            )
                        }
                    })
                    .collect::<Vec<_>>();
                if selected.is_empty() {
                    return Err("compiler host object missing from filenames".into());
                }
                candidates.extend(selected);
            }
            for executable in candidates {
                let path = Path::new(executable);
                if !path.is_absolute() || !path.starts_with(root.join(RUNNER_TARGET)) {
                    return Err("foreign instrumented Cargo output".into());
                }
                let path = relative(root, path)?;
                if Path::new(&path)
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
                {
                    return Err("aliased instrumented Cargo output".into());
                }
                paths.insert(path);
                if paths.len() > MAX_OBJECTS {
                    return Err("instrumented binary set bounded".into());
                }
            }
        }
    }
    if paths.is_empty() {
        return Err("instrumented binary set empty".into());
    }
    Ok(paths)
}
/// Called immediately after a physically completed Cargo phase, before another can relink.
pub fn snapshot_compiler_record(
    root: &Path,
    directory: &Path,
    stream: Artifact,
) -> Result<CompilerRecord> {
    bound(root, &stream, MAX)?;
    let mut objects = Vec::new();
    let mut retained = 0;
    let mut count = 0;
    for entry in fs::read_dir(directory.join("objects"))? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Err("compiler archive contains a non-file".into());
        }
        retained = object_total(retained, entry.metadata()?.len())?;
        count += 1;
        object_count(count)?;
    }
    for executable in declared_objects(root, &read_relative(root, &stream.path, MAX)?)? {
        let original = artifact(root, &root.join(&executable), 512 * 1024 * 1024)?;
        let destination = directory
            .join("objects")
            .join(format!("{}.bin", original.sha256));
        if !destination.exists() {
            retained = object_total(retained, original.bytes)?;
            count += 1;
            object_count(count)?;
            use std::io::Write;
            let mut pending = tempfile::NamedTempFile::new_in(directory.join("objects"))?;
            std::io::copy(&mut fs::File::open(root.join(&executable))?, &mut pending)?;
            pending.flush()?;
            pending.persist_noclobber(&destination)?;
        }
        let archived = artifact(root, &destination, 512 * 1024 * 1024)?;
        if archived.sha256 != original.sha256 || archived.bytes != original.bytes {
            return Err("compiler object changed during archival".into());
        }
        objects.push(CompiledObject {
            executable,
            archived,
        });
    }
    Ok(CompilerRecord { stream, objects })
}
pub fn compiler_objects(
    root: &Path,
    directory: &Path,
    streams: &[CompilerRecord],
) -> Result<Vec<Artifact>> {
    if streams.is_empty() || streams.len() > 16 {
        return Err("compiler record count bound".into());
    }
    let mut seen = BTreeSet::new();
    let mut objects = BTreeMap::new();
    let mut total = 0;
    for record in streams {
        bound(root, &record.stream, MAX)?;
        if !seen.insert(&record.stream.path) {
            return Err("duplicate compiler record".into());
        }
        let expected = declared_objects(root, &read_relative(root, &record.stream.path, MAX)?)?;
        let actual = record
            .objects
            .iter()
            .map(|o| o.executable.clone())
            .collect::<BTreeSet<_>>();
        if expected != actual || actual.len() != record.objects.len() {
            return Err("compiler object correspondence missing/duplicate/foreign".into());
        }
        for object in &record.objects {
            if object.archived.path
                != relative(
                    root,
                    &directory
                        .join("objects")
                        .join(format!("{}.bin", object.archived.sha256)),
                )?
            {
                return Err("compiler object archive alias".into());
            }
            bound(root, &object.archived, 512 * 1024 * 1024)?;
            if !objects.contains_key(&object.archived.path) {
                total = object_total(total, object.archived.bytes)?;
                objects.insert(object.archived.path.clone(), object.archived.clone());
            }
            object_count(objects.len())?;
        }
    }
    let retained = fs::read_dir(directory.join("objects"))?
        .take(MAX_OBJECTS + 1)
        .map(|entry| {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                return Err("compiler archive contains a non-file".into());
            }
            relative(root, &entry.path())
        })
        .collect::<Result<BTreeSet<_>>>()?;
    if retained != objects.keys().cloned().collect() {
        return Err("compiler archive has missing/unregistered objects".into());
    }
    Ok(objects.into_values().collect())
}
#[cfg(not(feature = "imap-075-coverage"))]
fn capture_worker(_: &Path, _: &str) -> Result<()> {
    Err("coverage runner must use the trusted instrumentation feature".into())
}
#[cfg(feature = "imap-075-coverage")]
fn capture_worker(root: &Path, path: &str) -> Result<()> {
    use crate::imap_build_process_075::BuildStage;
    let (control, directory) = read_control(root, path)?;
    if artifact(root, &std::env::current_exe()?, 512 * 1024 * 1024)? != control.producer {
        return Err("coverage runner bytes differ from prepared controller".into());
    }
    store_new(root, &directory.join("capture-started.json"), &identity()?)?;
    let mut driver = ManagedDriver::new(root)?;
    let mut streams = vec![control.bootstrap.clone()];
    let mut command = crate::imap_ci_075::tooling_command(root, cfg!(windows));
    command.args([
        "--features",
        "imap-075-coverage",
        "--message-format=json-render-diagnostics",
    ]);
    instrumentation(&mut command, root, &directory.join("raw/tooling"));
    let report = driver.execute(BuildStage::ToolingTests, command)?;
    streams.push(snapshot_compiler_record(
        root,
        &directory,
        report_output(root, &report)?,
    )?);
    let mut command = Command::new("cargo");
    command.current_dir(root).args([
        "test",
        "-p",
        "hydracache-cluster-testkit",
        "--test",
        "imap_backend_contract_075",
        "--test",
        "imap_backend_helper_contract_075",
        "--test",
        "value_plane_service_async_075",
        "--locked",
        "--message-format=json-render-diagnostics",
    ]);
    instrumentation(&mut command, root, &directory.join("raw/testkit"));
    let report = driver.execute(BuildStage::HelperContractTests, command)?;
    streams.push(snapshot_compiler_record(
        root,
        &directory,
        report_output(root, &report)?,
    )?);
    capture_consumer(root, &directory, &mut driver, &mut streams)?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .current_dir(root)
        .args(["imap-pre074-evidence", "--release", "0.75", "--output"])
        .arg(relative(root, &directory.join("preparation"))?)
        .env(
            "LLVM_PROFILE_FILE",
            directory.join("raw/controller").join("%p.profraw"),
        );
    for key in [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_TARGET_DIR",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
    ] {
        command.env_remove(key);
    }
    driver.execute(BuildStage::CoverageExecution, command)?;
    let preparation = artifact(root, &directory.join("preparation/receipt.json"), MAX)?;
    // Exercise the public entry points as a user would, after their real artifacts exist.
    // Resume starts a separate short segment; it never extends the smoke dwell claim.
    for arguments in public_workloads(&relative(root, &directory)?) {
        let mut command = Command::new(std::env::current_exe()?);
        command.current_dir(root).args(arguments).env(
            "LLVM_PROFILE_FILE",
            directory.join("raw/controller").join("%p.profraw"),
        );
        driver.execute(BuildStage::CoverageExecution, command)?;
    }
    let prepared: crate::imap_pre074_075::PreparationReceipt = load(root, &preparation.path)?;
    let provenance: crate::imap_sbom_075::Provenance =
        load(root, &prepared.development_inventory.path)?;
    let inputs: crate::imap_sbom_capture_075::Capture = load(root, &provenance.capture.path)?;
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args([
            "test",
            "-p",
            "xtask",
            "--test",
            "imap_sbom_075",
            "--test",
            "imap_soak_075",
            "--features",
            "imap-075-coverage",
            "--locked",
            "--message-format=json-render-diagnostics",
        ])
        .env("IMAP075_CARGO_FIXTURE", &inputs.directory)
        .env(
            "IMAP075_PROVENANCE_FIXTURE",
            &prepared.development_inventory.path,
        )
        .env(
            "IMAP075_SOAK_FIXTURE",
            relative(root, &directory.join("soak-cli/reference"))?,
        );
    instrumentation(&mut command, root, &directory.join("raw/tooling"));
    command.args(["--", "actual_", "--ignored"]);
    let report = driver.execute(BuildStage::ToolingTests, command)?;
    streams.push(snapshot_compiler_record(
        root,
        &directory,
        report_output(root, &report)?,
    )?);
    let mut command = Command::new(std::env::current_exe()?);
    command
        .current_dir(root)
        .args(["imap-sbom-check", "--release", "0.75", "--receipt"])
        .arg(&prepared.development_inventory.path)
        .env(
            "LLVM_PROFILE_FILE",
            directory.join("raw/controller").join("%p.profraw"),
        );
    driver.execute(BuildStage::CoverageExecution, command)?;
    let java: Vec<JavaSession> = load(
        root,
        &format!("{}/coverage-sessions.json", inputs.directory),
    )?;
    let mut objects = compiler_objects(root, &directory, &streams)?;
    objects.push(control.producer.clone());
    objects.sort_by(|a, b| a.path.cmp(&b.path));
    let receipt = CaptureReceipt {
        schema: "imap075-coverage-capture-v2".into(),
        control: artifact(root, &root.join(path), MAX)?,
        controller: identity()?,
        cargo_streams: streams,
        objects,
        java,
        preparation,
        java_tools: java_tool_artifacts(root)?,
        completed: true,
    };
    if crate::imap_ci_075::source_state(root)? != control.source_sha
        || source_snapshot(root)? != control.sources
        || current_profile()? != control.profile
    {
        return Err("coverage inputs changed during execution".into());
    }
    store_new(root, &directory.join("capture.json"), &receipt)?;
    Ok(())
}
pub fn public_workloads(directory: &str) -> Vec<Vec<String>> {
    let mut result: Vec<Vec<String>> = [
        "imap-native-observation",
        "imap-native-concurrent",
        "imap-native-listener-recovery",
    ]
    .into_iter()
    .map(|name| vec![name.into(), "--release".into(), "0.75".into()])
    .collect();
    for (cohort, output, parent) in [
        ("all", "soak-cli", None),
        (
            "reference",
            "soak-resume-checkpoint",
            Some("checkpoint.json"),
        ),
        ("reference", "soak-resume-report", Some("report.json")),
    ] {
        let mut arguments: Vec<String> = [
            "imap-soak",
            "--release",
            "0.75",
            "--cohort",
            cohort,
            "--profile",
            "smoke",
            "--host-budget-seconds",
            "180",
            "--output",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        arguments.push(format!("{directory}/{output}"));
        if let Some(parent) = parent {
            arguments.extend([
                "--resume".into(),
                format!("{directory}/soak-cli/reference/{parent}"),
            ]);
        }
        result.push(arguments);
    }
    result
}
#[cfg(feature = "imap-075-coverage")]
fn capture_consumer(
    root: &Path,
    directory: &Path,
    driver: &mut impl CoverageDriver,
    streams: &mut Vec<CompilerRecord>,
) -> Result<()> {
    let temp = consumer_workspace(root)?;
    fs::create_dir(temp.path().join("src"))?;
    fs::copy(
        root.join("tests/rust-imap-contract-consumer-075/src/main.rs"),
        temp.path().join("src/main.rs"),
    )?;
    let dependency = root
        .join("crates/hydracache-cluster-testkit")
        .to_string_lossy()
        .replace('\\', "/");
    fs::write(
        temp.path().join("Cargo.toml"),
        toml::to_string(
            &serde_json::json!({"package":{"name":"imap-contract-consumer-075","version":"0.0.0","edition":"2021","publish":false},"workspace":{},"dependencies":{"hydracache-cluster-testkit":{"path":dependency}}}),
        )?,
    )?;
    fs::copy(root.join("Cargo.lock"), temp.path().join("Cargo.lock"))?;
    let remap = format!(
        "--remap-path-prefix={}={}",
        temp.path().display(),
        root.join("tests/rust-imap-contract-consumer-075").display()
    );
    for action in ["test", "build"] {
        let mut command = Command::new("cargo");
        command.current_dir(temp.path()).args([
            action,
            "--offline",
            "--message-format=json-render-diagnostics",
        ]);
        if action == "build" {
            command.arg("--locked");
        }
        instrumentation(&mut command, root, &directory.join("raw/consumer"));
        command.env_remove("RUSTFLAGS").env(
            "CARGO_ENCODED_RUSTFLAGS",
            ["-C", "instrument-coverage", "-C", "debuginfo=0", &remap].join("\x1f"),
        );
        let report = driver.execute(
            crate::imap_build_process_075::BuildStage::CoverageExecution,
            command,
        )?;
        streams.push(snapshot_compiler_record(
            root,
            directory,
            report_output(root, &report)?,
        )?);
        crate::imap_pre074_075::validate_consumer_lock(
            &fs::read_to_string(root.join("Cargo.lock"))?,
            &fs::read_to_string(temp.path().join("Cargo.lock"))?,
        )?;
    }
    let mut command = Command::new(
        root.join(RUNNER_TARGET)
            .join("debug")
            .join(suffix("imap-contract-consumer-075")),
    );
    command.env(
        "LLVM_PROFILE_FILE",
        directory.join("raw/consumer").join("%p.profraw"),
    );
    driver.execute(
        crate::imap_build_process_075::BuildStage::CoverageExecution,
        command,
    )?;
    Ok(())
}

/// A single owned temporary workspace keeps the remap flag and dependency cache
/// stable across captures. Existing/stale workspaces refuse; their contents are
/// never reused. TempDir removes only this newly created workspace on drop.
pub fn consumer_workspace(root: &Path) -> Result<tempfile::TempDir> {
    Ok(tempfile::Builder::new()
        .prefix("coverage-consumer-workspace")
        .rand_bytes(0)
        .tempdir_in(root.join("target/imap-075"))?)
}

/// Code-chosen JaCoCo phase arguments; inherited JVM injection remains scrubbed by the executor.
pub fn java_arguments(
    root: &Path,
    directory: &Path,
    module: &str,
) -> Result<(String, Vec<String>)> {
    if ![
        "java-imap-benchmark",
        "java-imap-hazelcast-075",
        "java-imap-provenance-075",
    ]
    .contains(&module)
    {
        return Err("foreign coverage Java module".into());
    }
    let id = format!(
        "imap075_{}_{}",
        &digest(relative(root, directory)?.as_bytes())[..16],
        module.replace('-', "_")
    );
    let exec = directory.join(format!("{module}.exec"));
    if exec.exists() {
        return Err("JaCoCo execution file must be fresh".into());
    }
    Ok((
        id.clone(),
        vec![
            format!("-Djacoco.destFile={}", exec.display()),
            format!("-Djacoco.dataFile={}", exec.display()),
            format!("-Djacoco.sessionId={id}"),
            "-Djacoco.append=false".into(),
        ],
    ))
}
pub fn retain_java_session(
    root: &Path,
    directory: &Path,
    module: &str,
    id: &str,
    report: &crate::imap_build_process_075::BuildReport,
) -> Result<()> {
    if report.outcome != crate::imap_build_process_075::BuildOutcome::Completed
        || !report.root_reaped
        || report.remaining_live_members != Some(0)
    {
        return Err("Java coverage phase incomplete".into());
    }
    let xml = directory.join(format!("{module}.xml"));
    fs::copy(
        root.join(format!("tests/{module}/target/site/jacoco/jacoco.xml")),
        &xml,
    )?;
    let build = Path::new(&report.stdout_path).with_extension("json");
    let session = JavaSession {
        schema: "imap075-java-coverage-session-v1".into(),
        module: module.into(),
        id: id.into(),
        source_sha: crate::imap_ci_075::source_state(root)?,
        exec: artifact(
            root,
            &directory.join(format!("{module}.exec")),
            32 * 1024 * 1024,
        )?,
        xml: artifact(root, &xml, MAX)?,
        jar: artifact(
            root,
            &root.join(format!(
                "tests/{module}/target/{module}-0.75.0-SNAPSHOT.jar"
            )),
            32 * 1024 * 1024,
        )?,
        build: artifact(root, &build, MAX)?,
    };
    let path = directory.join("coverage-sessions.json");
    let mut sessions: Vec<JavaSession> = if path.exists() {
        serde_json::from_slice(&fs::read(&path)?)?
    } else {
        vec![]
    };
    if sessions.len() >= 3 || sessions.iter().any(|s| s.module == module || s.id == id) {
        return Err("duplicate Java coverage phase/session".into());
    }
    sessions.push(session);
    sessions.sort_by(|a, b| a.module.cmp(&b.module));
    fs::write(path, serde_json::to_vec_pretty(&sessions)?)?;
    Ok(())
}
/// A measurement may defer only its own still-live analysis file. Qualification passes None.
pub fn collect_profiles(
    root: &Path,
    directory: &Path,
    live_analysis: Option<u32>,
) -> Result<Vec<Artifact>> {
    if live_analysis == Some(0) {
        return Err("invalid deferred analysis PID".into());
    }
    let mut result = Vec::new();
    let mut total = 0u64;
    for role in ["controller", "tooling", "testkit", "consumer", "analysis"] {
        let before = result.len();
        for entry in fs::read_dir(directory.join("raw").join(role))? {
            let path = entry?.path();
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or("invalid LLVM profile filename")?;
            let pid = name.strip_suffix(".profraw").and_then(|stem| {
                stem.parse::<u32>()
                    .ok()
                    .filter(|p| *p > 0 && p.to_string() == stem)
            });
            if pid.is_none() || result.len() >= 2048 {
                return Err("foreign/oversized LLVM raw profile set".into());
            }
            if role == "analysis" && pid == live_analysis {
                continue;
            }
            let reference = artifact(root, &path, MAX_RAW)?;
            total = profile_total(total, reference.bytes)?;
            result.push(reference);
        }
        if role != "analysis" && before == result.len() {
            return Err(format!("LLVM profile execution phase missing: {role}").into());
        }
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    if result.is_empty() {
        return Err("LLVM execution profiles missing".into());
    }
    Ok(result)
}
pub fn profile_total(total: u64, bytes: u64) -> Result<u64> {
    let sum = total
        .checked_add(bytes)
        .ok_or("LLVM profile bytes overflow")?;
    if bytes == 0 || bytes > MAX_RAW as u64 || sum > MAX_RAW_TOTAL {
        return Err("LLVM raw profile aggregate bound".into());
    }
    Ok(sum)
}
fn java_tool_artifacts(root: &Path) -> Result<Vec<Artifact>> {
    [
        crate::imap_sbom_capture_075::SPY.to_owned(),
        format!(
            "{}/org/jacoco/org.jacoco.cli/0.8.13/org.jacoco.cli-0.8.13-nodeps.jar",
            crate::imap_sbom_capture_075::REPOSITORY
        ),
    ]
    .iter()
    .map(|path| artifact(root, &root.join(path), 32 * 1024 * 1024))
    .collect()
}
pub fn validate_measured_profile(profile: &Value) -> Result<()> {
    if profile["opt_level"] != "0"
        || profile["debuginfo"] != 0
        || profile["debug_assertions"] != true
        || profile["overflow_checks"] != true
    {
        return Err("scoped implementation requires an unoptimized checked Cargo profile".into());
    }
    Ok(())
}
fn contexts(
    root: &Path,
    sources: &[Source],
    streams: &[Artifact],
    profile: &Profile,
) -> Result<BTreeMap<String, Vec<crate::imap_sbom_075::CargoTarget>>> {
    let mut owners: BTreeMap<String, Vec<crate::imap_sbom_075::CargoTarget>> = BTreeMap::new();
    for stream in streams {
        for line in read_relative(root, &stream.path, MAX)?
            .split(|b| *b == b'\n')
            .filter(|l| l.first() == Some(&b'{'))
        {
            let row: Value = serde_json::from_slice(line)?;
            if row["reason"] != "compiler-artifact" {
                continue;
            }
            let source = row["target"]["src_path"]
                .as_str()
                .ok_or("compiler source path missing")?
                .replace('\\', "/");
            let owner = if source.ends_with("/crates/xtask/src/lib.rs") {
                "xtask-lib"
            } else if source.ends_with("/crates/xtask/src/main.rs") {
                "xtask-main"
            } else if source.ends_with("/crates/xtask/src/bin/imap-soak-worker-075.rs") {
                "xtask-worker"
            } else if source.ends_with("/crates/hydracache-cluster-testkit/src/lib.rs") {
                "testkit"
            } else if row["target"]["name"] == "imap-contract-consumer-075" {
                "consumer"
            } else {
                continue;
            };
            validate_measured_profile(&row["profile"])?;
            let mut cfg = profile.target.clone();
            if row["profile"]["test"] == true {
                cfg.cfg.insert("test".into());
            }
            for feature in row["features"]
                .as_array()
                .ok_or("compiler features missing")?
            {
                cfg.cfg.insert(format!(
                    "feature={:?}",
                    feature.as_str().ok_or("compiler feature string")?
                ));
            }
            let values = owners.entry(owner.into()).or_default();
            if !values.contains(&cfg) {
                values.push(cfg);
            }
        }
    }
    let mut result = BTreeMap::new();
    for source in sources
        .iter()
        .filter(|s| s.path.ends_with(".rs") && !s.added.is_empty())
    {
        let owner = if source.path == "crates/xtask/src/main.rs" {
            "xtask-main"
        } else if source.path == "crates/xtask/src/bin/imap-soak-worker-075.rs" {
            "xtask-worker"
        } else if source.path.starts_with("crates/xtask/src/") {
            "xtask-lib"
        } else if source
            .path
            .starts_with("crates/hydracache-cluster-testkit/")
        {
            "testkit"
        } else {
            "consumer"
        };
        result.insert(
            source.path.clone(),
            owners
                .get(owner)
                .ok_or("scoped compiler context absent")?
                .clone(),
        );
    }
    Ok(result)
}
fn normalized_java(root: &Path, xml: &Path, driver: &mut impl CoverageDriver) -> Result<Value> {
    let mut command = Command::new(crate::imap_api_differential::java_executable());
    command
        .args([
            "-Xms16m",
            "-Xmx128m",
            "-XX:ActiveProcessorCount=2",
            "-XX:ReservedCodeCacheSize=32m",
            "-XX:CompressedClassSpaceSize=64m",
            "-cp",
        ])
        .arg(root.join(crate::imap_sbom_capture_075::SPY))
        .arg("io.hydracache.imap.provenance.CoverageDataMain075")
        .arg(xml);
    let report = driver.execute(
        crate::imap_build_process_075::BuildStage::CoverageReport,
        command,
    )?;
    Ok(serde_json::from_slice(&fs::read(report.stdout_path)?)?)
}
/// LLVM response files keep Windows command lines bounded. Normalized path separators
/// avoid platform-specific backslash escaping; quotes/control bytes fail explicitly.
pub fn response_file(arguments: &[String]) -> Result<Vec<u8>> {
    if arguments.len() > 4096 {
        return Err("LLVM response argument count bound".into());
    }
    let mut output = String::new();
    for argument in arguments {
        if argument.starts_with('@') || argument.chars().any(|c| c.is_control() || c == '"') {
            return Err("unsupported LLVM response argument".into());
        }
        let normalized = if let Some(unc) = argument.strip_prefix(r"\\?\UNC\") {
            format!("//{unc}")
        } else {
            argument
                .strip_prefix(r"\\?\")
                .unwrap_or(argument)
                .to_owned()
        }
        .replace('\\', "/");
        output.push('"');
        output.push_str(&normalized);
        output.push_str("\"\n");
        if output.len() > MAX {
            return Err("LLVM response byte bound".into());
        }
    }
    Ok(output.into_bytes())
}
fn regenerate(
    root: &Path,
    directory: &Path,
    control: &Control,
    capture: &CaptureReceipt,
    raw: &[Artifact],
    driver: &mut impl CoverageDriver,
) -> Result<(Artifact, Vec<Artifact>, Lines)> {
    use std::io::Write;
    let tools = tool_paths()?;
    let temp = tempfile::tempdir_in(directory)?;
    let profdata = temp.path().join("merged.profdata");
    let mut list = tempfile::NamedTempFile::new_in(temp.path())?;
    for reference in raw {
        writeln!(list, "{}", bound(root, reference, MAX_RAW)?.display())?;
    }
    list.flush()?;
    let mut command = Command::new(&tools["llvm-profdata"]);
    command
        .args(["merge", "--sparse", "--failure-mode=any", "--input-files"])
        .arg(list.path())
        .arg("-o")
        .arg(&profdata);
    driver.execute(
        crate::imap_build_process_075::BuildStage::CoverageReport,
        command,
    )?;
    let known = control
        .sources
        .iter()
        .map(|s| s.path.clone())
        .collect::<BTreeSet<_>>();
    let mut arguments = vec![
        bound(root, &control.producer, 512 * 1024 * 1024)?
            .to_string_lossy()
            .into_owned(),
        "--format=lcov".into(),
        "--check-binary-ids".into(),
        "--instr-profile".into(),
        profdata.to_string_lossy().into_owned(),
        "--compilation-dir".into(),
        root.to_string_lossy().into_owned(),
    ];
    for object in &capture.objects {
        if object != &control.producer {
            arguments.push("--object".into());
            arguments.push(
                bound(root, object, 512 * 1024 * 1024)?
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    arguments.push("--sources".into());
    for source in control
        .sources
        .iter()
        .filter(|s| s.path.ends_with(".rs") && !s.added.is_empty())
    {
        arguments.push(root.join(&source.path).to_string_lossy().into_owned());
    }
    let mut response = tempfile::NamedTempFile::new_in(temp.path())?;
    response.write_all(&response_file(&arguments)?)?;
    response.flush()?;
    let mut command = Command::new(&tools["llvm-cov"]);
    command
        .arg("export")
        .arg(format!("@{}", response.path().display()));
    let report = driver.execute(
        crate::imap_build_process_075::BuildStage::CoverageReport,
        command,
    )?;
    let rust_report = report_output(root, &report)?;
    let mut lines = parse_lcov(
        &fs::read(&report.stdout_path)?,
        &root.to_string_lossy(),
        &known,
    )?;
    if capture.java.len() != 3 {
        return Err("coverage Java component missing".into());
    }
    let mut modules = BTreeSet::new();
    let mut sessions = BTreeSet::new();
    let mut java_reports = Vec::new();
    let cli = root
        .join(crate::imap_sbom_capture_075::REPOSITORY)
        .join("org/jacoco/org.jacoco.cli/0.8.13/org.jacoco.cli-0.8.13-nodeps.jar");
    for session in &capture.java {
        if session.schema != "imap075-java-coverage-session-v1"
            || session.source_sha != control.source_sha
            || !modules.insert(&session.module)
            || !sessions.insert(&session.id)
        {
            return Err("stale/duplicate Java execution identity".into());
        }
        let build: crate::imap_build_process_075::BuildReport = load(root, &session.build.path)?;
        bound(root, &session.build, MAX)?;
        if build.outcome != crate::imap_build_process_075::BuildOutcome::Completed
            || !build.logs_complete
            || !build.root_reaped
            || build.remaining_live_members != Some(0)
        {
            return Err("Java tests did not physically complete".into());
        }
        let archived = normalized_java(root, &bound(root, &session.xml, MAX)?, driver)?;
        let xml = temp.path().join(format!("{}.xml", session.module));
        let mut command = Command::new(crate::imap_api_differential::java_executable());
        command
            .args([
                "-Xms16m",
                "-Xmx128m",
                "-XX:ActiveProcessorCount=2",
                "-XX:ReservedCodeCacheSize=32m",
                "-XX:CompressedClassSpaceSize=64m",
                "-jar",
            ])
            .arg(&cli)
            .arg("report")
            .arg(bound(root, &session.exec, 32 * 1024 * 1024)?)
            .arg("--classfiles")
            .arg(bound(root, &session.jar, 32 * 1024 * 1024)?)
            .arg("--sourcefiles")
            .arg(root.join(format!("tests/{}/src/main/java", session.module)))
            .arg("--xml")
            .arg(&xml);
        driver.execute(
            crate::imap_build_process_075::BuildStage::CoverageReport,
            command,
        )?;
        let fresh = normalized_java(root, &xml, driver)?;
        if fresh != archived {
            return Err("JaCoCo XML differs from fresh raw.exec/JAR reconstruction".into());
        }
        let mapped = java_lines(
            &fresh,
            &session.module,
            &session.id,
            &root.to_string_lossy(),
            &known,
        )?;
        for (path, counts) in mapped {
            if lines.insert(path, counts).is_some() {
                return Err("coverage Java component duplicated/foreign".into());
            }
        }
        java_reports.push(session.xml.clone());
    }
    Ok((rust_report, java_reports, lines))
}
pub fn gate_result(summary: &Summary, strict: bool) -> Result<bool> {
    let result = require_complete(summary);
    if strict {
        result?;
        Ok(true)
    } else {
        Ok(false)
    }
}
fn analyze(root: &Path, path: &str, strict: bool) -> Result<Analysis> {
    let capture: CaptureReceipt = load(root, path)?;
    if capture.schema != "imap075-coverage-capture-v2" || !capture.completed {
        return Err("coverage execution incomplete/future".into());
    }
    bound(root, &capture.control, MAX)?;
    let (control, directory) = read_control(root, &capture.control.path)?;
    if path != relative(root, &directory.join("capture.json"))? {
        return Err("coverage capture alias/foreign path".into());
    }
    dead(&capture.controller)?;
    bound(root, &capture.preparation, MAX)?;
    let prepared: crate::imap_pre074_075::PreparationReceipt =
        load(root, &capture.preparation.path)?;
    crate::imap_pre074_075::validate_receipt_header(&prepared, &control.source_sha)?;
    if prepared.tooling != control.producer {
        return Err("coverage workload used another producer".into());
    }
    if capture.java_tools != java_tool_artifacts(root)? {
        return Err("coverage Java report tools substituted".into());
    }
    let live_analysis = analysis_phase(root, &directory, &control, strict)?;
    if capture.cargo_streams.len() != 6 || capture.cargo_streams.first() != Some(&control.bootstrap)
    {
        return Err("coverage bootstrap/compiler phases incomplete/substituted".into());
    }
    let mut expected = compiler_objects(root, &directory, &capture.cargo_streams)?;
    expected.push(control.producer.clone());
    expected.sort_by(|a, b| a.path.cmp(&b.path));
    if expected != capture.objects {
        return Err("coverage instrumented object set missing/substituted".into());
    }
    let raw = collect_profiles(root, &directory, live_analysis)?;
    let analysis_prefix = format!("{}/raw/analysis/", relative(root, &directory)?);
    if strict && !raw.iter().any(|a| a.path.starts_with(&analysis_prefix)) {
        return Err("completed analysis LLVM profile missing".into());
    }
    let mut driver = ManagedDriver::new(root)?;
    let (rust_report, java_reports, lines) =
        regenerate(root, &directory, &control, &capture, &raw, &mut driver)?;
    let contexts = contexts(
        root,
        &control.sources,
        &capture
            .cargo_streams
            .iter()
            .map(|r| r.stream.clone())
            .collect::<Vec<_>>(),
        &control.profile,
    )?;
    let ranges = compiled_out_ranges(root, &control.sources, &contexts)?;
    let summary = summarize(&control.sources, &lines, &control.profile.os, ranges)?;
    let qualification = gate_result(&summary, strict)?;
    if crate::imap_ci_075::source_state(root)? != control.source_sha
        || source_snapshot(root)? != control.sources
        || current_profile()? != control.profile
    {
        return Err("coverage inputs changed during analysis".into());
    }
    Ok(Analysis {
        schema: "imap075-scoped-coverage-v2".into(),
        source_sha: control.source_sha,
        run_id: control.run_id,
        profile: control.profile,
        sources: control.sources,
        raw_profiles: raw,
        rust_report,
        java_reports,
        summary,
        qualification,
    })
}
pub fn analysis_phase(
    root: &Path,
    directory: &Path,
    control: &Control,
    strict: bool,
) -> Result<Option<u32>> {
    let started: crate::imap_soak_075::WorkerIdentity = load(
        root,
        &relative(root, &directory.join("analysis-started.json"))?,
    )?;
    if strict {
        dead(&started)?;
        let measured: Analysis = load(root, &relative(root, &directory.join("analysis.json"))?)?;
        if measured.schema != "imap075-scoped-coverage-v2"
            || measured.qualification
            || measured.source_sha != control.source_sha
            || measured.run_id != control.run_id
            || measured.profile != control.profile
            || measured.sources != control.sources
        {
            return Err("coverage analysis phase incomplete/stale".into());
        }
        Ok(None)
    } else {
        let current = identity()?;
        if started.pid != current.pid
            || started.platform_birth != current.platform_birth
            || started.incarnation != current.incarnation
        {
            return Err("measurement may defer only its own live profile".into());
        }
        Ok(Some(current.pid))
    }
}
pub fn capture(root: &Path, path: &Path) -> Result<PathBuf> {
    let source = crate::imap_ci_075::source_state(root)?;
    let profile = current_profile()?;
    let sources = source_snapshot(root)?;
    let mut driver = ManagedDriver::new(root)?;
    let receipt = execute_pipeline(root, path, source, profile, sources, &mut driver)?;
    let analysis = analyze(root, &relative(root, &receipt)?, true)?;
    let result = receipt
        .parent()
        .ok_or("coverage result parent")?
        .join("report.json");
    store_new(root, &result, &analysis)?;
    Ok(result)
}
pub fn check(root: &Path, path: &Path) -> Result<Analysis> {
    check_report(root, path, true)
}
fn check_report(root: &Path, path: &Path, strict: bool) -> Result<Analysis> {
    let relative = if path.is_absolute() {
        relative(root, path)?
    } else {
        path.to_string_lossy().replace('\\', "/")
    };
    let saved: Analysis = load(root, &relative)?;
    if saved.schema != "imap075-scoped-coverage-v2" || saved.qualification != strict {
        return Err("coverage report is not finalized qualification".into());
    }
    bound(root, &saved.rust_report, MAX)?;
    for report in &saved.java_reports {
        bound(root, report, MAX)?;
    }
    let capture = Path::new(&relative)
        .parent()
        .ok_or("coverage receipt parent")?
        .join("capture.json");
    let fresh = analyze(root, &capture.to_string_lossy().replace('\\', "/"), strict)?;
    compare_analysis(&saved, &fresh)?;
    Ok(fresh)
}
pub fn compare_analysis(saved: &Analysis, fresh: &Analysis) -> Result<()> {
    if saved.schema != fresh.schema
        || saved.qualification != fresh.qualification
        || saved.source_sha != fresh.source_sha
        || saved.run_id != fresh.run_id
        || saved.profile != fresh.profile
        || saved.sources != fresh.sources
        || saved.raw_profiles != fresh.raw_profiles
        || saved.summary != fresh.summary
        || saved.rust_report.sha256 != fresh.rust_report.sha256
        || saved.rust_report.bytes != fresh.rust_report.bytes
        || saved.java_reports != fresh.java_reports
    {
        return Err("archived coverage differs from actual fresh reconstruction".into());
    }
    Ok(())
}
pub fn run(args: Vec<String>, mode: &str) -> Result<()> {
    let root = crate::doc_check::find_repo_root()?;
    if args.len() != 4 || args[0] != "--release" || args[1] != "0.75" {
        return Err("coverage expects --release 0.75 and one explicit path".into());
    }
    match mode {
        "capture" if args[2] == "--output" => {
            let path = capture(&root, Path::new(&args[3]))?;
            println!("imap-coverage: qualified {}", path.display());
        }
        "worker" if args[2] == "--control" => capture_worker(&root, &args[3])?,
        "analysis" if args[2] == "--control" => {
            let (_, directory) = read_control(&root, &args[3])?;
            let marker = directory.join("analysis-started.json");
            store_new(&root, &marker, &identity()?)?;
            let analysis = analyze(
                &root,
                &relative(&root, &directory.join("capture.json"))?,
                false,
            )?;
            store_new(&root, &directory.join("analysis.json"), &analysis)?;
            check_report(&root, &directory.join("analysis.json"), false)?;
            println!(
                "imap-coverage: measurement only; {}/{} changed executable lines",
                analysis.summary.covered_added, analysis.summary.executable_added
            );
        }
        "check" if args[2] == "--receipt" => {
            let analysis = check(&root, Path::new(&args[3]))?;
            println!(
                "imap-coverage-check: {}/{} changed executable lines; {} {}",
                analysis.summary.covered_added,
                analysis.summary.executable_added,
                analysis.profile.os,
                analysis.profile.target.triple
            );
        }
        _ => return Err("unknown/ambiguous coverage command".into()),
    }
    Ok(())
}
