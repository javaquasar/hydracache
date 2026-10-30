//! Exact-artifact local v2 receipts. They never authorize production or shipping.
//! Runner identities are checked before and after execution in one build lane.
//! This is not filesystem sealing against a hostile writer replacing and restoring binaries.
use crate::imap_extended_differential_075::{self as peer, ObservationRun};
use hydracache_cluster_testkit::value_plane_extended_075::{
    shrink_extended_bounded, ExtendedCorpus, MAX_BYTES, MAX_OUTPUT,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub const SCHEMA: &str = "hydracache.imap.extended-qualification.v1";
const MAX_RECEIPT: usize = 256 * 1024;
// ELF development executables retain DWARF in the file; Windows keeps PDBs separately.
// Stream hashing/snapshot validation so this finite disk bound is not a heap reservation.
const MAX_BINARY: usize = 512 * 1024 * 1024;
const FAULTS: [&str; 8] = [
    "java_drop_event",
    "java_wrong_ttl",
    "java_reverse_bulk",
    "java_reuse_token",
    "java_wrong_owners",
    "java_wrong_clock",
    "java_wrong_cut",
    "rust_drop_event",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stream {
    pub artifact: Artifact,
    pub normalized_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Canary {
    pub id: String,
    pub observed: Stream,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShrinkProof {
    pub maximum_replays: usize,
    pub maximum_work: usize,
    pub replays: usize,
    pub work: usize,
    pub budget_exhausted: bool,
    pub corpus: Artifact,
    pub expected: Stream,
    pub broken_rust: Stream,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBinding {
    pub rust_fingerprint: String,
    pub java_fingerprint: String,
    pub java_launcher_sha256: String,
    pub process_scope: String,
    pub process_timeout_ms: u64,
    pub process_memory_bytes: u64,
    pub process_count_limit: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtendedReceipt {
    pub schema: String,
    pub release: String,
    pub scope: String,
    pub source_sha: String,
    pub corpus_format: String,
    pub observation_format: String,
    pub seed: i64,
    pub steps: usize,
    pub corpus: Artifact,
    pub rust_peer: Artifact,
    pub java_classpath: String,
    pub java_classes: Vec<Artifact>,
    pub runtime: RuntimeBinding,
    pub expected: Stream,
    pub java_actual: Stream,
    pub rust_actual: Stream,
    pub canaries: Vec<Canary>,
    pub shrink: ShrinkProof,
    pub passed: bool,
    pub production_enabled: bool,
}
fn digest(bytes: &[u8]) -> String {
    crate::imap_qualification_075::digest(bytes)
}
fn hex(value: &str, length: usize) -> bool {
    crate::imap_qualification_075::hex(value, length)
}
fn checked_path(root: &Path, relative: &str) -> Result<PathBuf, Box<dyn Error>> {
    if relative.is_empty()
        || relative.len() > 512
        || relative.contains('\\')
        || relative.contains(':')
        || relative
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
        || Path::new(relative)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("invalid receipt artifact path".into());
    }
    let path = root.join(relative);
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root.canonicalize()?) {
        return Err("artifact escapes receipt root".into());
    }
    Ok(path)
}
pub fn artifact(root: &Path, path: &Path, maximum: usize) -> Result<Artifact, Box<dyn Error>> {
    let relative = path
        .canonicalize()?
        .strip_prefix(root.canonicalize()?)?
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    let resolved = checked_path(root, &relative)?;
    let (bytes, sha256) = file_digest(&resolved, maximum)?;
    Ok(Artifact {
        path: relative,
        bytes,
        sha256,
    })
}
fn file_digest(path: &Path, maximum: usize) -> Result<(u64, String), Box<dyn Error>> {
    let mut input = fs::File::open(path)?;
    let metadata = input.metadata()?;
    if !metadata.is_file() || metadata.len() > maximum as u64 {
        return Err(format!(
            "receipt artifact size/type bound: {} bytes, maximum {maximum}",
            metadata.len()
        )
        .into());
    }
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut block = [0u8; 16 * 1024];
    loop {
        let n = input.read(&mut block)?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        if bytes > maximum as u64 {
            return Err("receipt artifact grew beyond bound".into());
        }
        hash.update(&block[..n]);
    }
    Ok((
        bytes,
        hash.finalize().iter().map(|b| format!("{b:02x}")).collect(),
    ))
}
fn read(root: &Path, reference: &Artifact, maximum: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    verify_artifact(root, reference, maximum)?;
    let path = checked_path(root, &reference.path)?;
    let mut bytes = Vec::new();
    fs::File::open(&path)?
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != reference.bytes || digest(&bytes) != reference.sha256 {
        return Err("artifact changed while reading".into());
    }
    Ok(bytes)
}
fn verify_artifact(
    root: &Path,
    reference: &Artifact,
    maximum: usize,
) -> Result<(), Box<dyn Error>> {
    if !hex(&reference.sha256, 64) || reference.bytes > maximum as u64 {
        return Err("invalid artifact identity/bound".into());
    }
    let path = checked_path(root, &reference.path)?;
    if artifact(root, &path, maximum)? != *reference {
        return Err("artifact digest/size/path mismatch".into());
    }
    Ok(())
}
fn store(root: &Path, path: &Path, bytes: &[u8]) -> Result<Artifact, Box<dyn Error>> {
    if bytes.len() > MAX_OUTPUT {
        return Err("retained evidence bound".into());
    }
    let parent = path.parent().ok_or("missing evidence parent")?;
    if !parent.canonicalize()?.starts_with(root.canonicalize()?) {
        return Err("evidence output escapes root".into());
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    artifact(root, path, MAX_OUTPUT)
}
pub fn normalized(observations: &[Value]) -> Result<String, Box<dyn Error>> {
    struct Bound(Vec<u8>);
    impl Write for Bound {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self
                .0
                .len()
                .checked_add(bytes.len())
                .is_none_or(|n| n > MAX_OUTPUT)
            {
                return Err(std::io::Error::other("normalized transcript byte bound"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    fn value(v: &Value, out: &mut Bound, depth: usize) -> Result<(), Box<dyn Error>> {
        if depth > 12 {
            return Err("normalized observation nesting bound".into());
        }
        match v {
            Value::Array(items) => {
                if items.len() > 256 {
                    return Err("normalized array bound".into());
                }
                out.write_all(b"[")?;
                for (i, item) in items.iter().enumerate() {
                    if i != 0 {
                        out.write_all(b",")?;
                    }
                    value(item, out, depth + 1)?;
                }
                out.write_all(b"]")?;
            }
            Value::Object(map) => {
                if map.len() > 4096 {
                    return Err("normalized object bound".into());
                }
                out.write_all(b"{")?;
                let mut keys = map.keys().collect::<Vec<_>>();
                keys.sort();
                for (i, key) in keys.iter().enumerate() {
                    if i != 0 {
                        out.write_all(b",")?;
                    }
                    serde_json::to_writer(&mut *out, key)?;
                    out.write_all(b":")?;
                    value(&map[*key], out, depth + 1)?;
                }
                out.write_all(b"}")?;
            }
            _ => serde_json::to_writer(out, v)?,
        }
        Ok(())
    }
    if observations.len() > 256 {
        return Err("normalized step bound".into());
    }
    let mut bytes = Bound(Vec::new());
    for observation in observations {
        value(observation, &mut bytes, 0)?;
        bytes.write_all(b"\n")?;
    }
    Ok(digest(&bytes.0))
}
fn retain_run(root: &Path, path: &Path, run: &ObservationRun) -> Result<Stream, Box<dyn Error>> {
    Ok(Stream {
        artifact: store(root, path, &run.transcript)?,
        normalized_sha256: normalized(&run.observations)?,
    })
}
fn classes(root: &Path) -> Result<Vec<Artifact>, Box<dyn Error>> {
    classes_at(root, &root.join("tests/java-imap-benchmark/target/classes"))
}
fn classes_at(root: &Path, base: &Path) -> Result<Vec<Artifact>, Box<dyn Error>> {
    if !base.canonicalize()?.starts_with(root.canonicalize()?) {
        return Err("classpath escapes root".into());
    }
    let mut pending = vec![(base.to_owned(), 0usize)];
    let mut result = Vec::new();
    let mut bytes = 0u64;
    let mut examined = 0usize;
    let mut directories = 0usize;
    while let Some((path, depth)) = pending.pop() {
        directories += 1;
        if depth > 12 || directories > 512 {
            return Err("Java classpath directory/depth bound".into());
        }
        for entry in fs::read_dir(path)? {
            examined += 1;
            if examined > 4096 {
                return Err("classpath inventory work bound".into());
            }
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err("classpath symlink refused".into());
            }
            if kind.is_dir() {
                pending.push((entry.path(), depth + 1));
                if pending.len() > 512 {
                    return Err("classpath directory bound".into());
                }
            } else if entry.path().extension().is_some_and(|e| e == "class") {
                if result.len() == 512 {
                    return Err("classpath class count bound".into());
                }
                let item = artifact(root, &entry.path(), 4 * 1024 * 1024)?;
                bytes += item.bytes;
                if bytes > 16 * 1024 * 1024 {
                    return Err("classpath bytes bound".into());
                }
                result.push(item);
            }
        }
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    if result.is_empty() {
        return Err("missing compiled Java classpath".into());
    }
    Ok(result)
}
fn encode_receipt(receipt: &ExtendedReceipt) -> Result<Vec<u8>, Box<dyn Error>> {
    struct Writer(Vec<u8>);
    impl Write for Writer {
        fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
            if self
                .0
                .len()
                .checked_add(input.len())
                .is_none_or(|n| n > MAX_RECEIPT)
            {
                return Err(std::io::Error::other("receipt byte bound"));
            }
            self.0.extend_from_slice(input);
            Ok(input.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Writer(Vec::new());
    serde_json::to_writer_pretty(&mut out, receipt)?;
    Ok(out.0)
}
fn snapshot(
    root: &Path,
    source: &Artifact,
    path: &Path,
    maximum: usize,
) -> Result<Artifact, Box<dyn Error>> {
    let source_path = checked_path(root, &source.path)?;
    let mut input = fs::File::open(source_path)?;
    let mut output = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut bytes = 0usize;
    let mut block = [0u8; 16 * 1024];
    loop {
        let count = input.read(&mut block)?;
        if count == 0 {
            break;
        }
        bytes = bytes.checked_add(count).ok_or("snapshot byte overflow")?;
        if bytes > maximum {
            return Err("snapshot bytes bound".into());
        }
        output.write_all(&block[..count])?;
    }
    output.sync_all()?;
    let saved = artifact(root, path, maximum)?;
    if saved.bytes != source.bytes || saved.sha256 != source.sha256 {
        return Err("artifact changed during bounded snapshot".into());
    }
    Ok(saved)
}
#[derive(PartialEq, Eq)]
struct ClassIdentity<'a> {
    relative_path: &'a str,
    bytes: u64,
    sha256: &'a str,
}
fn class_identities<'a>(
    files: &'a [Artifact],
    prefix: &str,
) -> Result<Vec<ClassIdentity<'a>>, Box<dyn Error>> {
    let prefix = format!("{prefix}/");
    files
        .iter()
        .map(|a| {
            Ok(ClassIdentity {
                relative_path: a
                    .path
                    .strip_prefix(&prefix)
                    .ok_or("class manifest prefix mismatch")?,
                bytes: a.bytes,
                sha256: a.sha256.as_str(),
            })
        })
        .collect()
}
fn matching_environment(root: &Path, receipt: &ExtendedReceipt) -> Result<bool, Box<dyn Error>> {
    let rust = artifact(root, &peer::rust_peer_path(root)?, MAX_BINARY)?;
    let java = classes(root)?;
    Ok(rust.sha256 == receipt.rust_peer.sha256
        && rust.bytes == receipt.rust_peer.bytes
        && class_identities(&java, "tests/java-imap-benchmark/target/classes")?
            == class_identities(&receipt.java_classes, &receipt.java_classpath)?
        && runtime()? == receipt.runtime)
}
fn new_output(root: &Path, output: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let relative = if output.is_absolute() {
        output.strip_prefix(root)?
    } else {
        output
    };
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
        || relative.components().next() != Some(Component::Normal(std::ffi::OsStr::new("target")))
        || relative.components().count() < 2
    {
        return Err("evidence output must be a new normal path under source target/".into());
    }
    let path = root.join(relative);
    if path.exists() {
        return Err("evidence directory already exists".into());
    }
    let mut ancestor = path.parent().ok_or("evidence parent required")?;
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("missing evidence root")?;
    }
    if !ancestor.canonicalize()?.starts_with(root.canonicalize()?) {
        return Err("evidence ancestor escapes source root".into());
    }
    fs::create_dir_all(path.parent().ok_or("missing evidence parent")?)?;
    if !path
        .parent()
        .ok_or("missing evidence parent")?
        .canonicalize()?
        .starts_with(root.canonicalize()?)
    {
        return Err("evidence parent changed".into());
    }
    fs::create_dir(&path)?;
    Ok(path)
}
fn runtime() -> Result<RuntimeBinding, Box<dyn Error>> {
    fn fingerprint(
        program: impl AsRef<std::ffi::OsStr>,
        args: &[&str],
    ) -> Result<String, Box<dyn Error>> {
        let mut c = std::process::Command::new(program);
        c.args(args);
        let output = crate::imap_proof_process_075::execute(
            c,
            &[],
            crate::imap_proof_process_075::ProcessBounds {
                stdout_bytes: 4096,
                stderr_bytes: 4096,
                ..Default::default()
            },
        )?;
        Ok(String::from_utf8(output.stdout)?.trim().into())
    }
    let java = crate::imap_api_differential::java_executable();
    let (_, launcher) = file_digest(&java, 64 * 1024 * 1024)?;
    let bounds = crate::imap_proof_process_075::ProcessBounds::default();
    Ok(RuntimeBinding {
        rust_fingerprint: fingerprint("rustc", &["--version"])?,
        java_fingerprint: fingerprint(&java, &["-Xmx128m", "--version"])?,
        java_launcher_sha256: launcher,
        process_scope: if cfg!(windows) {
            "windows_job_kill_on_close_process_commit_limit"
        } else {
            "linux_owned_session_nested_groups_per_process_address_space_limit"
        }
        .into(),
        process_timeout_ms: bounds.timeout.as_millis() as u64,
        process_memory_bytes: bounds.memory_bytes as u64,
        process_count_limit: if bounds.maximum_processes == 0 {
            None
        } else {
            Some(bounds.maximum_processes)
        },
    })
}

pub fn produce(root: &Path, output: &Path, seed: i64) -> Result<PathBuf, Box<dyn Error>> {
    let source = crate::imap_ci_075::source_state(root)?;
    let output = new_output(root, output)?;
    let original_rust = artifact(root, &peer::rust_peer_path(root)?, MAX_BINARY)?;
    let original_java = classes(root)?;
    let runtime = runtime()?;
    let saved_artifacts = output.join("artifacts");
    let saved_classes = saved_artifacts.join("classes");
    fs::create_dir_all(&saved_classes)?;
    let rust = snapshot(
        root,
        &original_rust,
        &saved_artifacts.join(if cfg!(windows) { "owner.exe" } else { "owner" }),
        MAX_BINARY,
    )?;
    for source in &original_java {
        let relative = source
            .path
            .strip_prefix("tests/java-imap-benchmark/target/classes/")
            .ok_or("original class prefix")?;
        let target = saved_classes.join(relative);
        fs::create_dir_all(target.parent().ok_or("class snapshot parent")?)?;
        snapshot(root, source, &target, 4 * 1024 * 1024)?;
    }
    let java = classes_at(root, &saved_classes)?;
    let java_classpath = saved_classes
        .canonicalize()?
        .strip_prefix(root.canonicalize()?)?
        .to_string_lossy()
        .replace('\\', "/");
    let corpus = peer::generate(root, seed, 64)?;
    let corpus_ref = store(
        root,
        &output.join("corpus.tsv"),
        corpus.encode()?.as_bytes(),
    )?;
    let expected = peer::execute_java_record(root, &corpus, true, None)?;
    let actual = peer::execute_java_record(root, &corpus, false, None)?;
    let actual_rust = peer::execute_rust_record(root, &corpus, false)?;
    if expected.observations != actual.observations
        || expected.observations != actual_rust.observations
    {
        return Err(
            "actual owners differ from independent model; no passing receipt emitted".into(),
        );
    }
    let expected_ref = retain_run(root, &output.join("expected.tsv"), &expected)?;
    let java_ref = retain_run(root, &output.join("java.tsv"), &actual)?;
    let rust_ref = retain_run(root, &output.join("rust.tsv"), &actual_rust)?;
    let mut canaries = Vec::new();
    for id in FAULTS {
        let run = if let Some(fault) = id.strip_prefix("java_") {
            peer::execute_java_record(root, &corpus, false, Some(fault))?
        } else {
            peer::execute_rust_record(root, &corpus, true)?
        };
        if run.observations == expected.observations {
            return Err(format!("undetected actual process defect: {id}").into());
        }
        canaries.push(Canary {
            id: id.into(),
            observed: retain_run(root, &output.join(format!("{id}.tsv")), &run)?,
        });
    }
    let shrunk = shrink_extended_bounded(corpus.clone(), 16, 4096, |candidate| {
        let Ok(e) = peer::execute_java_record(root, candidate, true, None) else {
            return Ok(false);
        };
        let Ok(r) = peer::execute_rust_record(root, candidate, true) else {
            return Ok(false);
        };
        Ok(e.observations != r.observations)
    })?;
    let e = peer::execute_java_record(root, &shrunk.corpus, true, None)?;
    let r = peer::execute_rust_record(root, &shrunk.corpus, true)?;
    let shrink = ShrinkProof {
        maximum_replays: 16,
        maximum_work: 4096,
        replays: shrunk.replays,
        work: shrunk.work,
        budget_exhausted: shrunk.budget_exhausted,
        corpus: store(
            root,
            &output.join("minimized.tsv"),
            shrunk.corpus.encode()?.as_bytes(),
        )?,
        expected: retain_run(root, &output.join("minimized-expected.tsv"), &e)?,
        broken_rust: retain_run(root, &output.join("minimized-rust.tsv"), &r)?,
    };
    if crate::imap_ci_075::source_state(root)? != source
        || artifact(root, &peer::rust_peer_path(root)?, MAX_BINARY)? != original_rust
        || classes(root)? != original_java
        || self::runtime()? != runtime
    {
        return Err("source/runtime/runner artifacts changed during qualification".into());
    }
    let receipt = ExtendedReceipt {
        schema: SCHEMA.into(),
        release: "0.75".into(),
        scope: "local_actual_owner_qualification".into(),
        source_sha: source,
        corpus_format: "imap-extended-semantic-v2".into(),
        observation_format: "imap-extended-observations-v2".into(),
        seed,
        steps: corpus.steps.len(),
        corpus: corpus_ref,
        rust_peer: rust,
        java_classpath,
        java_classes: java,
        runtime,
        expected: expected_ref,
        java_actual: java_ref,
        rust_actual: rust_ref,
        canaries,
        shrink,
        passed: true,
        production_enabled: false,
    };
    validate(root, &receipt)?;
    let path = output.join("receipt.json");
    store(root, &path, &encode_receipt(&receipt)?)?;
    Ok(path)
}

pub fn validate(root: &Path, receipt: &ExtendedReceipt) -> Result<(), Box<dyn Error>> {
    let encoded = encode_receipt(receipt)?;
    let schema: Value = serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/schemas/extended-qualification.schema.json"
    ))?;
    let validator = jsonschema::validator_for(&schema)
        .map_err(|error| format!("extended receipt schema: {error}"))?;
    if !validator.is_valid(&serde_json::from_slice::<Value>(&encoded)?) {
        return Err("extended receipt schema validation failed".into());
    }
    if receipt.schema != SCHEMA
        || receipt.release != "0.75"
        || receipt.scope != "local_actual_owner_qualification"
        || !hex(&receipt.source_sha, 40)
        || !receipt.passed
        || receipt.production_enabled
        || receipt.corpus_format != "imap-extended-semantic-v2"
        || receipt.observation_format != "imap-extended-observations-v2"
        || receipt.steps == 0
        || receipt.steps > 256
    {
        return Err("unsupported or inadmissible extended receipt".into());
    }
    let corpus_bytes = read(root, &receipt.corpus, MAX_BYTES)?;
    let corpus = ExtendedCorpus::parse(std::str::from_utf8(&corpus_bytes)?)?;
    if corpus.encode()?.as_bytes() != corpus_bytes
        || corpus.seed != receipt.seed
        || corpus.steps.len() != receipt.steps
    {
        return Err("corpus canonical identity/seed/step mismatch".into());
    }
    let check =
        |s: &Stream, c: &ExtendedCorpus, expected: bool| -> Result<String, Box<dyn Error>> {
            let rows =
                peer::parse_observations(&read(root, &s.artifact, MAX_OUTPUT)?, c, expected)?;
            let hash = normalized(&rows)?;
            if hash != s.normalized_sha256 {
                return Err("normalized stream digest mismatch".into());
            }
            Ok(hash)
        };
    let e = check(&receipt.expected, &corpus, true)?;
    if check(&receipt.java_actual, &corpus, false)? != e
        || check(&receipt.rust_actual, &corpus, false)? != e
    {
        return Err("independent expected/actual streams differ".into());
    }
    let mut paths = BTreeSet::new();
    for item in [
        &receipt.corpus,
        &receipt.expected.artifact,
        &receipt.java_actual.artifact,
        &receipt.rust_actual.artifact,
    ] {
        if !paths.insert(&item.path) {
            return Err("aliased semantic proof artifacts".into());
        }
    }
    let mut ids = BTreeSet::new();
    if receipt.canaries.len() != 8 {
        return Err("missing process defect controls".into());
    }
    for canary in &receipt.canaries {
        if !FAULTS.contains(&canary.id.as_str())
            || !ids.insert(&canary.id)
            || !paths.insert(&canary.observed.artifact.path)
            || check(&canary.observed, &corpus, false)? == e
        {
            return Err("missing/aliased/undetected process defect control".into());
        }
    }
    let s = &receipt.shrink;
    if s.maximum_replays != 16
        || s.maximum_work != 4096
        || s.replays == 0
        || s.replays > 16
        || s.work < corpus.steps.len()
        || s.work > 4096
    {
        return Err("invalid shrink replay/work accounting".into());
    }
    for item in [&s.corpus, &s.expected.artifact, &s.broken_rust.artifact] {
        if !paths.insert(&item.path) {
            return Err("aliased shrink artifacts".into());
        }
    }
    let reduced = ExtendedCorpus::parse(std::str::from_utf8(&read(root, &s.corpus, MAX_BYTES)?)?)?;
    if reduced.seed != corpus.seed
        || reduced.steps.len() > corpus.steps.len()
        || check(&s.expected, &reduced, true)? == check(&s.broken_rust, &reduced, false)?
    {
        return Err("minimized process defect does not reproduce".into());
    }
    let mut cursor = 0;
    for step in &reduced.steps {
        let Some(index) = corpus.steps[cursor..]
            .iter()
            .position(|original| original == step)
        else {
            return Err("shrink changed step identity/payload rather than deleting".into());
        };
        cursor += index + 1;
    }
    verify_artifact(root, &receipt.rust_peer, MAX_BINARY)?;
    checked_path(root, &receipt.java_classpath)?;
    if receipt.java_classes.is_empty() || receipt.java_classes.len() > 512 {
        return Err("Java class manifest bound".into());
    }
    let class_prefix = format!("{}/", receipt.java_classpath);
    let mut previous = None;
    let mut class_bytes = 0;
    for class in &receipt.java_classes {
        if !class.path.starts_with(&class_prefix)
            || !class.path.ends_with(".class")
            || previous.is_some_and(|p: &String| p >= &class.path)
        {
            return Err("Java class manifest path/order".into());
        }
        read(root, class, 4 * 1024 * 1024)?;
        class_bytes += class.bytes;
        if class_bytes > 16 * 1024 * 1024 {
            return Err("Java class manifest bytes".into());
        }
        previous = Some(&class.path);
    }
    if classes_at(root, &root.join(&receipt.java_classpath))? != receipt.java_classes {
        return Err("unrecorded classpath artifact".into());
    }
    let r = &receipt.runtime;
    if r.rust_fingerprint.is_empty()
        || r.rust_fingerprint.len() > 4096
        || r.java_fingerprint.is_empty()
        || r.java_fingerprint.len() > 4096
        || !hex(&r.java_launcher_sha256, 64)
        || !matches!(
            r.process_scope.as_str(),
            "windows_job_kill_on_close_process_commit_limit"
                | "linux_owned_session_nested_groups_per_process_address_space_limit"
        )
        || r.process_timeout_ms != 15_000
        || r.process_memory_bytes == 0
        || (r.process_scope.starts_with("windows")
            && !r.process_count_limit.is_some_and(|n| (1..=32).contains(&n)))
        || (r.process_scope.starts_with("linux") && r.process_count_limit.is_some())
    {
        return Err("invalid runtime envelope identity".into());
    }
    Ok(())
}

pub fn verify(root: &Path, path: &Path) -> Result<ExtendedReceipt, Box<dyn Error>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_RECEIPT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RECEIPT {
        return Err("receipt byte bound".into());
    }
    let receipt: ExtendedReceipt = serde_json::from_slice(&bytes)?;
    validate(root, &receipt)?;
    if crate::imap_ci_075::source_state(root)? != receipt.source_sha
        || !matching_environment(root, &receipt)?
    {
        return Err(
            "receipt source/runner/runtime differs from current execution environment".into(),
        );
    }
    let corpus = ExtendedCorpus::parse(std::str::from_utf8(&read(
        root,
        &receipt.corpus,
        MAX_BYTES,
    )?)?)?;
    if peer::generate(root, receipt.seed, receipt.steps)? != corpus {
        return Err("receipt corpus does not match its bound generated seed".into());
    }
    if normalized(&peer::execute_java_record(root, &corpus, true, None)?.observations)?
        != receipt.expected.normalized_sha256
        || normalized(&peer::execute_java_record(root, &corpus, false, None)?.observations)?
            != receipt.java_actual.normalized_sha256
        || normalized(&peer::execute_rust_record(root, &corpus, false)?.observations)?
            != receipt.rust_actual.normalized_sha256
    {
        return Err("bound actual artifact replay differs from retained proof".into());
    }
    for canary in &receipt.canaries {
        let run = if let Some(fault) = canary.id.strip_prefix("java_") {
            peer::execute_java_record(root, &corpus, false, Some(fault))?
        } else {
            peer::execute_rust_record(root, &corpus, true)?
        };
        if normalized(&run.observations)? != canary.observed.normalized_sha256 {
            return Err("bound process defect replay differs".into());
        }
    }
    let reduced = shrink_extended_bounded(corpus.clone(), 16, 4096, |candidate| {
        let Ok(e) = peer::execute_java_record(root, candidate, true, None) else {
            return Ok(false);
        };
        let Ok(r) = peer::execute_rust_record(root, candidate, true) else {
            return Ok(false);
        };
        Ok(e.observations != r.observations)
    })?;
    let retained = ExtendedCorpus::parse(std::str::from_utf8(&read(
        root,
        &receipt.shrink.corpus,
        MAX_BYTES,
    )?)?)?;
    if reduced.corpus != retained
        || reduced.replays != receipt.shrink.replays
        || reduced.work != receipt.shrink.work
        || reduced.budget_exhausted != receipt.shrink.budget_exhausted
        || normalized(&peer::execute_java_record(root, &retained, true, None)?.observations)?
            != receipt.shrink.expected.normalized_sha256
        || normalized(&peer::execute_rust_record(root, &retained, true)?.observations)?
            != receipt.shrink.broken_rust.normalized_sha256
    {
        return Err("bound shrink result/consumption replay differs".into());
    }
    if crate::imap_ci_075::source_state(root)? != receipt.source_sha
        || !matching_environment(root, &receipt)?
    {
        return Err("source/runner/runtime changed during evidence replay".into());
    }
    validate(root, &receipt)?;
    Ok(receipt)
}

pub fn run(args: Vec<String>, check: bool) -> Result<(), Box<dyn Error>> {
    let mut fields = std::collections::BTreeMap::new();
    let mut pairs = args.chunks_exact(2);
    for pair in &mut pairs {
        if !matches!(
            pair[0].as_str(),
            "--release" | "--seed" | "--output" | "--receipt"
        ) || fields.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unknown/duplicate extended evidence option".into());
        }
    }
    if !pairs.remainder().is_empty() || fields.remove("--release") != Some("0.75") {
        return Err("extended evidence requires --release 0.75".into());
    }
    let root = crate::doc_check::find_repo_root()?;
    if check {
        let file = fields.remove("--receipt").ok_or("--receipt required")?;
        if !fields.is_empty() {
            return Err("incompatible evidence check options".into());
        }
        let receipt = verify(&root, Path::new(file))?;
        println!("imap-extended-evidence-check: OK ({} steps, exact-artifact local scope; production disabled)",receipt.steps);
    } else {
        let output = fields.remove("--output").ok_or("--output required")?;
        let seed = fields.remove("--seed").unwrap_or("117");
        let parsed = seed.parse::<i64>()?;
        if parsed.to_string() != seed || !fields.is_empty() {
            return Err("invalid evidence seed/options".into());
        }
        println!(
            "imap-extended-evidence: {}",
            produce(&root, Path::new(output), parsed)?.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classpath_work_bounds_include_non_class_files_and_directory_fanout() {
        let fixture = tempfile::tempdir().unwrap();
        let base = fixture.path().join("classes");
        fs::create_dir(&base).unwrap();
        fs::write(base.join("Owner.class"), b"class").unwrap();
        for index in 0..4096 {
            fs::write(base.join(format!("ignored-{index}")), b"").unwrap();
        }
        assert!(classes_at(fixture.path(), &base)
            .unwrap_err()
            .to_string()
            .contains("work bound"));
        let dirs = fixture.path().join("directories");
        fs::create_dir(&dirs).unwrap();
        for index in 0..513 {
            fs::create_dir(dirs.join(index.to_string())).unwrap();
        }
        assert!(classes_at(fixture.path(), &dirs).is_err());
    }

    #[test]
    fn evidence_outputs_are_new_target_descendants_and_snapshots_detect_changed_sources() {
        let fixture = tempfile::tempdir().unwrap();
        for path in ["outside", "target", "target/../escape"] {
            assert!(new_output(fixture.path(), Path::new(path)).is_err());
        }
        let output = new_output(fixture.path(), Path::new("target/a/proof")).unwrap();
        assert!(new_output(fixture.path(), &output).is_err());
        let source = output.join("source");
        fs::write(&source, b"original").unwrap();
        let identity = artifact(fixture.path(), &source, 100).unwrap();
        let retained = snapshot(fixture.path(), &identity, &output.join("retained"), 100).unwrap();
        assert_eq!(identity.sha256, retained.sha256);
        fs::write(&source, b"changed").unwrap();
        assert!(snapshot(fixture.path(), &identity, &output.join("changed"), 100).is_err());
        assert!(snapshot(fixture.path(), &identity, &output.join("bounded"), 1).is_err());
    }

    #[test]
    fn development_elf_size_budget_is_streamed_and_remains_finite() {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("development-owner");
        let file = fs::File::create(&path).unwrap();
        file.set_len(64 * 1024 * 1024 + 1).unwrap();
        drop(file);
        assert!(artifact(fixture.path(), &path, 64 * 1024 * 1024).is_err());
        let retained = artifact(fixture.path(), &path, MAX_BINARY).unwrap();
        verify_artifact(fixture.path(), &retained, MAX_BINARY).unwrap();
        assert_eq!(retained.bytes, 64 * 1024 * 1024 + 1);
        let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len(MAX_BINARY as u64 + 1).unwrap();
        drop(file);
        assert!(artifact(fixture.path(), &path, MAX_BINARY).is_err());
        assert!(verify_artifact(fixture.path(), &retained, MAX_BINARY).is_err());
    }
}
