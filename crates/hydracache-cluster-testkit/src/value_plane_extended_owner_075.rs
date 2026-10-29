//! Actual-owner adapter: retains only tokens and real listener/snapshot handles.
use super::*;
use crate::value_plane_collection_075::CollectionBounds;
use crate::value_plane_service_075::{
    ListenerRegistration, ListenerSnapshot, MapServiceError, OperationOutcomeQuery, ServiceCommand,
    ServiceListenerOptions, ServiceRecovery, ServiceResult, UnifiedIMapService, VersionBulkEntry,
    VersionBulkOperation, VersionToken,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

struct Actual {
    service: UnifiedIMapService,
    listener: ListenerRegistration,
    terminal_listener: bool,
    held: Option<ListenerSnapshot>,
    tokens: BTreeMap<String, Option<VersionToken>>,
    tick: u64,
}

fn register(service: &UnifiedIMapService) -> Result<ListenerRegistration, MapServiceError> {
    service.register_listener(ServiceListenerOptions {
        include_value: true,
        max_value_bytes: 8192,
        queue_capacity: 64,
        ..ServiceListenerOptions::default()
    })
}

pub fn execute_extended(corpus: &ExtendedCorpus) -> Result<Vec<Value>, DifferentialError> {
    execute(corpus, false)
}

/// Development process canary changes the real adapter's drained event stream.
pub fn execute_extended_drop_event(
    corpus: &ExtendedCorpus,
) -> Result<Vec<Value>, DifferentialError> {
    execute(corpus, true)
}

fn execute(corpus: &ExtendedCorpus, drop_event: bool) -> Result<Vec<Value>, DifferentialError> {
    corpus.validate()?;
    let service = UnifiedIMapService::new(
        CollectionBounds {
            max_key_bytes: 1024,
            max_value_bytes: 4096,
            max_entries: 256,
            max_page_items: 64,
            max_scan_items: 256,
            max_response_bytes: 1024 * 1024,
            max_bulk_mutation_items: 64,
        },
        1,
    )
    .map_err(|e| bad(e.kind()))?;
    let listener = register(&service).map_err(|e| bad(e.kind()))?;
    let mut actual = Actual {
        service,
        listener,
        terminal_listener: false,
        held: None,
        tokens: BTreeMap::new(),
        tick: 0,
    };
    let mut observations = Vec::with_capacity(corpus.steps.len());
    let mut size = 0;
    for step in &corpus.steps {
        let result = actual
            .step(step)
            .unwrap_or_else(|e| simple(&format!("error:{}", e.kind())));
        let drained = if actual.terminal_listener {
            Vec::new()
        } else {
            let events = actual.listener.drain().map_err(|e| bad(e.kind()))?;
            actual.terminal_listener = actual.service.statistics().listeners == 0;
            events
        };
        let mut events = drained.into_iter().map(|e|json!({"kind":e.kind,"key":hex(&e.key),"old_value":e.old_value.as_deref().map(hex),"value":e.value.as_deref().map(hex),"watermark":e.watermark})).collect::<Vec<_>>();
        if drop_event && !events.is_empty() {
            events.remove(0);
        }
        let snapshot = actual.service.diagnostic_snapshot(actual.tick).map_err(|e|bad(e.kind()))?.into_iter().map(|e|json!({"key":hex(&e.key),"value":hex(&e.value),"ttl":ttl(true,e.remaining_ttl)})).collect::<Vec<_>>();
        let stats = actual.service.statistics();
        let mut relations = BTreeMap::new();
        for (left, a) in &actual.tokens {
            for (right, b) in &actual.tokens {
                relations.insert(format!("{left}={right}"), a.is_some() && a == b);
            }
        }
        let observation = json!({"result":result,"snapshot":snapshot,"events":events,"token_relations":relations,"owners":{"outcomes":stats.retained_outcomes,"pending":stats.inflight,"listeners":stats.listeners,"snapshot_payload":actual.held.as_ref().is_some_and(|s|!s.entries.is_empty())},"tick":actual.tick});
        let encoded = serde_json::to_vec(&observation).map_err(|e| bad(&e.to_string()))?;
        size += encoded.len() + step.id.len() * 2 + 2;
        if encoded.len() > MAX_LINE || size > MAX_OUTPUT {
            return Err(bad("observation/transcript bound"));
        }
        observations.push(observation);
    }
    Ok(observations)
}

fn ttl(present: bool, remaining: Option<u64>) -> Value {
    if !present {
        json!({"kind":"absent","remaining_ticks":0})
    } else if let Some(ticks) = remaining {
        json!({"kind":"expiring","remaining_ticks":ticks})
    } else {
        json!({"kind":"eternal","remaining_ticks":0})
    }
}
fn simple(kind: &str) -> Value {
    json!({"kind":kind,"value":null,"ttl":ttl(false,None),"items":[],"statuses":[],"number":0})
}
fn numbered(kind: &str, number: u64) -> Value {
    let mut v = simple(kind);
    v["number"] = number.into();
    v
}
fn result(r: ServiceResult) -> Value {
    json!({"kind":r.kind,"value":r.value.as_deref().map(hex),"ttl":ttl(r.version.is_some() && !matches!(r.kind.as_str(),"removed"|"evicted"),r.remaining_ttl),"items":r.items.into_iter().map(|i|json!({"index":i.index,"key":hex(&i.key),"kind":i.kind,"value":i.value.as_deref().map(hex),"ttl":ttl(i.version.is_some() && !matches!(i.kind.as_str(),"removed"|"evicted"),i.remaining_ttl)})).collect::<Vec<_>>(),"statuses":[],"number":r.number})
}

impl Actual {
    // The shared development grammar binds both owners to Java's nonnegative signed clock.
    // Core Rust still accepts its original unsigned clock; no core/protocol limit is changed.
    fn common_expiry(&self, ttl: ServiceTtl) -> Result<(), MapServiceError> {
        if let ServiceTtl::ExpireAfter(ticks) = ttl {
            self.tick
                .checked_add(ticks)
                .filter(|expiry| *expiry <= i64::MAX as u64)
                .ok_or(MapServiceError::InvalidArgument)?;
        }
        Ok(())
    }
    fn dispatch_reserved(&self, command: ServiceCommand) -> Result<ServiceResult, MapServiceError> {
        let identity = command
            .identity
            .as_ref()
            .ok_or(MapServiceError::InvalidArgument)?;
        let pending = matches!(
            self.service.query_operation_outcome(identity)?,
            OperationOutcomeQuery::Pending
        );
        // An expired Pending dispatch reaches Java's shape/TTL validation before digest recovery.
        // Every other reserved path checks unknown/completed/conflicting identities first.
        if pending && command.now >= command.deadline {
            self.common_expiry(command.ttl)?;
        }
        if matches!(
            self.service.recover_command(&command, identity)?,
            ServiceRecovery::Unknown
        ) {
            return Err(MapServiceError::InvalidArgument);
        }
        self.common_expiry(command.ttl)?;
        self.service.execute_reserved(command)
    }
    fn token(&self, e: &ExtendedEntry) -> Result<VersionToken, MapServiceError> {
        self.tokens
            .get(e.label.as_ref().ok_or(MapServiceError::InvalidArgument)?)
            .and_then(|t| *t)
            .ok_or(MapServiceError::InvalidArgument)
    }
    fn command(&self, s: &ExtendedStep) -> Result<ServiceCommand, MapServiceError> {
        let op = match s.operation.as_str() {
            "reserve_put" | "dispatch_put" => "put",
            "clear" => "clear_detailed",
            other => other,
        };
        let mut c = ServiceCommand::point(op, s.entries.first().map_or(&[], |e| e.key.as_slice()));
        c.identity = s.requests.first().cloned();
        c.now = self.tick;
        c.deadline = i64::MAX as u64;
        c.lose_response = s.lose_response;
        if let Some(e) = s.entries.first() {
            c.value = e.value.clone();
            c.ttl = e.ttl;
            if matches!(
                op,
                "replace_if_version" | "remove_if_version" | "set_ttl_if_version"
            ) {
                c.token = Some(self.token(e)?);
            }
        }
        Ok(c)
    }
    fn step(&mut self, s: &ExtendedStep) -> Result<Value, MapServiceError> {
        Ok(match s.operation.as_str() {
            "put" | "replace_if_version" | "remove_if_version" | "set_ttl_if_version" | "evict"
            | "clear" | "destroy" => {
                let command = self.command(s)?;
                self.common_expiry(command.ttl)?;
                result(self.service.execute(command)?)
            }
            "capture_version" => {
                let e = &s.entries[0];
                let mut c = ServiceCommand::point("get_entry_view", &e.key);
                c.now = self.tick;
                let r = self.service.execute(c)?;
                self.tokens.insert(
                    e.label.clone().ok_or(MapServiceError::InvalidArgument)?,
                    r.token,
                );
                result(r)
            }
            "reserve_put" => {
                let c = self.command(s)?;
                self.common_expiry(c.ttl)?;
                self.service.reserve_operation(
                    c.identity.clone().ok_or(MapServiceError::InvalidArgument)?,
                    &c,
                )?;
                simple("pending")
            }
            "dispatch_put" => result(self.dispatch_reserved(self.command(s)?)?),
            "cancel_reserved" => simple(if self.service.cancel_reserved(&s.requests[0])? {
                "cancelled"
            } else {
                "absent"
            }),
            "bulk_replace" | "bulk_remove" | "bulk_set_ttl" => {
                let entries = s
                    .entries
                    .iter()
                    .map(|e| {
                        Ok(VersionBulkEntry {
                            key: e.key.clone(),
                            token: self.token(e)?,
                            value: e.value.clone(),
                            ttl: e.ttl,
                        })
                    })
                    .collect::<Result<Vec<_>, MapServiceError>>()?;
                for entry in &entries {
                    self.common_expiry(entry.ttl)?;
                }
                let action = match s.operation.as_str() {
                    "bulk_replace" => VersionBulkOperation::Replace,
                    "bulk_remove" => VersionBulkOperation::Remove,
                    _ => VersionBulkOperation::SetTtl,
                };
                result(self.service.bulk_version_conditional(
                    action,
                    &entries,
                    s.requests[0].clone(),
                    self.tick,
                    i64::MAX as u64,
                    s.lose_response,
                )?)
            }
            "query_outcomes" => {
                let outcomes = self
                    .service
                    .query_operation_outcomes(&s.requests, 1024 * 1024)?;
                let statuses=s.requests.iter().zip(outcomes).map(|(id,outcome)|{
                    let (kind,result)=match outcome { OperationOutcomeQuery::Pending=>("pending",None),OperationOutcomeQuery::Unknown=>("unknown",None),OperationOutcomeQuery::Completed(r)=>("completed",Some(result(*r))) };
                    json!({"identity":{"client":id.client,"sequence":id.sequence},"kind":kind,"result":result})
                }).collect::<Vec<_>>();
                let mut r = numbered("outcomes", statuses.len() as u64);
                r["statuses"] = statuses.into();
                r
            }
            "retire_outcomes" => {
                let r = &s.requests[0];
                numbered(
                    "retired",
                    self.service
                        .retire_outcomes(&r.client, 1, r.sequence)?
                        .removed as u64,
                )
            }
            "listener_resync" | "hold_listener_snapshot" => {
                if self.held.is_some() {
                    return Err(MapServiceError::ActiveOwners);
                }
                let (items, bytes) = s.limits.ok_or(MapServiceError::InvalidArgument)?;
                let mut lease =
                    self.service
                        .resync_listener(self.listener.id(), self.tick, items, bytes)?;
                let items=lease.entries.iter().enumerate().map(|(i,e)|json!({"index":i,"key":hex(&e.key),"kind":"present","value":hex(&e.value),"ttl":ttl(true,e.remaining_ttl)})).collect::<Vec<_>>();
                let mut r = numbered("snapshot", lease.watermark);
                r["items"] = items.into();
                if s.operation == "hold_listener_snapshot" {
                    self.held = Some(lease);
                } else {
                    lease.close();
                }
                r
            }
            "close_listener_snapshot" => {
                self.held = None;
                simple("closed")
            }
            "checkpoint_restore" => {
                if self.held.is_some() || self.service.statistics().inflight != 0 {
                    return Err(MapServiceError::ActiveOwners);
                }
                self.listener.close();
                let restored = self
                    .service
                    .checkpoint()
                    .and_then(UnifiedIMapService::restore);
                if let Ok(service) = restored.as_ref() {
                    self.service = service.clone();
                }
                self.listener = register(&self.service)?;
                self.terminal_listener = false;
                restored?;
                simple("restored")
            }
            "advance" => {
                let target = self
                    .tick
                    .checked_add(s.ticks)
                    .filter(|t| *t <= i64::MAX as u64)
                    .ok_or(MapServiceError::InvalidArgument)?;
                self.service.expire_step(target, 64)?;
                self.tick = target;
                simple("advanced")
            }
            "signal_gap" => {
                self.service.signal_listener_gap(self.listener.id())?;
                simple("gap")
            }
            "resources" => simple("resources"),
            _ => return Err(MapServiceError::UnsupportedOperation),
        })
    }
}
