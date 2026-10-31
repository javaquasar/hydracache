//! Bounded indexed views from the existing owner, with incarnation-scoped tokens per item.
use super::lifecycle::{budget, require_namespace};
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionedEntryView {
    pub index: usize,
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
    pub token: Option<VersionToken>,
    pub remaining_ttl: Option<u64>,
}

pub(super) fn input_bound(
    state: &ServiceState,
    keys: &[Vec<u8>],
    maximum_bytes: usize,
) -> Result<usize, MapServiceError> {
    require_namespace(state)?;
    if state.map.is_destroyed() {
        return Err(CollectionError::MapDestroyed.into());
    }
    if keys.is_empty()
        || keys.len() > state.limits.max_batch
        || maximum_bytes == 0
        || maximum_bytes
            > state
                .limits
                .query_bytes
                .min(state.map.service_bounds().max_response_bytes)
    {
        return Err(MapServiceError::InvalidArgument);
    }
    let mut request = 0usize;
    for key in keys {
        check_bound(
            "key_bytes",
            state.map.service_bounds().max_key_bytes,
            key.len(),
        )?;
        request = request
            .checked_add(16)
            .and_then(|n| n.checked_add(key.len()))
            .ok_or(MapServiceError::Capacity)?;
    }
    check_bound("request_bytes", 8 * 1_024 * 1_024, request)?;
    Ok(request)
}

pub(super) fn response_bound(
    state: &ServiceState,
    keys: &[Vec<u8>],
    now: u64,
    maximum_bytes: usize,
) -> Result<usize, MapServiceError> {
    input_bound(state, keys, maximum_bytes)?;
    if now < state.now {
        return Err(MapServiceError::ClockRegressed);
    }
    let mut bytes = 0usize;
    for key in keys {
        let value = state.map.service_live_value_len(key, now);
        bytes = bytes
            .checked_add(64)
            .and_then(|n| n.checked_add(key.len()))
            .and_then(|n| n.checked_add(value.map_or(0, |value| value.saturating_add(24))))
            .ok_or(MapServiceError::Capacity)?;
        budget("entry_views_response", maximum_bytes, bytes)?;
    }
    Ok(bytes)
}

impl UnifiedIMapService {
    /// Validate the entire result before cloning values or performing expiry maintenance.
    pub fn entry_views_response_bytes(
        &self,
        keys: &[Vec<u8>],
        now: u64,
        maximum_bytes: usize,
    ) -> Result<usize, MapServiceError> {
        let state = self.state.lock().expect("local IMap state poisoned");
        response_bound(&state, keys, now, maximum_bytes)
    }

    pub fn entry_views(
        &self,
        keys: &[Vec<u8>],
        now: u64,
        maximum_bytes: usize,
    ) -> Result<Vec<VersionedEntryView>, MapServiceError> {
        let mut state = self.state.lock().expect("local IMap state poisoned");
        response_bound(&state, keys, now, maximum_bytes)?;
        advance(&mut state, now)?;
        let views = keys
            .iter()
            .enumerate()
            .map(|(index, key)| {
                let entry = state.map.service_entry(key, now);
                let (value, token, remaining_ttl) = if let Some(entry) = entry {
                    (
                        Some(entry.value),
                        Some(VersionToken {
                            namespace_generation: state.namespace_generation,
                            map_id: state.map.service_map_id(),
                            version: entry.version,
                        }),
                        entry.remaining_ttl,
                    )
                } else {
                    (None, None, None)
                };
                VersionedEntryView {
                    index,
                    key: key.clone(),
                    value,
                    token,
                    remaining_ttl,
                }
            })
            .collect();
        let wakes = std::mem::take(&mut state.pending_wakes);
        drop(state);
        for wake in wakes {
            wake.wake();
        }
        Ok(views)
    }
}
