//! Bounded v2 development corpus shared with the independent Java semantic oracle.
//! No production protocol, SDK, or durable numeric identity is defined here.

use std::collections::BTreeSet;

use crate::value_plane_differential_075::{hex, DifferentialError};
use crate::value_plane_model_075::MutationIdentity;
use crate::value_plane_service_075::ServiceTtl;

#[path = "value_plane_extended_owner_075.rs"]
mod owner;
pub use owner::{execute_extended, execute_extended_drop_event};

pub const HEADER: &str = "imap-extended-semantic-v2";
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_LINE: usize = 1024 * 1024;
pub const MAX_OUTPUT: usize = 16 * 1024 * 1024;
pub const MAX_STEPS: usize = 256;
const OPS: &[&str] = &[
    "put",
    "capture_version",
    "replace_if_version",
    "remove_if_version",
    "set_ttl_if_version",
    "bulk_replace",
    "bulk_remove",
    "bulk_set_ttl",
    "query_outcomes",
    "retire_outcomes",
    "listener_resync",
    "hold_listener_snapshot",
    "close_listener_snapshot",
    "checkpoint_restore",
    "advance",
    "evict",
    "clear",
    "destroy",
    "signal_gap",
    "resources",
    "reserve_put",
    "dispatch_put",
    "cancel_reserved",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedEntry {
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
    pub label: Option<String>,
    pub ttl: ServiceTtl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedStep {
    pub id: String,
    pub operation: String,
    pub entries: Vec<ExtendedEntry>,
    pub requests: Vec<MutationIdentity>,
    pub limits: Option<(usize, usize)>,
    pub ticks: u64,
    pub lose_response: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedCorpus {
    pub seed: i64,
    pub steps: Vec<ExtendedStep>,
}

fn bad(message: &str) -> DifferentialError {
    DifferentialError(message.into())
}

pub(crate) fn number(raw: &str) -> Result<u64, DifferentialError> {
    if raw.is_empty()
        || (raw.len() > 1 && raw.starts_with('0'))
        || !raw.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(bad("noncanonical integer"));
    }
    let value = raw.parse::<u64>().map_err(|_| bad("integer overflow"))?;
    if value > i64::MAX as u64 {
        return Err(bad("integer exceeds common signed range"));
    }
    Ok(value)
}

fn bytes(raw: &str, maximum: usize) -> Result<Vec<u8>, DifferentialError> {
    if raw.len() > maximum * 2
        || !raw.len().is_multiple_of(2)
        || !raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(bad("invalid or oversized canonical hex"));
    }
    raw.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
            Ok(digit(pair[0]) * 16 + digit(pair[1]))
        })
        .collect()
}

fn text(raw: &str) -> Result<String, DifferentialError> {
    let value = String::from_utf8(bytes(raw, 96)?).map_err(|_| bad("invalid UTF8 text"))?;
    if value.chars().all(|c| matches!(c,'\u{0009}'..='\u{000d}'|'\u{001c}'..='\u{0020}'|'\u{1680}'|'\u{2000}'..='\u{2006}'|'\u{2008}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{205f}'|'\u{3000}')) || value.contains('=') { return Err(bad("invalid text")); }
    Ok(value)
}

