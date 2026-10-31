//! Shared bounded v2 corpus: two actual owners checked against Java's independent model.
use hydracache_cluster_testkit::value_plane_differential_075::hex;
use hydracache_cluster_testkit::value_plane_extended_075::{
    shrink_extended, ExtendedCorpus, MAX_BYTES, MAX_LINE, MAX_OUTPUT,
};
use serde_json::Value;
use std::error::Error;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedReport {
    pub seed: i64,
    pub steps: usize,
    pub process_canaries: usize,
}

#[derive(Debug)]
pub struct ObservationRun {
    pub observations: Vec<Value>,
    pub transcript: Vec<u8>,
}

pub fn generate(root: &Path, seed: i64, steps: usize) -> Result<ExtendedCorpus, Box<dyn Error>> {
    if steps == 0 || steps > 256 {
        return Err("extended generation step bound".into());
    }
    let seed = seed.to_string();
    let steps = steps.to_string();
    let output = crate::imap_api_differential::execute_java_bounded(
        root,
        "ExtendedSemanticProcessRunner",
        String::new(),
        &["--v2-generate", &seed, &steps],
        MAX_BYTES,
    )?;
    Ok(ExtendedCorpus::parse(std::str::from_utf8(&output)?)?)
}

pub fn execute_java(
    root: &Path,
    corpus: &ExtendedCorpus,
    expected: bool,
    fault: Option<&str>,
) -> Result<Vec<Value>, Box<dyn Error>> {
    Ok(execute_java_record(root, corpus, expected, fault)?.observations)
}

pub fn execute_java_record(
    root: &Path,
    corpus: &ExtendedCorpus,
    expected: bool,
    fault: Option<&str>,
) -> Result<ObservationRun, Box<dyn Error>> {
    if expected && fault.is_some() {
        return Err("oracle cannot inject actual-owner faults".into());
    }
    let mut args = vec!["--v2-script", "-"];
    if expected {
        args.push("--v2-expected");
    }
    if let Some(fault) = fault {
        args.extend(["--v2-fault", fault]);
    }
    let output = crate::imap_api_differential::execute_java_bounded(
        root,
        "ExtendedSemanticProcessRunner",
        corpus.encode()?,
        &args,
        MAX_OUTPUT,
    )?;
    let observations = parse_observations(&output, corpus, expected)?;
    Ok(ObservationRun {
        observations,
        transcript: output,
    })
}

pub fn execute_rust_process(
    root: &Path,
    corpus: &ExtendedCorpus,
    fault: bool,
) -> Result<Vec<Value>, Box<dyn Error>> {
    Ok(execute_rust_record(root, corpus, fault)?.observations)
}

pub(crate) fn rust_peer_path(root: &Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        root.join(target)
    };
    let path = target.join("debug").join(if cfg!(windows) {
        "imap-extended-owner-075.exe"
    } else {
        "imap-extended-owner-075"
    });
    if !path.is_file() {
        return Err("compiled actual Rust v2 owner peer is missing; build testkit integration targets first".into());
    }
    Ok(path)
}

pub fn execute_rust_record(
    root: &Path,
    corpus: &ExtendedCorpus,
    fault: bool,
) -> Result<ObservationRun, Box<dyn Error>> {
    let path = rust_peer_path(root)?;
    let mut command = std::process::Command::new(path);
    command.current_dir(root);
    if fault {
        command.arg("--canary-drop-event");
    }
    let output = crate::imap_api_differential::execute_process_bounded(
        command,
        corpus.encode()?,
        MAX_OUTPUT,
    )?;
    let observations = parse_observations(&output, corpus, false)?;
    Ok(ObservationRun {
        observations,
        transcript: output,
    })
}

