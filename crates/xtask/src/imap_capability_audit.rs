//! Source/contract audit of actual local and legacy SDK capabilities, never production admission.
use hydracache_cluster_testkit::value_plane_capability_075::{
    LocalCapabilityDescriptor, LIFECYCLE_FEATURES,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::Path;

pub struct AuditSources<'a> {
    pub java_descriptor: &'a str,
    pub java_service: &'a str,
    pub rust_service: &'a str,
    pub operation_contract: &'a str,
    pub status: &'a str,
    pub sdk_map: &'a str,
    pub sdk_lock: &'a str,
    pub sdk_properties: &'a str,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args != ["--release", "0.75"] {
        return Err("usage: imap-capability-audit --release 0.75".into());
    }
    let problems = check_at_root(&crate::doc_check::find_repo_root()?, "0.75")?;
    if !problems.is_empty() {
        return Err(problems.join("; ").into());
    }
    println!("imap-capability-audit 0.75: OK (Bounded 21; Unified 40 + 7 typed families; existing SDK 11 tokens; expanded production disabled)");
    Ok(())
}

pub fn check_at_root(root: &Path, release: &str) -> Result<Vec<String>, Box<dyn Error>> {
    if release != "0.75" {
        return Ok(vec!["unsupported capability audit release".into()]);
    }
    let read = |path: &str| -> Result<String, Box<dyn Error>> {
        let path = root.join(path);
        if fs::metadata(&path)?.len() > 2 * 1024 * 1024 {
            return Err("capability audit source exceeds bound".into());
        }
        Ok(fs::read_to_string(path)?)
    };
    let descriptor = read("tests/java-imap-benchmark/src/main/java/io/hydracache/imap/semantic/IMapCapabilityDescriptor.java")?;
    let java = read("tests/java-imap-benchmark/src/main/java/io/hydracache/imap/semantic/UnifiedIMapService.java")?;
    let rust = read("crates/hydracache-cluster-testkit/src/value_plane_service_075.rs")?;
    let contract = read("docs/testing/imap/0.75/operation-contract.json")?;
    let status = read("docs/testing/imap/0.75/status.json")?;
    let map = read(
        "sdks/java/hydracache-hazelcast-facade/src/main/java/io/hydracache/hazelcast/HydraMap.java",
    )?;
    let lock = read("sdks/java/hydracache-hazelcast-facade/src/main/java/io/hydracache/hazelcast/HydraFencedLock.java")?;
    let properties = read("sdks/java/hydracache-hazelcast-facade/src/main/resources/META-INF/hydracache/hazelcast-capabilities.properties")?;
    check_sources(&AuditSources {
        java_descriptor: &descriptor,
        java_service: &java,
        rust_service: &rust,
        operation_contract: &contract,
        status: &status,
        sdk_map: &map,
        sdk_lock: &lock,
        sdk_properties: &properties,
    })
}