impl ExtendedCorpus {
    pub fn parse(source: &str) -> Result<Self, DifferentialError> {
        if source.len() > MAX_BYTES || !source.is_ascii() || source.contains('\r') {
            return Err(bad("invalid corpus encoding/bound"));
        }
        let mut lines = source.strip_suffix('\n').unwrap_or(source).split('\n');
        let raw_header = lines.next().unwrap_or_default();
        if raw_header.len() > MAX_LINE {
            return Err(bad("header bound"));
        }
        let header: Vec<_> = raw_header.split('\t').collect();
        if header.len() != 3 || header[0] != HEADER {
            return Err(bad("unsupported corpus header"));
        }
        let seed = header[1].parse::<i64>().map_err(|_| bad("invalid seed"))?;
        if seed.to_string() != header[1] {
            return Err(bad("noncanonical seed"));
        }
        let count = number(header[2])?;
        if count > MAX_STEPS as u64 {
            return Err(bad("step bound"));
        }
        let mut steps = Vec::new();
        for line in lines {
            if steps.len() == MAX_STEPS || line.len() > MAX_LINE {
                return Err(bad("step/line bound"));
            }
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != 7 || !OPS.contains(&fields[1]) {
                return Err(bad("unknown operation/field count"));
            }
            let mut entries = Vec::new();
            if fields[2] != "~" {
                for raw in fields[2].split(',') {
                    if entries.len() == 64 {
                        return Err(bad("entry bound"));
                    }
                    let e: Vec<_> = raw.split(':').collect();
                    if e.len() != 4 {
                        return Err(bad("entry shape"));
                    }
                    let ttl = match e[3] {
                        "p" => ServiceTtl::Preserve,
                        "e" => ServiceTtl::Eternal,
                        value => {
                            let ticks =
                                number(value.strip_prefix('x').ok_or_else(|| bad("invalid TTL"))?)?;
                            if ticks == 0 {
                                return Err(bad("zero TTL"));
                            }
                            ServiceTtl::ExpireAfter(ticks)
                        }
                    };
                    entries.push(ExtendedEntry {
                        key: bytes(e[0], 1024)?,
                        value: if e[1] == "~" {
                            None
                        } else {
                            Some(bytes(e[1], 4096)?)
                        },
                        label: if e[2] == "~" { None } else { Some(text(e[2])?) },
                        ttl,
                    });
                }
            }
            let mut requests = Vec::new();
            if fields[3] != "~" {
                for raw in fields[3].split(',') {
                    if requests.len() == 64 {
                        return Err(bad("request bound"));
                    }
                    let (client, sequence) =
                        raw.split_once(':').ok_or_else(|| bad("request shape"))?;
                    let sequence = number(sequence)?;
                    if sequence == 0 {
                        return Err(bad("zero sequence"));
                    }
                    requests.push(MutationIdentity::new(text(client)?, sequence));
                }
            }
            let limits = if fields[4] == "~" {
                None
            } else {
                let (items, size) = fields[4]
                    .split_once(':')
                    .ok_or_else(|| bad("limits shape"))?;
                let (items, size) = (number(items)?, number(size)?);
                if items == 0 || items > 64 || size == 0 || size > 1024 * 1024 {
                    return Err(bad("limits bound"));
                }
                Some((items as usize, size as usize))
            };
            steps.push(ExtendedStep {
                id: text(fields[0])?,
                operation: fields[1].into(),
                entries,
                requests,
                limits,
                ticks: number(fields[5])?,
                lose_response: match fields[6] {
                    "0" => false,
                    "1" => true,
                    _ => return Err(bad("invalid loss flag")),
                },
            });
        }
        if steps.len() as u64 != count {
            return Err(bad("step count mismatch"));
        }
        let corpus = Self { seed, steps };
        corpus.validate()?;
        Ok(corpus)
    }

