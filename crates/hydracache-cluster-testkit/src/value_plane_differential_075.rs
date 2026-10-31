//! Bounded development traces executed independently by the Rust and Java IMap services.
//!
//! This is a test interchange format, not a production protocol or durable identity.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::value_plane_collection_075::{CollectionBounds, ReplacementEntry, ScanCursor};
use crate::value_plane_model_075::MutationIdentity;
use crate::value_plane_service_075::{
    OperationOutcomeQuery, ServiceCommand, ServiceItem, ServiceLimits, ServiceResult, ServiceTtl,
    UnifiedIMapService, VersionBulkEntry, VersionBulkOperation, VersionToken,
};

pub const MAX_TRACE_STEPS: usize = 512;
pub const MAX_TRACE_BYTES: usize = 512 * 1024;
pub const MAX_TRACE_LINE_BYTES: usize = 8 * 1024;
pub const MAX_OBSERVATION_BYTES: usize = 64 * 1024;
pub const MAX_TRANSCRIPT_BYTES: usize = 4 * 1024 * 1024;
pub const TRACE_HEADER: &str = "imap-api-trace-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DifferentialError(pub String);

impl fmt::Display for DifferentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DifferentialError {}

/// Ten tab-delimited fields keep the Java runner dependency-free and bounded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceStep {
    pub operation: String,
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
    pub expected: Option<Vec<u8>>,
    pub ttl: String,
    pub identity: u64,
    pub now: u64,
    pub deadline: u64,
    pub detail: String,
    pub extra: String,
}

impl TraceStep {
    pub fn point(operation: &str, key: &[u8], value: Option<&[u8]>) -> Self {
        Self {
            operation: operation.into(),
            key: key.to_vec(),
            value: value.map(<[u8]>::to_vec),
            expected: None,
            ttl: "eternal".into(),
            identity: 0,
            now: 0,
            deadline: 1_000_000,
            detail: String::new(),
            extra: String::new(),
        }
    }

    pub fn encode(&self) -> String {
        [
            self.operation.clone(),
            hex(&self.key),
            optional_hex(self.value.as_deref()),
            optional_hex(self.expected.as_deref()),
            self.ttl.clone(),
            self.identity.to_string(),
            self.now.to_string(),
            self.deadline.to_string(),
            self.detail.clone(),
            self.extra.clone(),
        ]
        .join("\t")
    }