pub fn check_sources(s: &AuditSources<'_>) -> Result<Vec<String>, Box<dyn Error>> {
    let descriptor = clean(s.java_descriptor)?;
    let java = clean(s.java_service)?;
    let rust = clean(s.rust_service)?;
    let map = clean(s.sdk_map)?;
    let lock = clean(s.sdk_lock)?;
    check_clean(&AuditSources {
        java_descriptor: &descriptor,
        java_service: &java,
        rust_service: &rust,
        operation_contract: s.operation_contract,
        status: s.status,
        sdk_map: &map,
        sdk_lock: &lock,
        sdk_properties: s.sdk_properties,
    })
}
fn check_clean(s: &AuditSources<'_>) -> Result<Vec<String>, Box<dyn Error>> {
    let contract: Value = serde_json::from_str(s.operation_contract)?;
    let status: Value = serde_json::from_str(s.status)?;
    let mut problems = Vec::new();
    if contract["release"] != "0.75"
        || contract["contract_version"] != 1
        || contract["status"] != "provisional"
        || contract["api_extensions"]["production_enabled"] != false
        || status["release"] != "0.75"
        || status["production_capability"] != "disabled_fail_closed"
    {
        problems.push("expanded 0.75 capability must stay provisional and disabled".into());
    }
    let ids = contract["operations"]
        .as_array()
        .ok_or("missing operations")?
        .iter()
        .map(|op| {
            op["id"]
                .as_str()
                .map(str::to_owned)
                .ok_or("missing operation id")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if ids.len()
        != contract["operations"]
            .as_array()
            .ok_or("missing operations")?
            .len()
    {
        problems.push("duplicate operation contract ID".into());
    }
    let unified = LocalCapabilityDescriptor::unified();
    let bounded = LocalCapabilityDescriptor::bounded();
    let sdk = LocalCapabilityDescriptor::sdk_actual();
    compare(&mut problems, "Unified contract", &ids, &unified.operations);
    let local = body(s.java_descriptor, "IMapCapabilityDescriptor local(")?;
    let calls = set_calls(local);
    if calls.len() < 2 {
        return Err("missing bounded descriptor sets".into());
    }
    compare(&mut problems, "Java planned", &ids, &quotes(calls[0]));
    compare(
        &mut problems,
        "Java Bounded",
        &bounded.operations,
        &quotes(calls[1]),
    );
    compare(
        &mut problems,
        "Java lifecycle",
        &unified.interface_features,
        &quotes(body(s.java_descriptor, "Set<String> lifecycleFeatures()")?),
    );
    let mutations = s
        .java_service
        .split("MUTATIONS = Set.of(")
        .nth(1)
        .ok_or("missing Java mutation set")?
        .split(");")
        .next()
        .ok_or("unterminated mutation set")?;
    let mut dispatch = quotes(mutations);
    for line in body(s.java_service, "private Result read(")?
        .lines()
        .filter(|line| line.trim_start().starts_with("case "))
    {
        dispatch.extend(quotes(line.split("->").next().unwrap_or(line)));
    }
    dispatch.remove("advance");
    compare(&mut problems, "Java dispatch", &ids, &dispatch);
    compare(
        &mut problems,
        "Rust dispatch",
        &ids,
        &quotes(body(s.rust_service, "fn supported_operation(")?),
    );
    let public_map = java_methods(s.sdk_map);
    let public_lock = java_methods(s.sdk_lock);
    compare(
        &mut problems,
        "SDK map public methods",
        &set(&[
            "get",
            "put",
            "replace",
            "remove",
            "addEntryListener",
            "getLock",
            "executeOnKey",
        ]),
        &public_map,
    );
    compare(
        &mut problems,
        "SDK lock public methods",
        &set(&[
            "tryLock",
            "lockAndGetFence",
            "renew",
            "unlock",
            "isLocked",
            "getFence",
            "heldFence",
            "close",
        ]),
        &public_lock,
    );
    if !body(s.sdk_map, "void executeOnKey(")?
        .contains("throw HydraCacheInstance.unsupported(\"IMap.executeOnKey\")")
    {
        problems.push("SDK executeOnKey must fail explicitly".into());
    }
    let mut properties = BTreeMap::new();
    for line in s
        .sdk_properties
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let (key, value) = line
            .split_once('=')
            .ok_or("malformed SDK capability property")?;
        if properties.insert(key, value).is_some() {
            return Err("duplicate SDK capability property".into());
        }
    }
    if properties.get("wireCompatibility") != Some(&"false") {
        problems.push("SDK cannot claim Hazelcast wire compatibility".into());
    }
    compare(
        &mut problems,
        "SDK supported manifest",
        &sdk.operations,
        &tokens(
            properties
                .get("supported")
                .ok_or("missing SDK supported capabilities")?,
        )?,
    );
    let sdk_java = body(s.java_descriptor, "IMapCapabilityDescriptor sdkActual()")?;
    let sdk_sets = set_calls(sdk_java);
    compare(
        &mut problems,
        "Java SDK descriptor",
        &sdk.operations,
        &quotes(sdk_sets.first().ok_or("missing Java SDK operations")?),
    );
    let unsupported = tokens(
        properties
            .get("unsupported")
            .ok_or("missing SDK unsupported capabilities")?,
    )?;
    compare(
        &mut problems,
        "SDK unsupported descriptor",
        &unsupported,
        &quotes(sdk_sets.get(2).ok_or("missing Java SDK unsupported set")?),
    );
    if !unsupported.contains("IMap.executeOnKey") || !unsupported.is_disjoint(&sdk.operations) {
        problems.push("SDK unsupported capabilities are contradictory".into());
    }
    for (feature, method) in LIFECYCLE_FEATURES.iter().zip([
        "retireOutcomes(",
        "resourceUsage()",
        "snapshotListener(",
        "versionBulk(",
        "queryOutcomes(",
        "expireStep(",
        "checkpoint()",
        "entryViews(",
    ]) {
        if !s.java_service.contains(method) {
            problems.push(format!("typed family {feature} has no Java helper"));
        }
    }
    if !s
        .java_descriptor
        .contains("public boolean productionEnabled() { return false; }")
    {
        problems.push("Java expanded production capability is enabled".into());
    }
    Ok(problems)
}

pub(crate) fn clean(source: &str) -> Result<String, Box<dyn Error>> {
    let mut output = String::new();
    let mut chars = source.chars().peekable();
    let mut inside = false;
    let mut escape = false;
    while let Some(c) = chars.next() {
        if inside {
            output.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                inside = false;
            }
        } else if c == '"' {
            inside = true;
            output.push(c);
        } else if c == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for next in chars.by_ref() {
                if next == '\n' {
                    output.push('\n');
                    break;
                }
            }
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut ended = false;
            while let Some(next) = chars.next() {
                if next == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    ended = true;
                    break;
                }
                if next == '\n' {
                    output.push('\n');
                }
            }
            if !ended {
                return Err("unterminated capability source comment".into());
            }
            output.push(' ');
        } else {
            output.push(c);
        }
    }
    if inside {
        return Err("unterminated capability source string".into());
    }
    Ok(output)
}

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).into()).collect()
}
fn compare(
    problems: &mut Vec<String>,
    name: &str,
    expected: &BTreeSet<String>,
    actual: &BTreeSet<String>,
) {
    if actual != expected {
        problems.push(format!(
            "{name} drift: missing {:?}; extra {:?}",
            expected.difference(actual).collect::<Vec<_>>(),
            actual.difference(expected).collect::<Vec<_>>()
        ));
    }
}
fn quotes(source: &str) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    let mut inside = false;
    let mut escape = false;
    let mut value = String::new();
    for character in source.chars() {
        if inside {
            if escape {
                value.push(character);
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == '"' {
                inside = false;
                values.insert(std::mem::take(&mut value));
            } else {
                value.push(character);
            }
        } else if character == '"' {
            inside = true;
        }
    }
    values
}
fn body<'a>(source: &'a str, anchor: &str) -> Result<&'a str, Box<dyn Error>> {
    let tail = source
        .split_once(anchor)
        .ok_or_else(|| format!("missing source anchor {anchor}"))?
        .1;
    let start = tail.find('{').ok_or("missing source body")?;
    let mut depth = 0;
    let mut inside = false;
    let mut escape = false;
    for (index, character) in tail[start..].char_indices() {
        if inside {
            if escape {
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == '"' {
                inside = false;
            }
        } else {
            match character {
                '"' => inside = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(&tail[start + 1..start + index]);
                    }
                }
                _ => {}
            }
        }
    }
    Err("unterminated source body".into())
}
fn set_calls(mut source: &str) -> Vec<&str> {
    let mut result = Vec::new();
    while let Some((_, tail)) = source.split_once("Set.of(") {
        let mut depth = 1;
        let mut inside = false;
        let mut end = None;
        for (index, c) in tail.char_indices() {
            if c == '"' {
                inside = !inside;
            } else if !inside {
                if c == '(' {
                    depth += 1;
                } else if c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(index);
                        break;
                    }
                }
            }
        }
        let Some(end) = end else {
            break;
        };
        result.push(&tail[..end]);
        source = &tail[end + 1..];
    }
    result
}
fn java_methods(source: &str) -> BTreeSet<String> {
    let mut depth: i32 = 0;
    let mut methods = BTreeSet::new();
    for line in source.lines() {
        if depth == 1 {
            if let Some((_, public)) = line.split_once("public ") {
                if let Some((head, _)) = public.split_once('(') {
                    if let Some(name) = head.split_whitespace().last() {
                        methods.insert(name.into());
                    }
                }
            }
        }
        depth += line.matches('{').count() as i32 - line.matches('}').count() as i32;
    }
    methods
}
fn tokens(source: &str) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let mut result = BTreeSet::new();
    let mut depth: i32 = 0;
    let mut start = 0;
    for (index, c) in source
        .char_indices()
        .chain(std::iter::once((source.len(), ',')))
    {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                if index == start || !result.insert(source[start..index].into()) {
                    return Err("empty or duplicate SDK capability token".into());
                }
                start = index + 1;
            }
            _ => {}
        }
        if depth < 0 {
            return Err("malformed SDK capability token".into());
        }
    }
    if depth != 0 {
        return Err("unbalanced SDK capability token".into());
    }
    Ok(result)
}
