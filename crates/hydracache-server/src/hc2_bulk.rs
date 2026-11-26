//! Complete ingress validation and bounded owner-group execution of HC/2 batches.

use super::*;
use crate::imap_value_decision::WriteOp;
use crate::imap_value_journal::hash;
use crate::imap_value_plane::MAX_VALUE_BYTES;
use hydracache_client_hc2::wire::{BatchRequest, RetryDirective};

pub(super) const MAX_ACTIVE_BATCHES: usize = 8;
const MAX_BATCH_ITEMS: usize = 256;
const MAX_OWNER_GROUPS: usize = 4;
const MAX_BATCH_REPLY_BYTES: usize = 4 * 1024 * 1024;
const MAX_BATCH_RUN_MS: u64 = 10_000;

pub(super) async fn dispatch(
    service: &Hc2ClientPlaneService,
    member: &DistributedMember,
    correlation: u64,
    meta: RequestMeta,
    identity: ClientIdentity,
    batch: BatchRequest,
) -> InvocationResponse {
    if let Err(error) = member
        .plane
        .authorize(identity.client_id(), identity.tenant())
    {
        return wire_error(
            map_client_error(error.code),
            "HC/2 batch scope is not admitted",
        );
    }
    let Ok(_permit) = member.bulk.try_acquire() else {
        return wire_error(
            StableErrorCode::StableErrorQuotaExceeded,
            "HC/2 batch concurrency bound exceeded",
        );
    };
    let requests = match normalize(batch, service.state.limits()) {
        Ok(requests) if meta.idempotency_key.len() <= 128 => requests,
        Ok(_) => {
            return wire_error(
                StableErrorCode::StableErrorInvalidRequest,
                "batch identity exceeds its bound",
            )
        }
        Err((code, detail)) => return wire_error(code, detail),
    };
    let owners = match member
        .plane
        .bulk_placements(identity.tenant(), requests.iter())
    {
        Ok(owners) => owners,
        Err(error) => {
            return wire_error(
                map_client_error(error.code),
                "HC/2 batch placement is unavailable",
            )
        }
    };
    let mut results: Vec<Option<InvocationResponse>> = vec![None; requests.len()];
    let mut groups = BTreeMap::<_, Vec<_>>::new();
    for (index, (request, owner)) in requests.into_iter().zip(owners).enumerate() {
        groups.entry(owner).or_default().push((index, request));
    }
    let latest = unix_time_ms().saturating_add(MAX_BATCH_RUN_MS);
    let deadline = if meta.deadline_unix_ms == 0 {
        latest
    } else {
        latest.min(meta.deadline_unix_ms)
    };
    let token = scoped_identity(&meta.idempotency_key);
    let issued_epoch = token.as_deref().and_then(|token| token.split(':').nth(1));
    let parent = meta.idempotency_key.as_ref();
    let identity = &identity;
    let mut executions = futures_util::stream::iter(groups.into_values())
        .map(|group| async move {
            let mut items = Vec::with_capacity(group.len());
            let read_only = group
                .iter()
                .all(|(_, request)| matches!(request, ClientRequest::Get { .. }));
            let mut reads = BTreeMap::<Vec<String>, InvocationResponse>::new();
            for (index, request) in group {
                let response = if unix_time_ms() >= deadline {
                    let mut refused = wire_error(
                        StableErrorCode::StableErrorDeadlineExceeded,
                        "batch item was not dispatched",
                    );
                    if let Some(meta) = &mut refused.meta {
                        meta.retry = RetryDirective::SameConnection as i32;
                        meta.topology_epoch = service.topology_epoch();
                    }
                    refused
                } else {
                    match &request {
                        ClientRequest::Get { key, .. } => {
                            let cache_key = key.segments().to_vec();
                            if let Some(repeated) = reads.get(&cache_key) {
                                items.push((index, repeated.clone()));
                                continue;
                            }
                            let mut response = execute_normalized(
                                service,
                                correlation,
                                deadline,
                                identity,
                                request.clone(),
                                None,
                                true,
                            )
                            .await;
                            if unix_time_ms() < deadline
                                && response.meta.as_ref().is_some_and(|meta| {
                                    matches!(
                                        StableErrorCode::try_from(meta.error),
                                        Ok(StableErrorCode::StableErrorUnavailable
                                            | StableErrorCode::StableErrorDeadlineExceeded)
                                    )
                                })
                            {
                                response = execute_normalized(
                                    service,
                                    correlation,
                                    deadline,
                                    identity,
                                    request,
                                    None,
                                    true,
                                )
                                .await;
                            }
                            if read_only
                                && matches!(
                                    response.result,
                                    Some(invocation_response::Result::Value(_))
                                )
                            {
                                reads.insert(cache_key, response.clone());
                            }
                            response
                        }
                        _ => {
                            let key = (!parent.is_empty())
                                .then(|| item_identity(parent, issued_epoch, index));
                            execute_normalized(
                                service,
                                correlation,
                                deadline,
                                identity,
                                request,
                                key,
                                true,
                            )
                            .await
                        }
                    }
                };
                items.push((index, response));
            }
            items
        })
        .buffer_unordered(MAX_OWNER_GROUPS);
    while let Some(group) = executions.next().await {
        for (index, response) in group {
            results[index] = Some(response);
        }
    }
    let mut remaining = MAX_BATCH_REPLY_BYTES;
    let items = results
        .into_iter()
        .map(|result| {
            let item = result.expect("one result for each admitted batch operation");
            let len = item.encoded_len() + 10;
            if matches!(item.result, Some(invocation_response::Result::Value(_))) && len > remaining
            {
                let mut omitted = wire_error(
                    StableErrorCode::StableErrorQuotaExceeded,
                    "batch value reply exceeds its bound",
                );
                if let Some(meta) = &mut omitted.meta {
                    meta.topology_epoch = service.topology_epoch();
                }
                omitted
            } else {
                remaining = remaining.saturating_sub(len);
                item
            }
        })
        .collect();
    wire_success(Some(invocation_response::Result::Batch(BatchResult {
        items,
    })))
}

