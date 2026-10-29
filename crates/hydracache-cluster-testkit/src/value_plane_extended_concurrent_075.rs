//! Real two-worker extended histories. The independent legal-history checker is Java;
//! these observers retain only invocation records, captured tokens and actual resource handles.
use crate::value_plane_collection_075::CollectionBounds;
use crate::value_plane_differential_075::{hex, DifferentialError};
use crate::value_plane_model_075::MutationIdentity;
use crate::value_plane_service_075::*;
use crate::value_plane_stream_075::DemandStream;
use futures_util::{task::noop_waker_ref, Stream};
use std::collections::{BTreeMap, BTreeSet};
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Barrier, Condvar, Mutex,
};
use std::task::{Context, Poll};
use std::time::Duration;

pub const SCHEDULE_HEADER: &str = "imap-extended-schedule-v1";
pub const HISTORY_HEADER: &str = "imap-extended-history-v1";
pub const MAX_SCHEDULE_BYTES: usize = 16 * 1024;
pub const MAX_HISTORY_BYTES: usize = 256 * 1024;
const FAMILIES: [&str; 5] = [
    "bulk_aba",
    "bulk_expiry",
    "retire_pending",
    "listener_cut",
    "checkpoint_lease",
];
const ORDERS: [&str; 3] = ["overlap", "left_first", "right_first"];
fn bad(message: &str) -> DifferentialError {
    DifferentialError(message.into())
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedRound {
    pub id: u32,
    pub family: String,
    pub order: String,
}
impl ExtendedRound {
    pub fn program(&self, worker: usize) -> Vec<&'static str> {
        match self.family.as_str() {
            "bulk_aba" => {
                if worker == 0 {
                    vec!["bulk"]
                } else {
                    vec!["remove", "put"]
                }
            }
            "bulk_expiry" => {
                if worker == 0 {
                    vec!["bulk"]
                } else {
                    vec!["expire"]
                }
            }
            "retire_pending" => {
                if worker == 0 {
                    vec!["reserve", "query", "finish"]
                } else {
                    vec!["retire", "query"]
                }
            }
            "listener_cut" => {
                if worker == 0 {
                    vec!["request"]
                } else {
                    vec!["snapshot", "put"]
                }
            }
            _ => {
                if worker == 0 {
                    vec!["close"]
                } else {
                    vec!["checkpoint"]
                }
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedSchedule {
    pub seed: i64,
    pub rounds: Vec<ExtendedRound>,
}
impl ExtendedSchedule {
    pub fn parse(source: &str) -> Result<Self, DifferentialError> {
        if source.len() > MAX_SCHEDULE_BYTES || !source.is_ascii() || source.contains('\r') {
            return Err(bad("schedule encoding/bound"));
        }
        let mut lines = source.strip_suffix('\n').unwrap_or(source).split('\n');
        let header = lines
            .next()
            .unwrap_or_default()
            .split('\t')
            .collect::<Vec<_>>();
        if header.len() != 2 || header[0] != SCHEDULE_HEADER {
            return Err(bad("schedule version"));
        }
        let seed = header[1].parse::<i64>().map_err(|_| bad("seed range"))?;
        if seed.to_string() != header[1] {
            return Err(bad("canonical seed"));
        }
        let mut rounds = Vec::new();
        let mut ids = BTreeSet::new();
        for line in lines {
            let f = line.split('\t').collect::<Vec<_>>();
            if f.len() != 3 || rounds.len() == 32 {
                return Err(bad("round fields/bound"));
            }
            let id = f[0].parse::<u32>().map_err(|_| bad("round id"))?;
            if id == 0
                || id > 32
                || id.to_string() != f[0]
                || !ids.insert(id)
                || !FAMILIES.contains(&f[1])
                || !ORDERS.contains(&f[2])
            {
                return Err(bad("round shape"));
            }
            rounds.push(ExtendedRound {
                id,
                family: f[1].into(),
                order: f[2].into(),
            });
        }
        if rounds.is_empty() {
            return Err(bad("empty schedule"));
        }
        Ok(Self { seed, rounds })
    }
    pub fn encode(&self) -> Result<String, DifferentialError> {
        if self.rounds.is_empty() || self.rounds.len() > 32 {
            return Err(bad("round bound"));
        }
        let mut ids = BTreeSet::new();
        for r in &self.rounds {
            if r.id == 0
                || r.id > 32
                || !ids.insert(r.id)
                || !FAMILIES.contains(&r.family.as_str())
                || !ORDERS.contains(&r.order.as_str())
            {
                return Err(bad("round shape"));
            }
        }
        let mut out = format!("{SCHEDULE_HEADER}\t{}\n", self.seed);
        for r in &self.rounds {
            out.push_str(&format!("{}\t{}\t{}\n", r.id, r.family, r.order));
        }
        Ok(out)
    }
}
pub fn generated_extended_schedule(
    seed: i64,
    count: usize,
) -> Result<ExtendedSchedule, DifferentialError> {
    if count == 0 || count > 32 {
        return Err(bad("generation bound"));
    }
    Ok(ExtendedSchedule {
        seed,
        rounds: (0..count)
            .map(|i| ExtendedRound {
                id: i as u32 + 1,
                family: FAMILIES[(seed.rem_euclid(5) as usize + i) % 5].into(),
                order: ORDERS[(seed.rem_euclid(3) as usize + i / 5) % 3].into(),
            })
            .collect(),
    })
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedCall {
    pub worker: usize,
    pub ordinal: usize,
    pub start: usize,
    pub end: usize,
    pub result: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedHistory {
    pub round: ExtendedRound,
    pub calls: Vec<ExtendedCall>,
    pub final_state: String,
}
pub fn encode_extended_histories(
    seed: i64,
    histories: &[ExtendedHistory],
) -> Result<String, DifferentialError> {
    if histories.is_empty() || histories.len() > 32 {
        return Err(bad("history count bound"));
    }
    for h in histories {
        if h.round.id == 0
            || h.round.id > 32
            || !FAMILIES.contains(&h.round.family.as_str())
            || !ORDERS.contains(&h.round.order.as_str())
        {
            return Err(bad("history round shape"));
        }
        if !h.final_state.is_ascii()
            || h.final_state.len() > 8192
            || h.calls.len() != h.round.program(0).len() + h.round.program(1).len()
        {
            return Err(bad("history shape/bound"));
        }
        let mut stamps = BTreeSet::new();
        let mut index = 0;
        for worker in 0..2 {
            let mut previous = 0;
            for ordinal in 0..h.round.program(worker).len() {
                let c = &h.calls[index];
                index += 1;
                if c.worker != worker
                    || c.ordinal != ordinal
                    || c.start <= previous
                    || c.end <= c.start
                    || c.end > 64
                    || !stamps.insert(c.start)
                    || !stamps.insert(c.end)
                    || !c.result.is_ascii()
                    || c.result.len() > 1024
                {
                    return Err(bad("history interval/result bound"));
                }
                previous = c.end;
            }
        }
        if h.round.order == "left_first"
            && h.calls[h.round.program(0).len() - 1].end >= h.calls[h.round.program(0).len()].start
            || h.round.order == "right_first"
                && h.calls.last().expect("bounded calls").end >= h.calls[0].start
        {
            return Err(bad("explicit worker order"));
        }
    }
    ExtendedSchedule {
        seed,
        rounds: histories.iter().map(|h| h.round.clone()).collect(),
    }
    .encode()?;
    let mut out = format!("{HISTORY_HEADER}\t{seed}\t{}\n", histories.len());
    for h in histories {
        let calls = h
            .calls
            .iter()
            .map(|c| {
                format!(
                    "{}.{}.{}.{}.{}",
                    c.worker,
                    c.ordinal,
                    c.start,
                    c.end,
                    hex(c.result.as_bytes())
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            h.round.id,
            h.round.family,
            h.round.order,
            calls,
            hex(h.final_state.as_bytes())
        ));
        if out.len() > MAX_HISTORY_BYTES {
            return Err(bad("history output bound"));
        }
    }
    Ok(out)
}
fn identity(client: &str, sequence: u64) -> MutationIdentity {
    MutationIdentity::new(client, sequence)
}
fn put(key: &[u8], value: &[u8], client: &str, sequence: u64) -> ServiceCommand {
    let mut c = ServiceCommand::point("put", key);
    c.value = Some(value.to_vec());
    c.identity = Some(identity(client, sequence));
    c.deadline = i64::MAX as u64;
    c
}
fn event(e: &ServiceEvent) -> String {
    format!(
        "{}/{}/{}/{}/{}",
        e.kind,
        hex(&e.key),
        e.old_value.as_deref().map_or("~".into(), hex),
        e.value.as_deref().map_or("~".into(), hex),
        e.watermark
    )
}
struct Flow {
    stream: DemandStream<ServiceListenerSource>,
    demand: usize,
    delivered: Vec<String>,
}
impl Flow {
    fn pump(&mut self) -> Result<(), MapServiceError> {
        let mut cx = Context::from_waker(noop_waker_ref());
        while self.demand > 0 {
            match Pin::new(&mut self.stream).poll_next(&mut cx) {
                Poll::Ready(Some(Ok(e))) => {
                    self.delivered.push(event(&e));
                    self.demand -= 1;
                }
                Poll::Pending => break,
                _ => return Err(MapServiceError::UnknownListener),
            }
        }
        Ok(())
    }
}
struct Owner {
    round: ExtendedRound,
    defect: bool,
    service: UnifiedIMapService,
    captured: BTreeMap<Vec<u8>, VersionToken>,
    listener_id: u64,
    listener: Mutex<Option<ListenerRegistration>>,
    flow: Mutex<Option<Flow>>,
    held: Mutex<Option<ListenerSnapshot>>,
    reserved: AtomicBool,
}
impl Owner {
    fn new(round: &ExtendedRound, defect: bool) -> Result<Self, DifferentialError> {
        let service = UnifiedIMapService::new(
            CollectionBounds {
                max_key_bytes: 16,
                max_value_bytes: 16,
                max_entries: 32,
                max_page_items: 8,
                max_scan_items: 32,
                max_response_bytes: 4096,
                max_bulk_mutation_items: 16,
            },
            1,
        )
        .map_err(|e| bad(e.kind()))?;
        let options = ServiceListenerOptions {
            include_value: true,
            max_value_bytes: 16,
            queue_capacity: 8,
            ..ServiceListenerOptions::default()
        };
        let (listener, flow, listener_id) = if round.family == "listener_cut" {
            let source =
                ServiceListenerSource::new(service.clone(), options).map_err(|e| bad(e.kind()))?;
            let id = source
                .registration_id()
                .ok_or_else(|| bad("listener registration"))?;
            (
                None,
                Some(Flow {
                    stream: DemandStream::new(source, 2).map_err(|_| bad("stream bound"))?,
                    demand: 0,
                    delivered: Vec::new(),
                }),
                id,
            )
        } else {
            let registration = service
                .register_listener(options)
                .map_err(|e| bad(e.kind()))?;
            let id = registration.id();
            (Some(registration), None, id)
        };
        let mut owner = Self {
            round: round.clone(),
            defect,
            service,
            captured: BTreeMap::new(),
            listener_id,
            listener: Mutex::new(listener),
            flow: Mutex::new(flow),
            held: Mutex::new(None),
            reserved: AtomicBool::new(false),
        };
        if round.family != "retire_pending" {
            let mut c = put(b"a", &[0], "setup", 1);
            if round.family == "bulk_expiry" {
                c.ttl = ServiceTtl::ExpireAfter(1);
            }
            owner.service.execute(c).map_err(|e| bad(e.kind()))?;
            owner.capture(b"a")?;
            if round.family != "checkpoint_lease" {
                owner
                    .service
                    .execute(put(b"b", &[0], "setup", 2))
                    .map_err(|e| bad(e.kind()))?;
                owner.capture(b"b")?;
            }
        }
        if round.family == "listener_cut" {
            let mut flow = owner.flow.lock().unwrap();
            let flow = flow.as_mut().expect("flow");
            flow.demand = 1;
            flow.pump().map_err(|e| bad(e.kind()))?;
            if flow.delivered.len() != 1 {
                return Err(bad("initial demand"));
            }
            flow.delivered.clear();
        } else {
            owner
                .service
                .drain_listener(listener_id)
                .map_err(|e| bad(e.kind()))?;
        }
        if round.family == "checkpoint_lease" {
            *owner.held.lock().unwrap() = Some(
                owner
                    .service
                    .resync_listener(listener_id, 0, 8, 4096)
                    .map_err(|e| bad(e.kind()))?,
            );
            owner.listener.lock().unwrap().take();
        }
        Ok(owner)
    }
    fn capture(&mut self, key: &[u8]) -> Result<(), DifferentialError> {
        let r = self
            .service
            .execute(ServiceCommand::point("get_entry_view", key))
            .map_err(|e| bad(e.kind()))?;
        self.captured
            .insert(key.to_vec(), r.token.ok_or_else(|| bad("missing token"))?);
        Ok(())
    }
    fn action(&self, action: &str) -> Result<String, MapServiceError> {
        Ok(match action {
            "bulk" => {
                let mut entries = Vec::new();
                for (key, value) in [(b"a".as_slice(), 1), (b"b".as_slice(), 2)] {
                    let mut token = self.captured[key];
                    if self.defect && self.round.family == "bulk_aba" {
                        token = self
                            .service
                            .execute(ServiceCommand::point("get_entry_view", key))?
                            .token
                            .unwrap_or(token);
                    }
                    entries.push(VersionBulkEntry {
                        key: key.to_vec(),
                        token,
                        value: Some(vec![value]),
                        ttl: ServiceTtl::Preserve,
                    });
                }
                let r = self.service.bulk_version_conditional(
                    VersionBulkOperation::Replace,
                    &entries,
                    identity("bulk", 1),
                    0,
                    i64::MAX as u64,
                    false,
                )?;
                format!(
                    "{}:{}",
                    r.kind,
                    r.items
                        .iter()
                        .map(|i| i.kind.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
            "remove" => {
                let mut c = ServiceCommand::point("get_and_remove", b"a");
                c.identity = Some(identity("aba", 1));
                self.service.execute(c)?.kind
            }
            "put" => {
                let r = self
                    .service
                    .execute(if self.round.family == "listener_cut" {
                        put(b"c", &[3], "event", 1)
                    } else {
                        put(b"a", &[0], "aba", 2)
                    })?;
                if let Some(flow) = self.flow.lock().unwrap().as_mut() {
                    flow.pump()?;
                }
                r.kind
            }
            "expire" => {
                let r = self.service.expire_step(1, 1)?;
                format!(
                    "expiry:{}:{}:{}",
                    r.examined,
                    r.expired,
                    u8::from(r.pending)
                )
            }
            "reserve" => {
                let c = put(b"a", &[1], "race", 1);
                self.service
                    .reserve_operation(c.identity.clone().expect("identity"), &c)?;
                self.reserved.store(true, Ordering::SeqCst);
                "pending".into()
            }
            "finish" => {
                if self.reserved.load(Ordering::SeqCst) {
                    self.service
                        .execute_reserved(put(b"a", &[1], "race", 1))?
                        .kind
                } else {
                    "skipped".into()
                }
            }
            "query" => match self.service.query_operation_outcome(&identity("race", 1))? {
                OperationOutcomeQuery::Pending => "pending".into(),
                OperationOutcomeQuery::Unknown => "unknown".into(),
                OperationOutcomeQuery::Completed(r) => format!("completed:{}", r.kind),
            },
            "retire" => format!(
                "retired:{}",
                self.service.retire_outcomes("race", 1, 1)?.removed
            ),
            "request" => {
                let mut flow = self.flow.lock().unwrap();
                let flow = flow.as_mut().expect("flow");
                flow.demand += 1;
                flow.pump()?;
                "requested".into()
            }
            "snapshot" => {
                let mut lease = self.service.resync_listener(self.listener_id, 0, 8, 4096)?;
                let result = format!(
                    "snapshot:{}:{}",
                    lease.watermark,
                    lease
                        .entries
                        .iter()
                        .map(|e| hex(&e.key))
                        .collect::<Vec<_>>()
                        .join(",")
                );
                lease.close();
                result
            }
            "close" => {
                self.held.lock().unwrap().take();
                "closed".into()
            }
            "checkpoint" => {
                let _checkpoint = self.service.checkpoint()?;
                "checkpoint".into()
            }
            _ => return Err(MapServiceError::InvalidArgument),
        })
    }
    fn finish(&self) -> Result<String, DifferentialError> {
        let events = if self.listener.lock().unwrap().is_some() {
            self.service
                .drain_listener(self.listener_id)
                .map_err(|e| bad(e.kind()))?
                .iter()
                .map(event)
                .collect::<Vec<_>>()
                .join(",")
        } else {
            String::new()
        };
        let flow = {
            let mut flow = self.flow.lock().unwrap();
            if let Some(flow) = flow.as_mut() {
                flow.stream.cancel();
                flow.delivered.join(",")
            } else {
                String::new()
            }
        };
        self.listener.lock().unwrap().take();
        self.held.lock().unwrap().take();
        let now = u64::from(self.round.family == "bulk_expiry");
        let mut rows = Vec::new();
        for e in self
            .service
            .diagnostic_snapshot(now)
            .map_err(|e| bad(e.kind()))?
        {
            let mut c = ServiceCommand::point("get_entry_view", &e.key);
            c.now = now;
            let token = self.service.execute(c).map_err(|e| bad(e.kind()))?.token;
            rows.push(format!(
                "{}/{}/{}/{}",
                hex(&e.key),
                hex(&e.value),
                e.remaining_ttl
                    .map_or_else(|| "e".into(), |ticks| format!("x{ticks}")),
                u8::from(token.is_some() && token.as_ref() == self.captured.get(&e.key))
            ));
        }
        let stats = self.service.statistics();
        let usage = self.service.resource_usage();
        Ok(format!(
            "{}|{events}|{flow}|{}/{}/{}/{}/{}",
            rows.join(","),
            stats.retained_outcomes,
            stats.inflight,
            stats.listeners,
            stats.active_streams,
            u8::from(usage.resync != 0)
        ))
    }
}
pub fn execute_extended_concurrent(
    schedule: &ExtendedSchedule,
    defect: bool,
) -> Result<Vec<ExtendedHistory>, DifferentialError> {
    ExtendedSchedule::parse(&schedule.encode()?)?;
    let mut histories = Vec::new();
    for round in &schedule.rounds {
        let owner = Arc::new(Owner::new(round, defect)?);
        let stamps = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let barrier = Arc::new(Barrier::new(2));
        let done = Arc::new((Mutex::new([false; 2]), Condvar::new()));
        let mut workers = Vec::new();
        for worker in 0..2 {
            let (owner, stamps, calls, barrier, done) = (
                owner.clone(),
                stamps.clone(),
                calls.clone(),
                barrier.clone(),
                done.clone(),
            );
            let round = round.clone();
            workers.push(std::thread::spawn(
                move || -> Result<(), DifferentialError> {
                    if round.order
                        == if worker == 0 {
                            "right_first"
                        } else {
                            "left_first"
                        }
                    {
                        let (lock, wake) = &*done;
                        let (state, timeout) = wake
                            .wait_timeout_while(
                                lock.lock().unwrap(),
                                Duration::from_secs(3),
                                |done| !done[1 - worker],
                            )
                            .map_err(|_| bad("worker gate"))?;
                        if timeout.timed_out() && !state[1 - worker] {
                            return Err(bad("worker gate timeout"));
                        }
                    }
                    for (ordinal, operation) in round.program(worker).iter().enumerate() {
                        let start = stamps.fetch_add(1, Ordering::SeqCst) + 1;
                        if ordinal == 0 && round.order == "overlap" {
                            barrier.wait();
                        }
                        let result = owner
                            .action(operation)
                            .unwrap_or_else(|error| format!("error:{}", error.kind()));
                        let end = stamps.fetch_add(1, Ordering::SeqCst) + 1;
                        calls.lock().unwrap().push(ExtendedCall {
                            worker,
                            ordinal,
                            start,
                            end,
                            result,
                        });
                    }
                    done.0.lock().unwrap()[worker] = true;
                    done.1.notify_all();
                    Ok(())
                },
            ));
        }
        for worker in workers {
            worker.join().map_err(|_| bad("worker panic"))??;
        }
        let mut calls = calls.lock().unwrap().clone();
        calls.sort_by_key(|call| (call.worker, call.ordinal));
        histories.push(ExtendedHistory {
            round: round.clone(),
            calls,
            final_state: owner.finish()?,
        });
    }
    Ok(histories)
}