    pub fn parse(line: &str) -> Result<Self, DifferentialError> {
        if line.len() > MAX_TRACE_LINE_BYTES {
            return fail("trace line exceeds byte bound");
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 10
            || fields[0].is_empty()
            || !fields[0]
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
        {
            return fail("trace requires an operation and exactly ten fields");
        }
        if !matches!(fields[4], "eternal" | "preserve") {
            let duration = fields[4]
                .strip_prefix("after:")
                .ok_or_else(|| DifferentialError("invalid trace TTL".into()))?;
            if parse_number(duration)? == 0 {
                return fail("trace TTL must be positive");
            }
        }
        for field in &fields[8..] {
            if !field
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && byte != b';' && byte != b'[' && byte != b']')
                && !field.is_empty()
            {
                return fail("invalid trace detail or extra field");
            }
        }
        if !fields[8].is_empty()
            && fields[8].bytes().all(|byte| byte.is_ascii_digit())
            && parse_number(fields[8])? > i32::MAX as u64
        {
            return fail("trace page limit exceeds common integer range");
        }
        Ok(Self {
            operation: fields[0].into(),
            key: decode_hex(fields[1])?,
            value: decode_optional(fields[2])?,
            expected: decode_optional(fields[3])?,
            ttl: fields[4].into(),
            identity: parse_number(fields[5])?,
            now: parse_number(fields[6])?,
            deadline: parse_number(fields[7])?,
            detail: fields[8].into(),
            extra: fields[9].into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiTrace {
    pub seed: u64,
    pub steps: Vec<TraceStep>,
}

impl ApiTrace {
    pub fn encode(&self) -> String {
        let mut result = format!("{TRACE_HEADER}\t{}\n", self.seed);
        for step in &self.steps {
            result.push_str(&step.encode());
            result.push('\n');
        }
        result
    }

    pub fn parse(source: &str) -> Result<Self, DifferentialError> {
        if source.len() > MAX_TRACE_BYTES || !source.is_ascii() || source.contains('\r') {
            return fail("trace exceeds byte bound or contains non-ASCII/CR data");
        }
        let mut lines = source.lines();
        let header = lines
            .next()
            .ok_or_else(|| DifferentialError("missing trace header".into()))?;
        let seed = header
            .strip_prefix(&format!("{TRACE_HEADER}\t"))
            .ok_or_else(|| DifferentialError("unsupported trace header".into()))?;
        let mut steps = Vec::new();
        for line in lines {
            if steps.len() == MAX_TRACE_STEPS {
                return fail("trace exceeds step bound");
            }
            steps.push(TraceStep::parse(line)?);
        }
        if steps.is_empty() {
            return fail("trace has no operations");
        }
        Ok(Self {
            seed: parse_number(seed)?,
            steps,
        })
    }
}

/// A deterministic prefix exercises every admitted operation before the seeded suffix.
pub fn generated_trace(seed: u64, random_steps: usize) -> Result<ApiTrace, DifferentialError> {
    if seed > i64::MAX as u64 {
        return fail("generated trace seed or step budget exceeded");
    }
    let mut steps = coverage_steps();
    steps.extend(lifecycle_steps());
    steps.extend(listener_filter_trace().steps);
    if random_steps > MAX_TRACE_STEPS.saturating_sub(steps.len() + 4) {
        return fail("generated trace seed or step budget exceeded");
    }
    let mut state = seed.max(1);
    let operations = [
        "get",
        "set",
        "contains_entry",
        "get_or_default",
        "put_if_absent",
        "conditional_replace",
        "get_entry_view",
        "try_put",
        "try_remove",
        "contains_key",
        "remaining_ttl",
        "replace_if_present",
        "delete",
    ];
    for index in 0..random_steps {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let operation = operations[(state as usize) % operations.len()];
        let key = [(state >> 8) as u8 % 8];
        let value = [(state >> 16) as u8 % 6];
        let mut step = TraceStep::point(operation, &key, Some(&value));
        step.expected = Some(vec![(state >> 24) as u8 % 6]);
        step.identity = 100 + index as u64;
        step.now = 10 + index as u64 / 5;
        step.deadline = if index % 11 == 0 {
            step.now
        } else {
            step.now + 100
        };
        if index % 7 == 0 {
            step.ttl = "after:3".into();
        }
        if operation == "try_put" && index % 3 == 0 {
            step.detail = "lost".into();
        }
        steps.push(step);
    }
    let now = steps.last().map_or(0, |step| step.now);
    let mut listener = TraceStep::point("entry_listener", &[], None);
    listener.now = now;
    listener.identity = 9001;
    listener.detail = "values".into();
    steps.push(listener);
    let mut expiring = TraceStep::point("set", b"expiry", Some(b"e"));
    expiring.identity = 9002;
    expiring.now = now;
    expiring.ttl = "after:1".into();
    steps.push(expiring);
    let mut expired_read = TraceStep::point("get", b"expiry", None);
    expired_read.now = now + 1;
    steps.push(expired_read);
    let mut destroy = TraceStep::point("destroy", &[], None);
    destroy.now = now + 1;
    destroy.identity = 9000;
    steps.push(destroy);
    Ok(ApiTrace { seed, steps })
}

fn coverage_steps() -> Vec<TraceStep> {
    let mut steps = Vec::new();
    for (operation, key, value) in [
        ("entry_listener", b"".as_slice(), None),
        ("get", b"a", None),
        ("put", b"a", Some(b"one".as_slice())),
        ("get", b"a", None),
        ("contains_key", b"a", None),
        ("contains_entry", b"a", Some(b"one".as_slice())),
        ("get_or_default", b"missing", Some(b"fallback".as_slice())),
        ("get_entry_view", b"a", None),
        ("replace_if_version", b"a", Some(b"two".as_slice())),
        ("remove_if_version", b"a", None),
        ("get_entry_view", b"a", None),
        ("set_ttl_if_version", b"a", None),
        ("put_if_absent", b"a", Some(b"other".as_slice())),
        ("conditional_replace", b"a", Some(b"three".as_slice())),
        ("conditional_remove", b"a", None),
        ("replace_if_present", b"a", Some(b"four".as_slice())),
        ("get_and_put", b"a", Some(b"five".as_slice())),
        ("get_and_remove", b"a", None),
        ("set", b"a", Some(b"six".as_slice())),
        ("set_ttl", b"a", None),
        ("remaining_ttl", b"a", None),
        ("size", b"", None),
        ("is_empty", b"", None),
        ("contains_value_bounded", b"", Some(b"six".as_slice())),
        ("set", b"b", Some(b"b".as_slice())),
        ("set", b"c", Some(b"c".as_slice())),
        ("key_set_page", b"", None),
        ("values_page", b"", None),
        ("entry_set_page", b"", None),
        ("put_all_detailed", b"", None),
        ("get_all", b"", None),
        ("replace_all_detailed", b"", None),
        ("remove_all_detailed", b"", None),
        ("try_put", b"a", Some(b"seven".as_slice())),
        ("query_operation_outcome", b"", None),
        ("reserve_operation", b"pending", Some(b"p".as_slice())),
        ("query_operation_outcome", b"", None),
        ("cancel_reserved", b"", None),
        ("query_operation_outcome", b"", None),
        ("try_remove", b"a", None),
        ("delete", b"b", None),
        ("evict", b"c", None),
        ("clear_detailed", b"", None),
        ("evict_all_detailed", b"", None),
        ("local_statistics", b"", None),
        ("capability_descriptor", b"", None),
        ("remove_entry_listener", b"", None),
    ] {
        let mut step = TraceStep::point(operation, key, value);
        step.identity = steps.len() as u64 + 1;
        step.expected = Some(b"mismatch".to_vec());
        if matches!(operation, "replace_if_version" | "set_ttl_if_version") {
            step.detail = "remembered".into();
        }
        if operation == "remove_if_version" {
            step.detail = "stale".into();
        }
        if matches!(operation, "set_ttl" | "set_ttl_if_version") {
            step.ttl = "after:5".into();
        }
        if operation == "entry_listener" {
            step.identity = 1;
            step.detail = "values".into();
            step.expected = None;
        }
        if operation == "remove_entry_listener" {
            step.identity = 1;
        }
        if operation == "put_all_detailed" || operation == "replace_all_detailed" {
            step.extra = "63:32,62:31".into();
        }
        if operation == "get_all" || operation == "remove_all_detailed" {
            step.extra = "63,62,65".into();
        }
        if operation == "query_operation_outcome" || operation == "cancel_reserved" {
            step.identity = steps
                .iter()
                .rev()
                .find(|previous: &&TraceStep| {
                    previous.operation == "try_put" || previous.operation == "reserve_operation"
                })
                .map_or(0, |previous| previous.identity);
        }
        if operation == "try_put" {
            step.detail = "lost".into();
        }
        steps.push(step);
        if operation == "conditional_remove" {
            let mut matched = TraceStep::point("conditional_remove", b"a", None);
            matched.expected = Some(b"two".to_vec());
            matched.identity = 8600;
            steps.push(matched);
            let mut restore = TraceStep::point("set", b"a", Some(b"two"));
            restore.identity = 8601;
            steps.push(restore);
        }
        if operation == "clear_detailed" {
            let mut empty_clear = TraceStep::point("clear_detailed", b"", None);
            empty_clear.identity = 8602;
            steps.push(empty_clear);
        }
        if operation == "key_set_page" {
            steps.last_mut().unwrap().detail = "1".into();
            let mut resume = TraceStep::point("key_set_page", b"", None);
            resume.detail = "resume".into();
            steps.push(resume.clone());
            let mut mutation = TraceStep::point("set", b"d", Some(b"d"));
            mutation.identity = 8500;
            steps.push(mutation);
            steps.push(resume);
            let mut wrong = TraceStep::point("values_page", b"", None);
            wrong.detail = "wrong_cursor".into();
            steps.push(wrong);
        }
    }
    steps
}

fn lifecycle_steps() -> Vec<TraceStep> {
    let mut steps = Vec::new();
    let mut reset = TraceStep::point("reset_limits", b"", None);
    reset.detail = "96".into();
    reset.extra = "1".into();
    steps.push(reset);
    let mut listener = TraceStep::point("entry_listener", b"", None);
    listener.identity = 10000;
    listener.detail = "values".into();
    listener.extra = "1".into();
    steps.push(listener);
    for (key, value, id) in [(b"x", b"1", 10001), (b"y", b"2", 10002)] {
        let mut step = TraceStep::point("put", key, Some(value));
        step.identity = id;
        step.ttl = "after:5".into();
        steps.push(step);
    }
    let mut over_budget = TraceStep::point("put", b"z", Some(&[1; 100]));
    over_budget.identity = 10003;
    steps.push(over_budget);
    let mut pending = TraceStep::point("reserve_operation", b"p", Some(b"p"));
    pending.identity = 10004;
    steps.push(pending);
    let mut retire = TraceStep::point("retire_outcomes", b"", None);
    retire.identity = 10004;
    steps.push(retire);
    let mut query = TraceStep::point("query_outcomes", b"", None);
    query.detail = "4096".into();
    query.extra = "10001,10004,99999".into();
    steps.push(query.clone());
    let mut cancel = TraceStep::point("cancel_reserved", b"", None);
    cancel.identity = 10004;
    steps.push(cancel);
    let mut retire = TraceStep::point("retire_outcomes", b"", None);
    retire.identity = 10002;
    steps.push(retire);
    let mut retry = TraceStep::point("put", b"x", Some(b"1"));
    retry.identity = 10001;
    retry.ttl = "after:5".into();
    steps.push(retry.clone());
    steps.push(query);
    for key in [b"x", b"y"] {
        steps.push(TraceStep::point("get_entry_view", key, None));
    }
    let mut bulk = TraceStep::point("version_bulk_replace", b"", None);
    bulk.identity = 10005;
    bulk.ttl = "preserve".into();
    bulk.extra = "79:33,78:34".into();
    steps.push(bulk.clone());
    let mut mismatch = bulk.clone();
    mismatch.identity = 10006;
    steps.push(mismatch);
    steps.push(bulk);
    let mut resync = TraceStep::point("resync_listener", b"", None);
    resync.identity = 10000;
    resync.detail = "2".into();
    resync.extra = "4096".into();
    steps.push(resync);
    for key in [b"x", b"y"] {
        steps.push(TraceStep::point("get_entry_view", key, None));
    }
    let mut ttl = TraceStep::point("version_bulk_set_ttl", b"", None);
    ttl.identity = 10007;
    ttl.ttl = "after:1".into();
    ttl.extra = "79,78".into();
    steps.push(ttl);
    for key in [b"x", b"y"] {
        steps.push(TraceStep::point("get_entry_view", key, None));
    }
    let mut remove = TraceStep::point("version_bulk_remove", b"", None);
    remove.identity = 10008;
    remove.extra = "78".into();
    steps.push(remove.clone());
    steps.push(TraceStep::point("checkpoint_restore", b"", None));
    let mut remove_listener = TraceStep::point("remove_entry_listener", b"", None);
    remove_listener.identity = 10000;
    steps.push(remove_listener);
    steps.push(TraceStep::point("checkpoint_restore", b"", None));
    remove.identity = 10009;
    remove.extra = "79".into();
    steps.push(remove);
    steps.push(retry);
    let mut resource = TraceStep::point("resource_usage", b"", None);
    resource.detail = "".into();
    steps.push(resource);
    let mut reset = TraceStep::point("reset_limits", b"", None);
    reset.detail = "96".into();
    reset.extra = "1".into();
    steps.push(reset);
    let mut listener = TraceStep::point("entry_listener", b"", None);
    listener.identity = 11000;
    listener.detail = "values".into();
    steps.push(listener);
    for (index, key) in [b"x", b"y", b"z"].iter().enumerate() {
        let mut step = TraceStep::point("set", *key, Some(b"e"));
        step.identity = 11001 + index as u64;
        step.ttl = "after:1".into();
        steps.push(step);
    }
    for _ in 0..4 {
        let mut expire = TraceStep::point("expire_step", b"", None);
        expire.now = 1;
        expire.detail = "1".into();
        steps.push(expire);
    }
    let mut reset = TraceStep::point("reset_limits", b"", None);
    reset.detail = "67108864".into();
    reset.extra = "64".into();
    reset.now = 1;
    steps.push(reset);
    steps
}

/// Operation-specific listener prefix/exact fixtures use the existing development trace fields.
pub fn listener_filter_trace() -> ApiTrace {
    let mut steps = Vec::new();
    let mut reset = TraceStep::point("reset_limits", b"", None);
    reset.detail = "67108864".into();
    reset.extra = "64".into();
    steps.push(reset.clone());
    for (id, prefix, exact) in [
        (20000, vec![0, 255], None),
        (20001, vec![0, 255], Some(vec![0, 255, 1])),
        (20002, vec![], Some(vec![])),
        (20003, vec![0, 254], Some(vec![0, 255, 1])),
    ] {
        let mut listener = TraceStep::point("entry_listener", &prefix, None);
        listener.identity = id;
        listener.expected = exact;
        listener.detail = "values".into();
        steps.push(listener);
    }
    for (index, key) in [
        vec![],
        vec![0],
        vec![0, 254],
        vec![0, 255],
        vec![0, 255, 1],
        vec![0, 255, 2],
    ]
    .iter()
    .enumerate()
    {
        let mut write = TraceStep::point("set", key, Some(b"v"));
        write.identity = 21000 + index as u64;
        steps.push(write);
    }
    for id in [20000, 20001, 20002, 20003] {
        let mut snapshot = TraceStep::point("resync_listener", b"", None);
        snapshot.identity = id;
        snapshot.detail = "8".into();
        snapshot.extra = "1024".into();
        steps.push(snapshot);
    }
    let mut clear = TraceStep::point("clear_detailed", b"", None);
    clear.identity = 21006;
    steps.push(clear);
    for id in [20000, 20001, 20002, 20003] {
        let mut remove = TraceStep::point("remove_entry_listener", b"", None);
        remove.identity = id;
        steps.push(remove);
    }
    steps.push(reset);
    ApiTrace { seed: 117, steps }
}

pub fn execute_rust(trace: &ApiTrace) -> Result<Vec<String>, DifferentialError> {
    ApiTrace::parse(&trace.encode())?;
    let mut service = UnifiedIMapService::new(CollectionBounds::default(), 1)
        .map_err(|error| DifferentialError(error.to_string()))?;
    let mut remembered = BTreeMap::<Vec<u8>, VersionToken>::new();
    let mut cursors = BTreeMap::<String, ScanCursor>::new();
    let mut listeners = BTreeSet::new();
    let mut observations = Vec::with_capacity(trace.steps.len());
    let mut transcript_bytes = 0_usize;
    for step in &trace.steps {
        let command = to_command(step, &remembered, &cursors)?;
        let result = match step.operation.as_str() {
            "reset_limits" => {
                let limits = ServiceLimits {
                    value_bytes: parse_number(&step.detail)? as usize,
                    expiry_work: parse_number(&step.extra)? as usize,
                    ..ServiceLimits::default()
                };
                UnifiedIMapService::with_limits(CollectionBounds::default(), 1, limits).map(
                    |replacement| {
                        service = replacement;
                        remembered.clear();
                        cursors.clear();
                        listeners.clear();
                        ServiceResult::new("reset")
                    },
                )
            }
            "retire_outcomes" => {
                service
                    .retire_outcomes("trace", 1, step.identity)
                    .map(|receipt| {
                        let mut result = ServiceResult::new("retired");
                        result.number = receipt.removed as u64;
                        result.version = Some(receipt.through);
                        result
                    })
            }
            "query_outcomes" => {
                let identities = step
                    .extra
                    .split(',')
                    .map(|number| Ok(MutationIdentity::new("trace", parse_number(number)?)))
                    .collect::<Result<Vec<_>, DifferentialError>>()?;
                service
                    .query_operation_outcomes(&identities, command.limit)
                    .map(|outcomes| {
                        let mut result = ServiceResult::new("outcomes");
                        result.number = outcomes.len() as u64;
                        result.items = outcomes
                            .into_iter()
                            .enumerate()
                            .map(|(index, outcome)| {
                                let mut item = ServiceItem {
                                    index,
                                    key: identities[index].sequence.to_string().into_bytes(),
                                    kind: String::new(),
                                    value: None,
                                    version: None,
                                    remaining_ttl: None,
                                };
                                match outcome {
                                    OperationOutcomeQuery::Unknown => item.kind = "unknown".into(),
                                    OperationOutcomeQuery::Pending => item.kind = "pending".into(),
                                    OperationOutcomeQuery::Completed(receipt) => {
                                        item.kind = format!("completed:{}", receipt.kind);
                                        item.value = receipt.value;
                                        item.version = receipt.version;
                                        item.remaining_ttl = receipt.remaining_ttl;
                                    }
                                }
                                item
                            })
                            .collect();
                        result
                    })
            }
            "resync_listener" => service
                .resync_listener(
                    step.identity,
                    step.now,
                    command.limit,
                    parse_number(&step.extra)? as usize,
                )
                .map(|mut snapshot| {
                    let mut result = ServiceResult::new("resynced");
                    result.version = Some(snapshot.watermark);
                    result.number = snapshot.entries.len() as u64;
                    result.flag = service.resource_usage().resync > 0;
                    result.items = snapshot
                        .entries
                        .iter()
                        .enumerate()
                        .map(|(index, entry)| ServiceItem {
                            index,
                            key: entry.key.clone(),
                            kind: "present".into(),
                            value: Some(entry.value.clone()),
                            version: Some(entry.version),
                            remaining_ttl: entry.remaining_ttl,
                        })
                        .collect();
                    snapshot.close();
                    result
                }),
            "expire_step" => service
                .expire_step(step.now, command.limit)
                .map(|progress| {
                    let mut result = ServiceResult::new("expiry_progress");
                    result.number = progress.expired as u64;
                    result.version = Some(progress.examined as u64);
                    result.flag = progress.pending;
                    result
                }),
            "checkpoint_restore" => service
                .checkpoint()
                .and_then(UnifiedIMapService::restore)
                .map(|replacement| {
                    service = replacement;
                    ServiceResult::new("restored")
                }),
            "resource_usage" => {
                let mut result = ServiceResult::new("resources");
                result.number = service.resource_usage().values as u64;
                Ok(result)
            }
            "version_bulk_replace" | "version_bulk_remove" | "version_bulk_set_ttl" => {
                let entries = version_entries(step, &remembered, command.ttl)?;
                let operation = match step.operation.as_str() {
                    "version_bulk_replace" => VersionBulkOperation::Replace,
                    "version_bulk_remove" => VersionBulkOperation::Remove,
                    _ => VersionBulkOperation::SetTtl,
                };
                service.bulk_version_conditional(
                    operation,
                    &entries,
                    MutationIdentity::new("trace", step.identity),
                    step.now,
                    step.deadline,
                    command.lose_response,
                )
            }
            "reserve_operation" => service
                .reserve_operation(MutationIdentity::new("trace", step.identity), &command)
                .map(|()| ServiceResult::new("reserved")),
            "cancel_reserved" => service
                .cancel_reserved(&MutationIdentity::new("trace", step.identity))
                .map(|cancelled| {
                    let mut result = ServiceResult::new("cancelled");
                    result.flag = cancelled;
                    result
                }),
            _ => service.execute(command),
        };
        let mut status = String::new();
        match &result {
            Ok(result) => {
                status.push_str(&canonical_result(result));
                if let Some(token) = &result.token {
                    remembered.insert(step.key.clone(), *token);
                }
                if let Some(cursor) = result.cursor {
                    cursors.insert(step.operation.clone(), cursor);
                } else if matches!(
                    step.operation.as_str(),
                    "key_set_page" | "values_page" | "entry_set_page" | "contains_value_bounded"
                ) {
                    cursors.remove(&step.operation);
                }
                if step.operation == "entry_listener" {
                    listeners.insert(step.identity);
                }
                if step.operation == "remove_entry_listener" {
                    listeners.remove(&step.identity);
                }
            }
            Err(error) => status.push_str(&format!("error:{}|~|~|~|0|0|[]", error.kind())),
        }
        let snapshot = service
            .diagnostic_snapshot(step.now)
            .map_err(|error| DifferentialError(error.to_string()))?;
        status.push_str("|[");
        status.push_str(
            &snapshot
                .iter()
                .map(|entry| {
                    format!(
                        "{}/{}/{}/{}",
                        hex(&entry.key),
                        hex(&entry.value),
                        entry.version,
                        optional_number(entry.remaining_ttl)
                    )
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        status.push_str("]|[");
        let mut events = Vec::new();
        for listener in &listeners {
            for event in service
                .drain_listener(*listener)
                .map_err(|error| DifferentialError(error.to_string()))?
            {
                events.push(format!(
                    "{}/{}/{}/{}/{}/{}/{}",
                    listener,
                    event.kind,
                    hex(&event.key),
                    optional_hex(event.old_value.as_deref()),
                    optional_hex(event.value.as_deref()),
                    event.version,
                    event.watermark
                ));
            }
        }
        status.push_str(&events.join(","));
        let stats = service.statistics();
        status.push_str(&format!(
            "]|[{},{},{},{},{},{},{},{},{},{},{}]",
            stats.live_entries,
            stats.value_bytes,
            stats.retained_outcomes,
            stats.listeners,
            stats.queued_events,
            stats.inflight,
            stats.rejections,
            stats.cancellations,
            stats.requests,
            stats.active_streams,
            stats.active_cursors
        ));
        let usage = service.resource_usage();
        status.push_str(&format!(
            "|[{},{},{},{},{},{}]",
            usage.values,
            usage.outcomes,
            usage.pending,
            usage.listeners,
            usage.streams,
            usage.resync
        ));
        transcript_bytes = transcript_bytes
            .saturating_add(status.len())
            .saturating_add(1);
        if status.len() > MAX_OBSERVATION_BYTES || transcript_bytes > MAX_TRANSCRIPT_BYTES {
            return fail("trace observations exceed output bound");
        }
        observations.push(status);
    }
    Ok(observations)
}

fn to_command(
    step: &TraceStep,
    remembered: &BTreeMap<Vec<u8>, VersionToken>,
    cursors: &BTreeMap<String, ScanCursor>,
) -> Result<ServiceCommand, DifferentialError> {
    let mut command = ServiceCommand::point(
        if step.operation == "reserve_operation" {
            "put"
        } else {
            &step.operation
        },
        &step.key,
    );
    command.value = step.value.clone();
    command.expected = step.expected.clone();
    command.now = step.now;
    command.deadline = step.deadline;
    command.identity = (step.identity > 0).then(|| MutationIdentity::new("trace", step.identity));
    command.listener_id = step.identity;
    command.include_value = step.detail == "values";
    command.lose_response = step.detail == "lost";
    command.limit =
        if !step.detail.is_empty() && step.detail.bytes().all(|byte| byte.is_ascii_digit()) {
            step.detail
                .parse()
                .map_err(|_| DifferentialError("invalid trace limit".into()))?
        } else {
            256
        };
    if step.operation == "entry_listener" && !step.extra.is_empty() {
        command.limit = parse_number(&step.extra)? as usize;
    }
    if step.detail == "resume" {
        command.cursor = cursors.get(&step.operation).copied();
        command.limit = 1;
    }
    if step.detail == "wrong_cursor" {
        command.cursor = cursors.get("key_set_page").copied();
        command.limit = 1;
    }
    command.ttl = match step.ttl.as_str() {
        "preserve" => ServiceTtl::Preserve,
        "eternal" => ServiceTtl::Eternal,
        value => ServiceTtl::ExpireAfter(parse_number(
            value
                .strip_prefix("after:")
                .ok_or_else(|| DifferentialError("invalid TTL".into()))?,
        )?),
    };
    if step.detail == "remembered" {
        command.token = remembered.get(&step.key).copied();
    } else if step.detail == "stale" || step.detail == "wrong_generation" {
        if let Some(token) = remembered.get(&step.key) {
            let mut token = *token;
            if step.detail == "stale" {
                token.version = if token.version > 1 {
                    token.version - 1
                } else {
                    token.version + 1
                };
            } else {
                token.namespace_generation += 1;
            }
            command.token = Some(token);
        }
    }
    if matches!(step.operation.as_str(), "get_all" | "remove_all_detailed")
        && !step.extra.is_empty()
    {
        command.keys = step
            .extra
            .split(',')
            .map(decode_hex)
            .collect::<Result<_, _>>()?;
    }
    if matches!(
        step.operation.as_str(),
        "put_all_detailed" | "replace_all_detailed"
    ) && !step.extra.is_empty()
    {
        command.entries = step
            .extra
            .split(',')
            .map(|entry| {
                let (key, value) = entry
                    .split_once(':')
                    .ok_or_else(|| DifferentialError("invalid bulk trace entry".into()))?;
                Ok(ReplacementEntry {
                    key: decode_hex(key)?,
                    value: decode_hex(value)?,
                })
            })
            .collect::<Result<_, DifferentialError>>()?;
    }
    Ok(command)
}

fn version_entries(
    step: &TraceStep,
    remembered: &BTreeMap<Vec<u8>, VersionToken>,
    ttl: ServiceTtl,
) -> Result<Vec<VersionBulkEntry>, DifferentialError> {
    step.extra
        .split(',')
        .map(|entry| {
            let (key, value) = if step.operation == "version_bulk_replace" {
                let (key, value) = entry
                    .split_once(':')
                    .ok_or_else(|| DifferentialError("invalid version bulk entry".into()))?;
                (decode_hex(key)?, Some(decode_hex(value)?))
            } else {
                (decode_hex(entry)?, None)
            };
            let token = remembered.get(&key).copied().unwrap_or(VersionToken {
                namespace_generation: 1,
                map_id: i64::MAX as u64,
                version: 1,
            });
            Ok(VersionBulkEntry {
                key,
                token,
                value,
                ttl,
            })
        })
        .collect()
}

fn canonical_result(result: &ServiceResult) -> String {
    let items = result
        .items
        .iter()
        .map(|item| {
            format!(
                "{}/{}/{}/{}/{}/{}",
                item.index,
                hex(&item.key),
                item.kind,
                optional_hex(item.value.as_deref()),
                optional_number(item.version),
                optional_number(item.remaining_ttl)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{}|{}|{}|{}|{}|{}|[{}]",
        result.kind,
        optional_hex(result.value.as_deref()),
        optional_number(result.version),
        optional_number(result.remaining_ttl),
        result.number,
        u8::from(result.flag),
        items
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceDifference {
    pub step: usize,
    pub operation: String,
    pub rust: String,
    pub java: String,
}

pub fn first_difference(
    trace: &ApiTrace,
    rust: &[String],
    java: &[String],
) -> Option<TraceDifference> {
    for index in 0..trace.steps.len().max(rust.len()).max(java.len()) {
        let left = rust.get(index).map(String::as_str).unwrap_or("<missing>");
        let right = java.get(index).map(String::as_str).unwrap_or("<missing>");
        if left != right
            || rust.get(index).is_none()
            || java.get(index).is_none()
            || index >= trace.steps.len()
        {
            return Some(TraceDifference {
                step: index,
                operation: trace
                    .steps
                    .get(index)
                    .map_or("<extra>", |step| step.operation.as_str())
                    .into(),
                rust: left.into(),
                java: right.into(),
            });
        }
    }
    None
}

/// Deterministic deletion shrinking has a bounded number of predicate calls.
pub fn shrink_trace(
    trace: &ApiTrace,
    max_attempts: usize,
    mut fails: impl FnMut(&ApiTrace) -> Result<bool, DifferentialError>,
) -> Result<ApiTrace, DifferentialError> {
    if max_attempts == 0 || !fails(trace)? {
        return Ok(trace.clone());
    }
    let mut result = trace.clone();
    let mut attempts = 1;
    let mut partitions = 2_usize;
    while result.steps.len() > 1 && attempts < max_attempts {
        let chunk = result.steps.len().div_ceil(partitions);
        let mut reduced = false;
        for start in (0..result.steps.len()).step_by(chunk) {
            if attempts == max_attempts {
                break;
            }
            let end = start.saturating_add(chunk).min(result.steps.len());
            if end - start == result.steps.len() {
                continue;
            }
            let mut candidate = result.clone();
            candidate.steps.drain(start..end);
            attempts += 1;
            if fails(&candidate)? {
                result = candidate;
                partitions = partitions.saturating_sub(1).max(2);
                reduced = true;
                break;
            }
        }
        if !reduced {
            if partitions >= result.steps.len() {
                break;
            }
            partitions = partitions.saturating_mul(2).min(result.steps.len());
        }
    }
    Ok(result)
}

pub fn hex(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn optional_hex(value: Option<&[u8]>) -> String {
    value.map(hex).unwrap_or_else(|| "~".into())
}
fn optional_number(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "~".into())
}

fn decode_optional(value: &str) -> Result<Option<Vec<u8>>, DifferentialError> {
    if value == "~" {
        Ok(None)
    } else {
        decode_hex(value).map(Some)
    }
}
fn parse_number(value: &str) -> Result<u64, DifferentialError> {
    value
        .parse()
        .ok()
        .filter(|value| *value <= i64::MAX as u64)
        .ok_or_else(|| DifferentialError("invalid nonnegative trace integer".into()))
}
fn decode_hex(value: &str) -> Result<Vec<u8>, DifferentialError> {
    if !value.len().is_multiple_of(2) || value.len() > 2048 {
        return fail("invalid or oversized trace hex");
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char)
                .to_digit(16)
                .ok_or_else(|| DifferentialError("invalid trace hex".into()))?;
            let low = (pair[1] as char)
                .to_digit(16)
                .ok_or_else(|| DifferentialError("invalid trace hex".into()))?;
            Ok((high * 16 + low) as u8)
        })
        .collect()
}
fn fail<T>(message: &str) -> Result<T, DifferentialError> {
    Err(DifferentialError(message.into()))
}