fn normalize(
    batch: BatchRequest,
    limits: hydracache_client_transport_axum::ClientSurfaceLimits,
) -> Result<Vec<ClientRequest>, (StableErrorCode, &'static str)> {
    use StableErrorCode::*;
    if batch.items.is_empty()
        || batch.items.len() > limits.max_batch_entries.min(MAX_BATCH_ITEMS)
        || batch.encoded_len() > limits.max_batch_bytes
    {
        return Err((
            StableErrorQuotaExceeded,
            "batch exceeds its entry or byte bound",
        ));
    }
    batch
        .items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            if item.item_id as usize != index + 1 {
                return Err((
                    StableErrorInvalidRequest,
                    "batch item IDs must match input order",
                ));
            }
            let operation = batch_operation(
                item.operation
                    .ok_or((StableErrorInvalidRequest, "batch operation is required"))?,
            );
            if !admitted_distributed_operation(&operation) {
                return Err((
                    StableErrorUnsupported,
                    "batch item is outside the distributed profile",
                ));
            }
            let request = to_client_request(operation)
                .map_err(|detail| (StableErrorInvalidRequest, detail))?;
            let key = match &request {
                ClientRequest::Get { key, .. } => key,
                _ => {
                    let (_, key, write) = WriteOp::request(&request)
                        .ok_or((StableErrorUnsupported, "batch operation is unsupported"))?;
                    if !write.fits(limits.max_value_bytes.min(MAX_VALUE_BYTES)) {
                        return Err((
                            StableErrorQuotaExceeded,
                            "batch item exceeds the value bound",
                        ));
                    }
                    key
                }
            };
            if key.segments()[0].len() > 60 * 1024 {
                return Err((
                    StableErrorQuotaExceeded,
                    "batch item key exceeds the key bound",
                ));
            }
            Ok(request)
        })
        .collect()
}

fn item_identity(parent: &[u8], issued_epoch: Option<&str>, index: usize) -> String {
    let identity = hex(&hash(("hc2-imap-batch-v1", parent, index as u64)));
    match issued_epoch {
        Some(epoch) => format!("w10e:{epoch}:batch:{identity}"),
        None => format!("batch:{identity}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydracache_client_hc2::wire::{batch_item, BatchItem, GetRequest};

    #[test]
    fn whole_batch_pre_admission_rejects_duplicates_and_empty_operations() {
        let get = |item_id| BatchItem {
            item_id,
            operation: Some(batch_item::Operation::Get(GetRequest {
                key: Bytes::from_static(b"a"),
            })),
        };
        let admit = |items| normalize(BatchRequest { items }, Default::default());
        assert!(admit(vec![get(1), get(2)]).is_ok());
        assert!(admit(vec![get(1), get(1)]).is_err());
        assert!(admit(vec![
            get(1),
            BatchItem {
                item_id: 2,
                operation: None
            }
        ])
        .is_err());
        assert!(admit(Vec::new()).is_err());
    }

    #[test]
    fn replay_ids_are_stable_ordered_and_cannot_alias_public_tokens() {
        let id = item_identity(b"w10e:4:nonce", Some("4"), 0);
        assert_eq!(id, item_identity(b"w10e:4:nonce", Some("4"), 0));
        assert_ne!(id, item_identity(b"w10e:4:nonce", Some("4"), 1));
        assert_ne!(id, item_identity(b"w10e:4:other", Some("4"), 0));
        assert!(id.starts_with("w10e:4:batch:"));
        assert!(scoped_identity(id.as_bytes()).is_none());
        assert!(item_identity(b"opaque", None, 0).starts_with("batch:"));
    }
}
