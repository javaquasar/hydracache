//! Development-only, bounded concurrent histories over the actual local IMap service.
//!
//! Gates control invocation intervals. The independent checker accepts either serial
//! order when calls overlap; scheduler winners are not cross-language observations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::task::{Context, Poll};
use std::thread;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::task::noop_waker_ref;

use crate::value_plane_async_075::{
    AsyncIMapFacade, AsyncMapBackend, AsyncOutcome, BackendRecovery, CancellationDisposition,
    InvocationClock, InvocationContext,
};
use crate::value_plane_collection_075::CollectionBounds;
use crate::value_plane_differential_075::DifferentialError;
use crate::value_plane_model_075::MutationIdentity;
use crate::value_plane_service_075::{
    MapServiceError, ServiceCommand, ServiceRecovery, ServiceResult, ServiceTtl, UnifiedIMapService,
};
use crate::value_plane_service_async_075::UnifiedServiceAsyncBackend;

pub const SCHEDULE_HEADER: &str = "imap-api-schedule-v1";
pub const MAX_SCHEDULE_ROUNDS: usize = 64;
pub const MAX_SCHEDULE_BYTES: usize = 16 * 1024;
pub const MAX_HISTORY_BYTES: usize = 256 * 1024;
const WAIT_BOUND: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleRound {
    pub id: u64,
    pub kind: String,
    pub order: String,
    pub left: u8,
    pub right: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrentSchedule {
    pub seed: u64,
    pub rounds: Vec<ScheduleRound>,
}

impl ConcurrentSchedule {
    pub fn encode(&self) -> String {
        let mut output = format!("{SCHEDULE_HEADER}\t{}\n", self.seed);
        for round in &self.rounds {
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                round.id, round.kind, round.order, round.left, round.right
            ));
        }
        output
    }

    pub fn parse(source: &str) -> Result<Self, DifferentialError> {
        if source.len() > MAX_SCHEDULE_BYTES || !source.is_ascii() || source.contains('\r') {
            return bad("invalid or oversized concurrent schedule");
        }
        let mut lines = source.lines();
        let seed = lines
            .next()
            .and_then(|line| line.strip_prefix(&format!("{SCHEDULE_HEADER}\t")))
            .ok_or_else(|| DifferentialError("unsupported concurrent schedule header".into()))?;
        let seed = positive_number(seed, true)?;
        let mut rounds = Vec::new();
        let mut ids = std::collections::BTreeSet::new();
        for line in lines {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 5 || rounds.len() == MAX_SCHEDULE_ROUNDS {
                return bad("concurrent schedule field or round bound");
            }
            let id = positive_number(fields[0], false)?;
            let left = positive_number(fields[3], false)?;
            let right = positive_number(fields[4], false)?;
            if !ids.insert(id)
                || !matches!(
                    fields[1],
                    "compare" | "duplicate" | "expiry" | "cancel_before" | "cancel_after"
                )
                || !matches!(fields[2], "overlap" | "left_first" | "right_first")
                || left > 255
                || right > 255
                || left == right
                || (fields[1].starts_with("cancel_") && fields[2] != "left_first")
            {
                return bad("invalid concurrent round");
            }
            rounds.push(ScheduleRound {
                id,
                kind: fields[1].into(),
                order: fields[2].into(),
                left: left as u8,
                right: right as u8,
            });
        }
        if rounds.is_empty() {
            return bad("concurrent schedule has no rounds");
        }
        Ok(Self { seed, rounds })
    }
}

