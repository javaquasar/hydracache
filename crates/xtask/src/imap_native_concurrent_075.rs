//! Independent bounded checker for actual official-client native histories.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    path::Path,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Get,
    ContainsKey,
    Put,
    PutIfAbsent,
    ReplaceIfPresent,
    GetAndPut,
    GetAndRemove,
    ReplaceIfValue,
    RemoveIfValue,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: usize,
    pub worker: usize,
    pub position: usize,
    pub operation: Operation,
    pub key: String,
    #[serde(deserialize_with = "required_option")]
    pub value: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub expected: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Invoke,
    Return,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawEvent {
    pub phase: Phase,
    pub stamp: u64,
    pub call_id: usize,
    pub worker: usize,
    #[serde(deserialize_with = "required_option")]
    pub value: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub boolean: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub key: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEvent {
    pub kind: String,
    pub key: String,
    #[serde(deserialize_with = "required_option")]
    pub old_value: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub value: Option<String>,
}
fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct History {
    pub seed: i64,
    pub attempts: u32,
    pub member_uuid: String,
    pub clients: Vec<String>,
    pub program: Vec<Command>,
    pub raw: Vec<RawEvent>,
    pub raw_digest: String,
    pub final_entries: Vec<Entry>,
    pub final_digest: String,
    pub overlap: bool,
    pub verdict: String,
    pub work: u64,
    pub witness: Vec<usize>,
    pub native_events: Vec<NativeEvent>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Histories {
    pub schema: String,
    pub scope: String,
    pub artifact_sha256: String,
    pub histories: Vec<History>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Linearizable,
    NonLinearizable,
    Exhausted,
}
#[derive(Clone, PartialEq, Eq)]
struct ResultValue {
    value: Option<String>,
    boolean: Option<bool>,
}
struct Call {
    start: u64,
    end: u64,
    result: ResultValue,
}
fn byte_string(value: &str) -> bool {
    value.len() <= 128
        && value.len().is_multiple_of(2)
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}
fn raw_digest(events: &[RawEvent]) -> String {
    let mut canonical = String::new();
    for event in events {
        canonical.push_str(&format!(
            "{}|{}|{}|{}",
            if event.phase == Phase::Invoke {
                "i"
            } else {
                "r"
            },
            event.stamp,
            event.call_id,
            event.worker
        ));
        if event.phase == Phase::Return {
            canonical.push_str(&format!(
                "|{}|{}",
                event.value.as_deref().unwrap_or("null"),
                event
                    .boolean
                    .map_or_else(|| "null".into(), |v| v.to_string())
            ));
        }
        canonical.push('\n');
    }
    crate::imap_qualification_075::digest(canonical.as_bytes())
}
fn final_map(history: &History) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    if history.final_entries.len() > 4 {
        return Err("native final key bound".into());
    }
    let mut map = BTreeMap::new();
    for entry in &history.final_entries {
        if !byte_string(&entry.key)
            || !byte_string(&entry.value)
            || !history.program.iter().any(|c| c.key == entry.key)
            || map.insert(entry.key.clone(), entry.value.clone()).is_some()
        {
            return Err("native final entry invalid/foreign/duplicate".into());
        }
    }
    let mut bytes = Vec::new();
    for (key, value) in &map {
        for hex in [key, value] {
            bytes.extend_from_slice(&((hex.len() / 2) as u32).to_be_bytes());
            for pair in hex.as_bytes().chunks_exact(2) {
                bytes.push(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?);
            }
        }
        bytes.push(0);
    }
    if history.final_digest != crate::imap_qualification_075::digest(&bytes) {
        return Err("native final digest differs".into());
    }
    Ok(map)
}
fn normalize(history: &History) -> Result<Vec<Call>, Box<dyn Error>> {
    let count = history.program.len();
    if !(2..=16).contains(&count)
        || history.raw.len() != 2 * count
        || history.raw_digest != raw_digest(&history.raw)
    {
        return Err("native raw completeness/hash bound".into());
    }
    let mut positions = [0; 2];
    let mut keys = BTreeSet::new();
    for (id, c) in history.program.iter().enumerate() {
        let writes = matches!(
            c.operation,
            Operation::Put
                | Operation::PutIfAbsent
                | Operation::ReplaceIfPresent
                | Operation::GetAndPut
                | Operation::ReplaceIfValue
        );
        let compares = matches!(
            c.operation,
            Operation::ReplaceIfValue | Operation::RemoveIfValue
        );
        if c.id != id
            || c.worker > 1
            || c.position != positions[c.worker]
            || !byte_string(&c.key)
            || c.value.as_ref().is_some_and(|v| !byte_string(v))
            || c.expected.as_ref().is_some_and(|v| !byte_string(v))
            || writes != c.value.is_some()
            || compares != c.expected.is_some()
        {
            return Err("native command identity/order/shape bound".into());
        }
        positions[c.worker] += 1;
        keys.insert(&c.key);
    }
    if positions.contains(&0) || keys.len() > 4 {
        return Err("native worker/key bound".into());
    }
    let mut starts = vec![None; count];
    let mut ends = vec![None; count];
    for (index, event) in history.raw.iter().enumerate() {
        if event.stamp != index as u64 + 1
            || event.call_id >= count
            || event.worker != history.program[event.call_id].worker
        {
            return Err("native raw stamp/call identity".into());
        }
        match event.phase {
            Phase::Invoke => {
                if event.value.is_some()
                    || event.boolean.is_some()
                    || starts[event.call_id].replace(event.stamp).is_some()
                {
                    return Err("duplicate/forged native invocation".into());
                }
            }
            Phase::Return => {
                let bool_result = matches!(
                    history.program[event.call_id].operation,
                    Operation::ContainsKey | Operation::ReplaceIfValue | Operation::RemoveIfValue
                );
                if starts[event.call_id].is_none()
                    || ends[event.call_id].replace(event).is_some()
                    || event.boolean.is_some() != bool_result
                    || bool_result && event.value.is_some()
                    || event.value.as_ref().is_some_and(|v| !byte_string(v))
                {
                    return Err("native return interval/type/duplicate".into());
                }
            }
        }
    }
    let mut calls = Vec::new();
    let mut previous = [0; 2];
    for c in &history.program {
        let start = starts[c.id].ok_or("missing native invocation")?;
        let end = ends[c.id].ok_or("missing native return")?;
        if start <= previous[c.worker] || start >= end.stamp {
            return Err("native worker interval order".into());
        }
        previous[c.worker] = end.stamp;
        calls.push(Call {
            start,
            end: end.stamp,
            result: ResultValue {
                value: end.value.clone(),
                boolean: end.boolean,
            },
        });
    }
    let overlap = calls.iter().enumerate().any(|(i, a)| {
        calls.iter().enumerate().any(|(j, b)| {
            history.program[i].worker != history.program[j].worker
                && a.start < b.end
                && b.start < a.end
        })
    });
    if history.overlap != overlap {
        return Err("native overlap witness differs".into());
    }
    Ok(calls)
}
fn apply(map: &mut BTreeMap<String, String>, c: &Command) -> ResultValue {
    let old = map.get(&c.key).cloned();
    let mut boolean = None;
    let value = match c.operation {
        Operation::Get => old,
        Operation::ContainsKey => {
            boolean = Some(old.is_some());
            None
        }
        Operation::Put | Operation::GetAndPut => {
            map.insert(c.key.clone(), c.value.clone().expect("validated value"))
        }
        Operation::PutIfAbsent => {
            if old.is_none() {
                map.insert(c.key.clone(), c.value.clone().expect("validated value"));
            }
            old
        }
        Operation::ReplaceIfPresent => {
            if old.is_some() {
                map.insert(c.key.clone(), c.value.clone().expect("validated value"));
            }
            old
        }
        Operation::GetAndRemove => map.remove(&c.key),
        Operation::ReplaceIfValue | Operation::RemoveIfValue => {
            let matched = old.is_some() && old == c.expected;
            if matched {
                if c.operation == Operation::ReplaceIfValue {
                    map.insert(c.key.clone(), c.value.clone().expect("validated value"));
                } else {
                    map.remove(&c.key);
                }
            }
            boolean = Some(matched);
            None
        }
    };
    ResultValue { value, boolean }
}
struct Search<'a> {
    history: &'a History,
    calls: &'a [Call],
    predecessors: Vec<u32>,
    target: BTreeMap<String, String>,
    work: u64,
    limit: u64,
    exhausted: bool,
}
impl Search<'_> {
    fn visit(&mut self, map: BTreeMap<String, String>, done: u32) -> bool {
        if self.work == self.limit {
            self.exhausted = true;
            return false;
        }
        self.work += 1;
        if done == (1 << self.calls.len()) - 1 {
            return map == self.target;
        }
        for id in 0..self.calls.len() {
            let bit = 1 << id;
            if done & bit != 0 || self.predecessors[id] & done != self.predecessors[id] {
                continue;
            }
            let mut next = map.clone();
            if apply(&mut next, &self.history.program[id]) != self.calls[id].result {
                continue;
            }
            if self.visit(next, done | bit) {
                return true;
            }
            if self.exhausted {
                return false;
            }
        }
        false
    }
}
pub fn check(history: &History, max_work: u64) -> Result<Verdict, Box<dyn Error>> {
    if !(1..=1_000_000).contains(&max_work) {
        return Err("native search work bound".into());
    }
    let calls = normalize(history)?;
    let target = final_map(history)?;
    let mut predecessors = vec![0; calls.len()];
    for (i, a) in calls.iter().enumerate() {
        for (j, b) in calls.iter().enumerate() {
            if a.end < b.start
                || history.program[i].worker == history.program[j].worker
                    && history.program[i].position < history.program[j].position
            {
                predecessors[j] |= 1 << i;
            }
        }
    }
    let mut search = Search {
        history,
        calls: &calls,
        predecessors,
        target,
        work: 0,
        limit: max_work,
        exhausted: false,
    };
    Ok(if search.visit(BTreeMap::new(), 0) {
        Verdict::Linearizable
    } else if search.exhausted {
        Verdict::Exhausted
    } else {
        Verdict::NonLinearizable
    })
}
pub fn validate(histories: &Histories) -> Result<(), Box<dyn Error>> {
    if histories.schema != "imap075-native-histories-v1"
        || histories.scope != "eternal_fixed_keys_two_clients_single_member"
        || histories.artifact_sha256 != crate::imap_pre074_075::HAZELCAST_SHA256
        || histories.histories.len() != 2
    {
        return Err("native history profile/pin differs".into());
    }
    for (history, seed) in histories.histories.iter().zip([117, 118]) {
        if history.seed != seed
            || !(1..=8).contains(&history.attempts)
            || !uuid(&history.member_uuid)
            || history.clients.len() != 2
            || history.clients.iter().any(|v| !uuid(v))
            || history.clients[0] == history.clients[1]
            || !history.overlap
            || history.program.len() != 12
            || history.verdict != "linearizable"
            || !(1..=100_000).contains(&history.work)
            || check(history, 100_000)? != Verdict::Linearizable
        {
            return Err("native history failed independent qualification".into());
        }
        let operations = history
            .program
            .iter()
            .map(|c| format!("{:?}", c.operation))
            .collect::<BTreeSet<_>>();
        if operations.len() != 9 {
            return Err("native concurrent operation coverage missing".into());
        }
        let calls = normalize(history)?;
        let mut map = BTreeMap::new();
        let mut applied = BTreeSet::new();
        if history.witness.len() != calls.len() {
            return Err("native witness incomplete".into());
        }
        for &id in &history.witness {
            if id >= calls.len()
                || !applied.insert(id)
                || calls.iter().enumerate().any(|(i, c)| {
                    c.end < calls[id].start && !applied.contains(&i)
                        || history.program[i].worker == history.program[id].worker
                            && history.program[i].position < history.program[id].position
                            && !applied.contains(&i)
                })
                || apply(&mut map, &history.program[id]) != calls[id].result
            {
                return Err("native witness violates result/order".into());
            }
        }
        if map != final_map(history)? {
            return Err("native witness final state differs".into());
        }
        let expected_events = history
            .program
            .iter()
            .zip(&calls)
            .filter(|(c, call)| match c.operation {
                Operation::Put | Operation::GetAndPut => true,
                Operation::PutIfAbsent => call.result.value.is_none(),
                Operation::ReplaceIfPresent | Operation::GetAndRemove => {
                    call.result.value.is_some()
                }
                Operation::ReplaceIfValue | Operation::RemoveIfValue => {
                    call.result.boolean == Some(true)
                }
                _ => false,
            })
            .count();
        if history.native_events.len() != expected_events
            || history.native_events.iter().any(|e| {
                !matches!(e.kind.as_str(), "added" | "updated" | "removed")
                    || !byte_string(&e.key)
                    || !history.program.iter().any(|c| c.key == e.key)
                    || e.value.as_ref().is_some_and(|v| !byte_string(v))
                    || e.old_value.as_ref().is_some_and(|v| !byte_string(v))
            })
        {
            return Err("native event stream incomplete/foreign".into());
        }
    }
    Ok(())
}
pub fn observe(root: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let bytes = crate::imap_native_probe_075::execute_native(
        root,
        "io.hydracache.imap.hazelcast.NativeConcurrentProbeMain075",
    )?;
    let histories: Histories = serde_json::from_slice(&bytes)?;
    validate(&histories)?;
    Ok(bytes)
}
pub fn replay(saved: &Histories, fresh: &Histories) -> Result<(), Box<dyn Error>> {
    validate(saved)?;
    validate(fresh)?;
    if saved
        .histories
        .iter()
        .zip(&fresh.histories)
        .any(|(a, b)| a.seed != b.seed || a.program != b.program)
    {
        return Err("native replay program/seed changed".into());
    }
    Ok(())
}
pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args != ["--release", "0.75"] {
        return Err("requires exactly --release 0.75".into());
    }
    println!(
        "{}",
        String::from_utf8(observe(&crate::doc_check::find_repo_root()?)?)?
    );
    Ok(())
}