/// Closed schemas and fixed depth reject partial, recursive or unrelated process output.
pub fn parse_observations(
    output: &[u8],
    corpus: &ExtendedCorpus,
    expected: bool,
) -> Result<Vec<Value>, Box<dyn Error>> {
    corpus.validate()?;
    if output.len() > MAX_OUTPUT || output.contains(&b'\r') {
        return Err("invalid extended transcript encoding/bound".into());
    }
    let source = std::str::from_utf8(output)?;
    let mut lines = source.strip_suffix('\n').unwrap_or(source).split('\n');
    if lines.next()
        != Some(if expected {
            "imap-extended-observations-v2\texpected"
        } else {
            "imap-extended-observations-v2\tactual"
        })
    {
        return Err("extended transcript mode/header mismatch".into());
    }
    let mut result = Vec::with_capacity(corpus.steps.len());
    for step in &corpus.steps {
        let line = lines.next().ok_or("missing extended observation")?;
        if line.len() > MAX_LINE + 193 {
            return Err("extended observation line bound".into());
        }
        let (id, json) = line.split_once('\t').ok_or("extended observation shape")?;
        if id != hex(step.id.as_bytes()) || json.len() > MAX_LINE {
            return Err("extended observation ID/bound mismatch".into());
        }
        // The recursively checked tree has only the admitted, nonrecursive result shape.
        let UniqueValue(value) = serde_json::from_str(json)?;
        validate_observation(&value)?;
        result.push(value);
    }
    if lines.next().is_some() {
        return Err("extra extended observations".into());
    }
    Ok(result)
}

struct UniqueValue(Value);
impl<'de> serde::Deserialize<'de> for UniqueValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> serde::de::Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON without duplicate fields")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Err(E::custom("floating point observation"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(UniqueValue(value)) = seq.next_element()? {
                    if values.len() == 65 {
                        return Err(serde::de::Error::custom("array bound"));
                    }
                    values.push(value);
                }
                Ok(UniqueValue(values.into()))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, UniqueValue(value))) =
                    map.next_entry::<String, UniqueValue>()?
                {
                    if values.len() == 4096 || values.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("duplicate field/object bound"));
                    }
                }
                Ok(UniqueValue(values.into()))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