pub fn generated_schedule(
    seed: u64,
    random_rounds: usize,
) -> Result<ConcurrentSchedule, DifferentialError> {
    if seed > i64::MAX as u64 || random_rounds > MAX_SCHEDULE_ROUNDS - 11 {
        return bad("concurrent generator bound");
    }
    let mut rounds = Vec::new();
    for kind in ["compare", "duplicate", "expiry"] {
        for order in ["overlap", "left_first", "right_first"] {
            rounds.push(ScheduleRound {
                id: rounds.len() as u64 + 1,
                kind: kind.into(),
                order: order.into(),
                left: 1,
                right: 2,
            });
        }
    }
    for kind in ["cancel_before", "cancel_after"] {
        rounds.push(ScheduleRound {
            id: rounds.len() as u64 + 1,
            kind: kind.into(),
            order: "left_first".into(),
            left: 1,
            right: 2,
        });
    }
    let mut state = seed.max(1);
    for _ in 0..random_rounds {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let kind = ["compare", "duplicate", "expiry"][(state % 3) as usize];
        let order = ["overlap", "left_first", "right_first"][((state >> 8) % 3) as usize];
        let left = ((state >> 16) % 254 + 1) as u8;
        let right = left % 255 + 1;
        rounds.push(ScheduleRound {
            id: rounds.len() as u64 + 1,
            kind: kind.into(),
            order: order.into(),
            left,
            right,
        });
    }
    Ok(ConcurrentSchedule { seed, rounds })
}

/// Fifteen bounded TSV fields expose outcomes and real-time edges, not only final hashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrentHistory {
    pub id: u64,
    pub kind: String,
    pub order: String,
    pub begin: [u64; 2],
    pub end: [u64; 2],
    pub results: [String; 2],
    pub snapshot: String,
    pub events: String,
    pub replay: String,
    pub query: String,
    pub asynchronous: String,
    pub resources: String,
}

impl ConcurrentHistory {
    pub fn encode(&self) -> String {
        [
            self.id.to_string(),
            self.kind.clone(),
            self.order.clone(),
            self.begin[0].to_string(),
            self.end[0].to_string(),
            self.results[0].clone(),
            self.begin[1].to_string(),
            self.end[1].to_string(),
            self.results[1].clone(),
            self.snapshot.clone(),
            self.events.clone(),
            self.replay.clone(),
            self.query.clone(),
            self.asynchronous.clone(),
            self.resources.clone(),
        ]
        .join("\t")
    }