    pub fn validate(&self) -> Result<(), DifferentialError> {
        if self.steps.len() > MAX_STEPS {
            return Err(bad("step bound"));
        }
        let (mut ids, mut keys, mut labels) = (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
        let mut size = 0;
        for step in &self.steps {
            // Validate constructed DTOs through the same canonical boundary too.
            text(&hex(step.id.as_bytes()))?;
            if !ids.insert(&step.id) || !OPS.contains(&step.operation.as_str()) {
                return Err(bad("duplicate id/unknown operation"));
            }
            let op = step.operation.as_str();
            let point = matches!(
                op,
                "put"
                    | "reserve_put"
                    | "dispatch_put"
                    | "capture_version"
                    | "replace_if_version"
                    | "remove_if_version"
                    | "set_ttl_if_version"
                    | "evict"
            );
            let bulk = matches!(op, "bulk_replace" | "bulk_remove" | "bulk_set_ttl");
            let mutation =
                point && op != "capture_version" || bulk || matches!(op, "clear" | "destroy");
            let snapshot = matches!(op, "listener_resync" | "hold_listener_snapshot");
            if step.entries.len() > 64
                || step.requests.len() > 64
                || (point && step.entries.len() != 1)
                || (bulk && step.entries.is_empty())
                || (!point && !bulk && !step.entries.is_empty())
                || (mutation && step.requests.len() != 1)
                || (matches!(op, "retire_outcomes" | "cancel_reserved") && step.requests.len() != 1)
                || (op == "query_outcomes" && step.requests.is_empty())
                || (!mutation
                    && !matches!(op, "retire_outcomes" | "cancel_reserved" | "query_outcomes")
                    && !step.requests.is_empty())
                || snapshot != step.limits.is_some()
                || (if op == "advance" {
                    step.ticks == 0
                } else {
                    step.ticks != 0
                })
                || step.ticks > i64::MAX as u64
                || (step.lose_response && (!mutation || op == "reserve_put"))
            {
                return Err(bad("step shape"));
            }
            if let Some((items, size)) = step.limits {
                if items == 0 || items > 64 || size == 0 || size > 1024 * 1024 {
                    return Err(bad("limits bound"));
                }
            }
            size += step.id.len() + 64;
            for entry in &step.entries {
                let value = matches!(
                    op,
                    "put" | "reserve_put" | "dispatch_put" | "replace_if_version" | "bulk_replace"
                );
                let token = !matches!(op, "put" | "reserve_put" | "dispatch_put" | "evict");
                if value != entry.value.is_some()
                    || token != entry.label.is_some()
                    || entry.key.len() > 1024
                    || entry.value.as_ref().is_some_and(|v| v.len() > 4096)
                    || matches!(entry.ttl, ServiceTtl::ExpireAfter(0))
                    || matches!(entry.ttl,ServiceTtl::ExpireAfter(n) if n > i64::MAX as u64)
                {
                    return Err(bad("entry shape/bound"));
                }
                keys.insert(&entry.key);
                size += entry.key.len() + entry.value.as_ref().map_or(0, Vec::len) + 128;
                if let Some(label) = &entry.label {
                    text(&hex(label.as_bytes()))?;
                    if op == "capture_version" {
                        if !labels.insert(label) {
                            return Err(bad("duplicate token label"));
                        }
                    } else if !labels.contains(label) {
                        return Err(bad("unknown token label"));
                    }
                }
            }
            for request in &step.requests {
                text(&hex(request.client.as_bytes()))?;
                if request.sequence == 0 || request.sequence > i64::MAX as u64 {
                    return Err(bad("sequence bound"));
                }
                size += request.client.len() + 16;
            }
            if size > 512 * 1024 || keys.len() > 64 || labels.len() > 64 {
                return Err(bad("corpus payload/key/token bound"));
            }
        }
        Ok(())
    }

    pub fn encode(&self) -> Result<String, DifferentialError> {
        self.validate()?;
        let mut out = format!("{HEADER}\t{}\t{}\n", self.seed, self.steps.len());
        for s in &self.steps {
            let entries = if s.entries.is_empty() {
                "~".into()
            } else {
                s.entries
                    .iter()
                    .map(|e| {
                        format!(
                            "{}:{}:{}:{}",
                            hex(&e.key),
                            e.value.as_deref().map_or_else(|| "~".into(), hex),
                            e.label
                                .as_ref()
                                .map_or_else(|| "~".into(), |s| hex(s.as_bytes())),
                            match e.ttl {
                                ServiceTtl::Preserve => "p".into(),
                                ServiceTtl::Eternal => "e".into(),
                                ServiceTtl::ExpireAfter(t) => format!("x{t}"),
                            }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let requests = if s.requests.is_empty() {
                "~".into()
            } else {
                s.requests
                    .iter()
                    .map(|r| format!("{}:{}", hex(r.client.as_bytes()), r.sequence))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let line = format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                hex(s.id.as_bytes()),
                s.operation,
                entries,
                requests,
                s.limits
                    .map_or_else(|| "~".into(), |(n, b)| format!("{n}:{b}")),
                s.ticks,
                u8::from(s.lose_response)
            );
            if line.len() - 1 > MAX_LINE || out.len() + line.len() > MAX_BYTES {
                return Err(bad("encoded corpus bound"));
            }
            out.push_str(&line);
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedShrinkReport {
    pub corpus: ExtendedCorpus,
    pub replays: usize,
    pub work: usize,
    pub budget_exhausted: bool,
}

pub fn shrink_extended(
    corpus: ExtendedCorpus,
    maximum: usize,
    mismatch: impl FnMut(&ExtendedCorpus) -> Result<bool, DifferentialError>,
) -> Result<ExtendedCorpus, DifferentialError> {
    Ok(shrink_extended_bounded(corpus, maximum, 4096, mismatch)?.corpus)
}

/// Counts the original replay and candidate work, retaining only valid reductions.
pub fn shrink_extended_bounded(
    mut corpus: ExtendedCorpus,
    maximum: usize,
    maximum_work: usize,
    mut mismatch: impl FnMut(&ExtendedCorpus) -> Result<bool, DifferentialError>,
) -> Result<ExtendedShrinkReport, DifferentialError> {
    corpus.validate()?;
    if maximum == 0
        || maximum > 16
        || maximum_work == 0
        || maximum_work > 4096
        || corpus.steps.len() > maximum_work
    {
        return Err(bad("shrink budget"));
    }
    if !mismatch(&corpus)? {
        return Err(bad("original corpus does not fail"));
    }
    let mut attempts = 1;
    let mut work = corpus.steps.len();
    let mut index = 0;
    let mut exhausted = false;
    while index < corpus.steps.len() {
        if attempts == maximum || corpus.steps.len() - 1 > maximum_work - work {
            exhausted = true;
            break;
        }
        let mut candidate = corpus.clone();
        candidate.steps.remove(index);
        work += candidate.steps.len();
        if candidate.validate().is_ok() {
            attempts += 1;
            if mismatch(&candidate)? {
                corpus = candidate;
                continue;
            }
        }
        index += 1;
    }
    Ok(ExtendedShrinkReport {
        corpus,
        replays: attempts,
        work,
        budget_exhausted: exhausted,
    })
}