fn fields(value: &Value, names: &[&str]) -> Result<(), Box<dyn Error>> {
    let object = value.as_object().ok_or("expected extended JSON object")?;
    if object.len() != names.len() || names.iter().any(|name| !object.contains_key(*name)) {
        return Err("unknown/missing extended JSON fields".into());
    }
    Ok(())
}
fn integer(v: &Value) -> Result<(), Box<dyn Error>> {
    if v.as_u64().is_none_or(|n| n > i64::MAX as u64) {
        Err("invalid common integer".into())
    } else {
        Ok(())
    }
}
fn binary(v: &Value, optional: bool, max: usize) -> Result<(), Box<dyn Error>> {
    if optional && v.is_null() {
        return Ok(());
    }
    let s = v.as_str().ok_or("expected hex bytes")?;
    if s.len() > max * 2
        || !s.len().is_multiple_of(2)
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("invalid observation bytes".into());
    }
    Ok(())
}
fn string(v: &Value) -> Result<(), Box<dyn Error>> {
    if v.as_str().is_none_or(|s|s.len()>96||s.contains('=')||s.chars().all(|c|matches!(c,'\u{0009}'..='\u{000d}'|'\u{001c}'..='\u{0020}'|'\u{1680}'|'\u{2000}'..='\u{2006}'|'\u{2008}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{205f}'|'\u{3000}'))){Err("invalid observation text".into())}else{Ok(())}
}
fn array(v: &Value) -> Result<&Vec<Value>, Box<dyn Error>> {
    let a = v.as_array().ok_or("expected array")?;
    if a.len() > 64 {
        return Err("observation item bound".into());
    }
    Ok(a)
}
fn ttl(v: &Value) -> Result<(), Box<dyn Error>> {
    fields(v, &["kind", "remaining_ticks"])?;
    integer(&v["remaining_ticks"])?;
    match v["kind"].as_str() {
        Some("absent" | "eternal") if v["remaining_ticks"] == 0 => Ok(()),
        Some("expiring") if v["remaining_ticks"].as_u64().is_some_and(|n| n > 0) => Ok(()),
        _ => Err("invalid TTL state".into()),
    }
}
fn validate_result(v: &Value, retained: bool) -> Result<(), Box<dyn Error>> {
    fields(v, &["kind", "value", "ttl", "items", "statuses", "number"])?;
    string(&v["kind"])?;
    binary(&v["value"], true, 4096)?;
    ttl(&v["ttl"])?;
    integer(&v["number"])?;
    for i in array(&v["items"])? {
        fields(i, &["index", "key", "kind", "value", "ttl"])?;
        integer(&i["index"])?;
        if i["index"].as_u64().is_none_or(|n| n > i32::MAX as u64) {
            return Err("item index bound".into());
        }
        binary(&i["key"], false, 1024)?;
        string(&i["kind"])?;
        binary(&i["value"], true, 4096)?;
        ttl(&i["ttl"])?;
    }
    let statuses = array(&v["statuses"])?;
    if retained && !statuses.is_empty() {
        return Err("recursive retained status".into());
    }
    for s in statuses {
        fields(s, &["identity", "kind", "result"])?;
        fields(&s["identity"], &["client", "sequence"])?;
        string(&s["identity"]["client"])?;
        integer(&s["identity"]["sequence"])?;
        if s["identity"]["sequence"] == 0 {
            return Err("zero sequence".into());
        }
        string(&s["kind"])?;
        if !s["result"].is_null() {
            validate_result(&s["result"], true)?;
        }
    }
    Ok(())
}
fn validate_observation(v: &Value) -> Result<(), Box<dyn Error>> {
    fields(
        v,
        &[
            "result",
            "snapshot",
            "events",
            "token_relations",
            "owners",
            "tick",
        ],
    )?;
    validate_result(&v["result"], false)?;
    integer(&v["tick"])?;
    for row in array(&v["snapshot"])? {
        fields(row, &["key", "value", "ttl"])?;
        binary(&row["key"], false, 1024)?;
        binary(&row["value"], false, 4096)?;
        ttl(&row["ttl"])?;
    }
    let events = v["events"].as_array().ok_or("expected events array")?;
    if events.len() > 65 {
        return Err("event bound".into());
    }
    for e in events {
        fields(e, &["kind", "key", "old_value", "value", "watermark"])?;
        string(&e["kind"])?;
        binary(&e["key"], false, 1024)?;
        binary(&e["old_value"], true, 4096)?;
        binary(&e["value"], true, 4096)?;
        integer(&e["watermark"])?;
        if e["watermark"] == 0 {
            return Err("zero event watermark".into());
        }
    }
    fields(
        &v["owners"],
        &["outcomes", "pending", "listeners", "snapshot_payload"],
    )?;
    for k in ["outcomes", "pending", "listeners"] {
        integer(&v["owners"][k])?;
    }
    if !v["owners"]["snapshot_payload"].is_boolean() {
        return Err("invalid snapshot ownership flag".into());
    }
    let relations = v["token_relations"]
        .as_object()
        .ok_or("expected token relations")?;
    if relations.len() > 4096
        || relations
            .iter()
            .any(|(k, v)| k.len() > 193 || !k.contains('=') || !v.is_boolean())
    {
        return Err("invalid token relation bound/shape".into());
    }
    Ok(())
}

pub fn check_corpus(root: &Path, corpus: &ExtendedCorpus) -> Result<(), Box<dyn Error>> {
    let original = retain_corpus(root, corpus)?;
    let contextual = |error: Box<dyn Error>| {
        format!(
            "{error}; original corpus retained at {}",
            original.display()
        )
    };
    let expected = execute_java(root, corpus, true, None).map_err(&contextual)?;
    let actual = execute_java(root, corpus, false, None).map_err(&contextual)?;
    let rust = execute_rust_process(root, corpus, false).map_err(&contextual)?;
    if actual != expected || rust != expected {
        let minimized = shrink_extended(corpus.clone(), 16, |candidate| {
            // Removing a write may leave a captured token absent; such a replay is
            // not a valid semantic reduction even though its DTO references exist.
            let Ok(e) = execute_java(root, candidate, true, None) else {
                return Ok(false);
            };
            let Ok(j) = execute_java(root, candidate, false, None) else {
                return Ok(false);
            };
            let Ok(r) = execute_rust_process(root, candidate, false) else {
                return Ok(false);
            };
            Ok(r != e || j != e)
        })
        .map_err(|e| format!("{e}; original corpus retained at {}", original.display()))?;
        let path = retain_corpus(root, &minimized)?;
        let index = expected
            .iter()
            .zip(actual.iter().zip(&rust))
            .position(|(e, (j, r))| e != j || e != r)
            .unwrap_or(0);
        return Err(format!("extended actual-owner divergence seed={} step={} id={}; bounded replay retained at {}\nexpected={}\nRust={}\nJava={}",corpus.seed,index,corpus.steps[index].id,path.display(),preview(&expected[index]),preview(&rust[index]),preview(&actual[index])).into());
    }
    Ok(())
}
fn preview(v: &Value) -> String {
    v.to_string().chars().take(2048).collect()
}
pub fn retain_corpus(
    root: &Path,
    corpus: &ExtendedCorpus,
) -> Result<std::path::PathBuf, Box<dyn Error>> {
    crate::imap_api_differential::retain_source(
        root,
        &corpus.encode()?,
        &format!("extended-seed-{}", corpus.seed),
    )
}