    pub fn parse(line: &str) -> Result<Self, DifferentialError> {
        if line.len() > 4096 || !line.is_ascii() || line.contains(['\r', '\n']) {
            return bad("invalid concurrent history line");
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 15 || fields.iter().any(|field| field.is_empty()) {
            return bad("concurrent history requires fifteen nonempty fields");
        }
        Ok(Self {
            id: positive_number(fields[0], false)?,
            kind: fields[1].into(),
            order: fields[2].into(),
            begin: [
                positive_number(fields[3], true)?,
                positive_number(fields[6], true)?,
            ],
            end: [
                positive_number(fields[4], true)?,
                positive_number(fields[7], true)?,
            ],
            results: [fields[5].into(), fields[8].into()],
            snapshot: fields[9].into(),
            events: fields[10].into(),
            replay: fields[11].into(),
            query: fields[12].into(),
            asynchronous: fields[13].into(),
            resources: fields[14].into(),
        })
    }
}

pub fn parse_histories(
    source: &[u8],
    expected: usize,
) -> Result<Vec<ConcurrentHistory>, DifferentialError> {
    if source.len() > MAX_HISTORY_BYTES || !source.is_ascii() || expected > MAX_SCHEDULE_ROUNDS {
        return bad("invalid or oversized concurrent history transcript");
    }
    let text = std::str::from_utf8(source).map_err(|error| DifferentialError(error.to_string()))?;
    let histories = text
        .lines()
        .map(ConcurrentHistory::parse)
        .collect::<Result<Vec<_>, _>>()?;
    if histories.len() != expected {
        return bad("concurrent history count mismatch");
    }
    Ok(histories)
}

/// Enumerates the two legal serial orders, constrained by the recorded invocation intervals.
pub fn validate_history(
    round: &ScheduleRound,
    actual: &ConcurrentHistory,
) -> Result<(), DifferentialError> {
    if actual.id != round.id || actual.kind != round.kind || actual.order != round.order {
        return bad("concurrent history identity mismatch");
    }
    if round.kind.starts_with("cancel_") {
        let mut expected = expected_async(round);
        expected.id = actual.id;
        return if actual == &expected {
            Ok(())
        } else {
            bad(&format!(
                "illegal cancellation/admission history; actual={}; expected={}",
                preview(&actual.encode()),
                preview(&expected.encode())
            ))
        };
    }
    let mut tickets = [
        actual.begin[0],
        actual.end[0],
        actual.begin[1],
        actual.end[1],
    ];
    tickets.sort_unstable();
    if tickets != [1, 2, 3, 4] || (0..2).any(|index| actual.begin[index] >= actual.end[index]) {
        return bad("invalid concurrent invocation tickets");
    }
    let ordered = match round.order.as_str() {
        "overlap" => actual.begin[0].max(actual.begin[1]) < actual.end[0].min(actual.end[1]),
        "left_first" => actual.end[0] < actual.begin[1],
        "right_first" => actual.end[1] < actual.begin[0],
        _ => false,
    };
    if !ordered {
        return bad("history violates explicit schedule checkpoints");
    }
    for order in [[0, 1], [1, 0]] {
        if actual.end[order[1]] < actual.begin[order[0]] {
            continue;
        }
        let mut expected = expected_race(round, order);
        expected.begin = actual.begin;
        expected.end = actual.end;
        if expected == *actual {
            return Ok(());
        }
    }
    bad(&format!("concurrent history has no legal serial order; actual={}; left-before-right={}; right-before-left={}", preview(&actual.encode()), preview(&expected_race(round, [0, 1]).encode()), preview(&expected_race(round, [1, 0]).encode())))
}

pub fn validate_histories(
    schedule: &ConcurrentSchedule,
    histories: &[ConcurrentHistory],
) -> Result<(), DifferentialError> {
    if schedule.rounds.len() != histories.len() {
        return bad("concurrent qualification count mismatch");
    }
    for (round, history) in schedule.rounds.iter().zip(histories) {
        validate_history(round, history).map_err(|error| {
            DifferentialError(format!("round {} {}: {}", round.id, round.kind, error))
        })?;
    }
    Ok(())
}

fn base_history(round: &ScheduleRound) -> ConcurrentHistory {
    ConcurrentHistory {
        id: round.id,
        kind: round.kind.clone(),
        order: round.order.clone(),
        begin: [1, 3],
        end: [2, 4],
        results: [String::new(), String::new()],
        snapshot: String::new(),
        events: "[]".into(),
        replay: String::new(),
        query: String::new(),
        asynchronous: "none".into(),
        resources: String::new(),
    }
}

fn expected_race(round: &ScheduleRound, order: [usize; 2]) -> ConcurrentHistory {
    let mut history = base_history(round);
    let winner = order[0];
    if round.kind == "expiry" {
        let absent = "absent|~|~|~|0|0|[]".to_owned();
        history.results = [absent.clone(), absent.clone()];
        history.snapshot = "[]".into();
        history.events = "[expired/6b/00/~/3/2]".into();
        history.replay = absent;
        history.query = "unknown|~|~|~|0|0|[]".into();
        history.resources = "[0,0,1,0,1,0]".into();
        return history;
    }
    let value = if round.kind == "duplicate" || winner == 0 {
        round.left
    } else {
        round.right
    };
    let applied = "replaced|00|3|10|0|1|[]".to_owned();
    history.results[winner] = applied.clone();
    history.results[1 - winner] = if round.kind == "duplicate" {
        applied.clone()
    } else {
        format!("mismatch|{value:02x}|3|10|0|0|[]")
    };
    history.snapshot = format!("[6b/{value:02x}/3/10]");
    history.events = format!("[updated/6b/00/{value:02x}/3/2]");
    history.replay = applied;
    history.query = "completed|00|3|10|0|1|[]".into();
    history.resources = format!(
        "[1,1,{},0,1,0]",
        if round.kind == "duplicate" { 2 } else { 3 }
    );
    history
}

fn expected_async(round: &ScheduleRound) -> ConcurrentHistory {
    let mut history = base_history(round);
    if round.kind == "cancel_before" {
        history.begin[1] = 0;
        history.end[1] = 0;
        history.results = ["not_applied|~|~|~|0|0|[]".into(), "-".into()];
        history.snapshot = "[6b/00/2/10]".into();
        history.replay = "not_run".into();
        history.query = "unknown|~|~|~|0|0|[]".into();
        history.asynchronous = "cancelled_before/not_applied/unknown".into();
        history.resources = "[1,1,1,0,1,0]".into();
    } else {
        history.results = [
            "outcome_unknown|~|~|~|0|0|[]".into(),
            "applied|~|3|10|0|1|[]".into(),
        ];
        history.snapshot = format!("[6b/{:02x}/3/10]", round.left);
        history.events = format!("[updated/6b/00/{:02x}/3/2]", round.left);
        history.replay = "applied|~|3|10|0|1|[]".into();
        history.query = "completed|~|3|10|0|1|[]".into();
        history.asynchronous = "cancelled_after/outcome_unknown/pending".into();
        history.resources = "[1,1,2,0,1,0]".into();
    }
    history
}

pub fn execute_concurrent_rust(
    schedule: &ConcurrentSchedule,
) -> Result<Vec<ConcurrentHistory>, DifferentialError> {
    ConcurrentSchedule::parse(&schedule.encode())?;
    schedule
        .rounds
        .iter()
        .map(|round| {
            let service = setup_service()?;
            if round.kind.starts_with("cancel_") {
                execute_async_round(round, service)
            } else {
                execute_race_round(round, service)
            }
        })
        .collect()
}

fn setup_service() -> Result<UnifiedIMapService, DifferentialError> {
    let service = UnifiedIMapService::new(CollectionBounds::default(), 1).map_err(domain)?;
    let mut listener = ServiceCommand::point("entry_listener", b"");
    listener.listener_id = 1;
    listener.include_value = true;
    service.execute(listener).map_err(domain)?;
    let mut initial = ServiceCommand::point("put", b"k");
    initial.value = Some(vec![0]);
    initial.ttl = ServiceTtl::ExpireAfter(10);
    initial.identity = Some(MutationIdentity::new("setup", 1));
    service.execute(initial).map_err(domain)?;
    service.drain_listener(1).map_err(domain)?;
    Ok(service)
}

fn race_command(round: &ScheduleRound, index: usize) -> ServiceCommand {
    let mut command = ServiceCommand::point(
        match round.kind.as_str() {
            "compare" => "conditional_replace",
            "duplicate" => "get_and_put",
            _ => "get",
        },
        b"k",
    );
    command.expected = Some(vec![0]);
    command.value = Some(vec![if index == 0 || round.kind == "duplicate" {
        round.left
    } else {
        round.right
    }]);
    command.ttl = ServiceTtl::Preserve;
    command.identity = Some(MutationIdentity::new(
        "race",
        if round.kind == "duplicate" {
            1
        } else {
            index as u64 + 1
        },
    ));
    command.now = if round.kind == "expiry" { 10 } else { 0 };
    command
}

fn execute_race_round(
    round: &ScheduleRound,
    service: UnifiedIMapService,
) -> Result<ConcurrentHistory, DifferentialError> {
    let tickets = Arc::new(AtomicU64::new(1));
    let (ready_tx, ready_rx) = mpsc::channel();
    let (started_tx, started_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let mut starts = Vec::new();
    let mut applies = Vec::new();
    let mut workers = Vec::new();
    for index in 0..2 {
        let (start_tx, start_rx) = mpsc::channel();
        let (apply_tx, apply_rx) = mpsc::channel();
        starts.push(start_tx);
        applies.push(apply_tx);
        let service = service.clone();
        let command = race_command(round, index);
        let tickets = Arc::clone(&tickets);
        let ready = ready_tx.clone();
        let started = started_tx.clone();
        let done = done_tx.clone();
        workers.push(thread::spawn(
            move || -> Result<(u64, u64, String), DifferentialError> {
                ready.send(index).map_err(channel)?;
                start_rx.recv_timeout(WAIT_BOUND).map_err(channel)?;
                let begin = tickets.fetch_add(1, Ordering::SeqCst);
                started.send(index).map_err(channel)?;
                apply_rx.recv_timeout(WAIT_BOUND).map_err(channel)?;
                let result = result_text(service.execute(command));
                let end = tickets.fetch_add(1, Ordering::SeqCst);
                done.send(index).map_err(channel)?;
                Ok((begin, end, result))
            },
        ));
    }
    for _ in 0..2 {
        ready_rx.recv_timeout(WAIT_BOUND).map_err(channel)?;
    }
    if round.order == "overlap" {
        for start in &starts {
            start.send(()).map_err(channel)?;
        }
        for _ in 0..2 {
            started_rx.recv_timeout(WAIT_BOUND).map_err(channel)?;
        }
        for apply in &applies {
            apply.send(()).map_err(channel)?;
        }
        for _ in 0..2 {
            done_rx.recv_timeout(WAIT_BOUND).map_err(channel)?;
        }
    } else {
        let first = usize::from(round.order == "right_first");
        for index in [first, 1 - first] {
            starts[index].send(()).map_err(channel)?;
            started_rx.recv_timeout(WAIT_BOUND).map_err(channel)?;
            applies[index].send(()).map_err(channel)?;
            done_rx.recv_timeout(WAIT_BOUND).map_err(channel)?;
        }
    }
    let mut history = base_history(round);
    for (index, worker) in workers.into_iter().enumerate() {
        let (begin, end, result) = worker
            .join()
            .map_err(|_| DifferentialError("concurrent worker panicked".into()))??;
        history.begin[index] = begin;
        history.end[index] = end;
        history.results[index] = result;
    }
    let winner = if history.results[0].starts_with("replaced|") || round.kind != "compare" {
        0
    } else {
        1
    };
    let replay_command = race_command(round, winner);
    history.replay = result_text(service.execute(replay_command.clone()));
    history.query = query_text(
        &service,
        replay_command.identity.unwrap(),
        replay_command.now,
    );
    finish_history(&service, &mut history, replay_command.now)?;
    Ok(history)
}

/// This gate wraps the real service admission ledger. Its admitted worker survives caller
/// cancellation, matching an operation whose server work has crossed the admission fence.
struct GatedBackend {
    service: UnifiedIMapService,
    release: std::sync::Mutex<Option<mpsc::Receiver<()>>>,
    worker: std::sync::Mutex<Option<thread::JoinHandle<Result<ServiceResult, MapServiceError>>>>,
}

#[async_trait]
impl AsyncMapBackend for GatedBackend {
    type Command = ServiceCommand;
    type Output = ServiceResult;
    type Error = MapServiceError;
    fn recover(
        &self,
        command: &ServiceCommand,
        identity: &MutationIdentity,
    ) -> Result<BackendRecovery<ServiceResult>, MapServiceError> {
        Ok(match self.service.recover_command(command, identity)? {
            ServiceRecovery::Unknown => BackendRecovery::NotRetained,
            ServiceRecovery::Pending => BackendRecovery::Unavailable,
            ServiceRecovery::Completed(result) => BackendRecovery::Retained(*result),
        })
    }
    async fn execute(
        &self,
        command: &ServiceCommand,
        context: InvocationContext,
    ) -> Result<ServiceResult, MapServiceError> {
        context
            .admit()
            .map_err(|_| MapServiceError::InvalidArgument)?;
        let mut command = command.clone();
        command.now = context.now();
        command.deadline = context.deadline;
        command.identity = Some(context.identity.clone());
        self.service.reserve_operation(context.identity, &command)?;
        let release = self
            .release
            .lock()
            .unwrap()
            .take()
            .ok_or(MapServiceError::InvalidArgument)?;
        let service = self.service.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        *self.worker.lock().unwrap() = Some(thread::spawn(move || {
            release
                .recv_timeout(WAIT_BOUND)
                .map_err(|_| MapServiceError::InvalidArgument)?;
            let result = service.execute_reserved(command);
            let _ = sender.send(result.clone());
            result
        }));
        receiver
            .await
            .map_err(|_| MapServiceError::InvalidArgument)?
    }
}

fn execute_async_round(
    round: &ScheduleRound,
    service: UnifiedIMapService,
) -> Result<ConcurrentHistory, DifferentialError> {
    let (release_tx, release_rx) = mpsc::channel();
    let backend = Arc::new(GatedBackend {
        service: service.clone(),
        release: std::sync::Mutex::new(Some(release_rx)),
        worker: std::sync::Mutex::new(None),
    });
    let clock = Arc::new(InvocationClock::default());
    let facade = AsyncIMapFacade::new(Arc::clone(&backend), Arc::clone(&clock), 1)
        .map_err(|error| DifferentialError(format!("async facade: {error:?}")))?;
    let identity = MutationIdentity::new("race", 1);
    let mut command = ServiceCommand::point("set", b"k");
    command.value = Some(vec![round.left]);
    command.ttl = ServiceTtl::Preserve;
    let (handle, mut future) = facade
        .invoke(identity.clone(), 100, command.clone())
        .map_err(|error| DifferentialError(format!("async invocation: {error:?}")))?;
    let mut context = Context::from_waker(noop_waker_ref());
    let mut history = base_history(round);
    if round.kind == "cancel_before" {
        if handle.cancel() != CancellationDisposition::NotApplied {
            return bad("pre-admission cancellation disposition");
        }
        history.results[0] = poll_outcome(future.as_mut().poll(&mut context))?;
        history.begin[1] = 0;
        history.end[1] = 0;
        history.results[1] = "-".into();
        history.replay = "not_run".into();
        history.asynchronous = "cancelled_before/not_applied/unknown".into();
        if backend.worker.lock().unwrap().is_some() {
            return bad("cancelled invocation started backend worker");
        }
    } else {
        if future.as_mut().poll(&mut context).is_ready() {
            return bad("admission gate did not suspend invocation");
        }
        let pending = query_text(&service, identity.clone(), 0);
        if pending != "pending|~|~|~|0|0|[]" {
            return bad("admitted operation absent from shared pending registry");
        }
        if handle.cancel() != CancellationDisposition::OutcomeUnknown {
            return bad("post-admission cancellation disposition");
        }
        history.results[0] = poll_outcome(future.as_mut().poll(&mut context))?;
        release_tx.send(()).map_err(channel)?;
        let worker = backend
            .worker
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| DifferentialError("missing admitted worker".into()))?;
        history.results[1] = result_text(
            worker
                .join()
                .map_err(|_| DifferentialError("admitted worker panicked".into()))?,
        );
        let retry = AsyncIMapFacade::new(
            Arc::new(UnifiedServiceAsyncBackend {
                service: service.clone(),
            }),
            clock,
            1,
        )
        .map_err(|error| DifferentialError(format!("retry facade: {error:?}")))?;
        let (_, mut replay) = retry
            .invoke(identity.clone(), 100, command)
            .map_err(|error| DifferentialError(format!("retry invocation: {error:?}")))?;
        history.replay = poll_outcome(replay.as_mut().poll(&mut context))?;
        history.asynchronous = "cancelled_after/outcome_unknown/pending".into();
    }
    if facade.inflight() != 0 {
        return bad("async invocation leaked inflight capacity");
    }
    history.query = query_text(&service, identity, 0);
    finish_history(&service, &mut history, 0)?;
    Ok(history)
}

fn poll_outcome(
    result: Poll<crate::value_plane_async_075::InvocationResult<ServiceResult, MapServiceError>>,
) -> Result<String, DifferentialError> {
    match result {
        Poll::Ready(result) => match result.outcome {
            AsyncOutcome::Completed(result) => Ok(result_text(result)),
            AsyncOutcome::NotApplied(
                crate::value_plane_async_075::InvocationRejection::Cancelled,
            ) => Ok("not_applied|~|~|~|0|0|[]".into()),
            AsyncOutcome::OutcomeUnknown => Ok("outcome_unknown|~|~|~|0|0|[]".into()),
            other => bad(&format!("unexpected async outcome: {other:?}")),
        },
        Poll::Pending => bad("controlled async checkpoint did not complete"),
    }
}

fn query_text(service: &UnifiedIMapService, identity: MutationIdentity, now: u64) -> String {
    let mut query = ServiceCommand::point("query_operation_outcome", b"");
    query.identity = Some(identity);
    query.now = now;
    result_text(service.execute(query))
}

fn finish_history(
    service: &UnifiedIMapService,
    history: &mut ConcurrentHistory,
    now: u64,
) -> Result<(), DifferentialError> {
    history.snapshot = format!(
        "[{}]",
        service
            .diagnostic_snapshot(now)
            .map_err(domain)?
            .iter()
            .map(|entry| format!(
                "{}/{}/{}/{}",
                hex(&entry.key),
                hex(&entry.value),
                entry.version,
                optional(entry.remaining_ttl)
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    history.events = format!(
        "[{}]",
        service
            .drain_listener(1)
            .map_err(domain)?
            .iter()
            .map(|event| format!(
                "{}/{}/{}/{}/{}/{}",
                event.kind,
                hex(&event.key),
                optional_hex(event.old_value.as_deref()),
                optional_hex(event.value.as_deref()),
                event.version,
                event.watermark
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    let stats = service.statistics();
    history.resources = format!(
        "[{},{},{},{},{},{}]",
        stats.live_entries,
        stats.value_bytes,
        stats.retained_outcomes,
        stats.inflight,
        stats.listeners,
        stats.queued_events
    );
    Ok(())
}

fn result_text(result: Result<ServiceResult, MapServiceError>) -> String {
    match result {
        Ok(result) => format!(
            "{}|{}|{}|{}|{}|{}|[]",
            result.kind,
            optional_hex(result.value.as_deref()),
            optional(result.version),
            optional(result.remaining_ttl),
            result.number,
            u8::from(result.flag)
        ),
        Err(error) => format!("error:{}|~|~|~|0|0|[]", error.kind()),
    }
}

/// Bounded deterministic ddmin removes whole rounds, then simplifies values and release order.
pub fn shrink_schedule(
    schedule: &ConcurrentSchedule,
    budget: usize,
    mut fails: impl FnMut(&ConcurrentSchedule) -> Result<bool, DifferentialError>,
) -> Result<ConcurrentSchedule, DifferentialError> {
    if budget == 0 || !fails(schedule)? {
        return Ok(schedule.clone());
    }
    let mut calls = 1;
    let mut current = schedule.clone();
    let mut partitions = 2;
    while current.rounds.len() > 1 && calls < budget {
        let chunk = current.rounds.len().div_ceil(partitions);
        let mut reduced = false;
        for start in (0..current.rounds.len()).step_by(chunk) {
            if calls == budget {
                break;
            }
            let mut candidate = current.clone();
            candidate
                .rounds
                .drain(start..(start + chunk).min(current.rounds.len()));
            if candidate.rounds.is_empty() {
                continue;
            }
            calls += 1;
            if fails(&candidate)? {
                current = candidate;
                partitions = partitions.saturating_sub(1).max(2);
                reduced = true;
                break;
            }
        }
        if !reduced {
            if partitions >= current.rounds.len() {
                break;
            }
            partitions = (partitions * 2).min(current.rounds.len());
        }
    }
    for index in 0..current.rounds.len() {
        if calls == budget {
            break;
        }
        let mut candidate = current.clone();
        candidate.rounds[index].left = 1;
        candidate.rounds[index].right = 2;
        candidate.rounds[index].order = "left_first".into();
        if candidate != current {
            calls += 1;
            if fails(&candidate)? {
                current = candidate;
            }
        }
    }
    Ok(current)
}

fn positive_number(text: &str, zero: bool) -> Result<u64, DifferentialError> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return bad("invalid concurrent numeric field");
    }
    let value = text
        .parse::<u64>()
        .map_err(|_| DifferentialError("concurrent numeric overflow".into()))?;
    if value > i64::MAX as u64 || (!zero && value == 0) {
        return bad("concurrent numeric bound");
    }
    Ok(value)
}
fn optional(value: Option<u64>) -> String {
    value.map_or_else(|| "~".into(), |value| value.to_string())
}
fn optional_hex(value: Option<&[u8]>) -> String {
    value.map_or_else(|| "~".into(), hex)
}
fn hex(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn domain(error: MapServiceError) -> DifferentialError {
    DifferentialError(error.to_string())
}
fn channel(error: impl std::fmt::Display) -> DifferentialError {
    DifferentialError(format!("concurrent checkpoint failed: {error}"))
}
fn bad<T>(message: &str) -> Result<T, DifferentialError> {
    Err(DifferentialError(message.into()))
}
fn preview(value: &str) -> String {
    value.chars().take(1024).collect()
}