pub fn check_actual(root: &Path, seed: i64) -> Result<ExtendedReport, Box<dyn Error>> {
    let corpus = generate(root, seed, 64)?;
    check_corpus(root, &corpus)?;
    for name in [
        "signed-point",
        "reserved-predeadline",
        "reserved-matching",
        "reserved-expired",
        "signed-bulk",
        "identifier-clients",
    ] {
        let source = std::fs::read_to_string(root.join(format!(
            "tests/java-imap-benchmark/src/test/resources/imap-v2-boundaries/{name}.tsv"
        )))?;
        check_corpus(root, &ExtendedCorpus::parse(&source)?)?;
    }
    let expected = execute_java(root, &corpus, true, None)?;
    let faults = [
        "drop_event",
        "wrong_ttl",
        "reverse_bulk",
        "reuse_token",
        "wrong_owners",
        "wrong_clock",
        "wrong_cut",
    ];
    for fault in faults {
        if execute_java(root, &corpus, false, Some(fault))? == expected {
            return Err(format!("real extended process canary escaped: {fault}").into());
        }
    }
    if execute_rust_process(root, &corpus, true)? == expected {
        return Err("real Rust v2 process canary escaped".into());
    }
    // Exercise replay/shrink/retention with a genuinely broken executable peer.
    let reduced = shrink_extended(corpus.clone(), 16, |candidate| {
        let Ok(e) = execute_java(root, candidate, true, None) else {
            return Ok(false);
        };
        let Ok(r) = execute_rust_process(root, candidate, true) else {
            return Ok(false);
        };
        Ok(r != e)
    })?;
    let path = retain_corpus(root, &reduced)?;
    let replay = ExtendedCorpus::parse(&std::fs::read_to_string(path)?)?;
    if execute_rust_process(root, &replay, true)? == execute_java(root, &replay, true, None)? {
        return Err("replayed Rust v2 process canary escaped after bounded shrink".into());
    }
    Ok(ExtendedReport {
        seed,
        steps: corpus.steps.len(),
        process_canaries: faults.len() + 1,
    })
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let root = crate::doc_check::find_repo_root()?;
    let mut release = None;
    let mut seed = 117;
    let mut script = None;
    let mut seen = std::collections::BTreeSet::new();
    let mut chunks = args.chunks_exact(2);
    for pair in &mut chunks {
        if !seen.insert(pair[0].as_str()) {
            return Err("duplicate extended option".into());
        }
        match pair[0].as_str() {
            "--release" => release = Some(pair[1].as_str()),
            "--seed" => {
                seed = pair[1].parse()?;
                if seed.to_string() != pair[1] {
                    return Err("noncanonical seed".into());
                }
            }
            "--script" => script = Some(&pair[1]),
            _ => return Err("unknown extended option".into()),
        }
    }
    if release != Some("0.75") || !chunks.remainder().is_empty() {
        return Err("extended differential requires --release 0.75 and valued options".into());
    }
    if script.is_some() && seen.contains("--seed") {
        return Err("choose either a generated seed or a retained script".into());
    }
    let replay = if let Some(path) = script {
        use std::io::Read;
        let mut input = String::new();
        std::fs::File::open(path)?
            .take(MAX_BYTES as u64 + 1)
            .read_to_string(&mut input)?;
        Some(ExtendedCorpus::parse(&input)?)
    } else {
        None
    };
    crate::imap_api_differential::prepare_java_at_root(&root)?;
    if let Some(corpus) = replay {
        check_corpus(&root, &corpus)?;
        println!("imap-extended-differential 0.75: replay OK");
    } else {
        println!(
            "imap-extended-differential 0.75: {:?}",
            check_actual(&root, seed)?
        );
    }
    Ok(())
}
