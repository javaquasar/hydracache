//! Guarded fixed-query Prometheus history adapter for Management Center 2.0.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::config::ManagementHistoryConfig;

/// Management history route.
pub const MANAGEMENT_HISTORY_PATH: &str = "/management/v1/history";
pub const MANAGEMENT_HISTORY_MAX_RANGE_MS: u64 = 24 * 60 * 60 * 1_000;
pub const MANAGEMENT_HISTORY_MIN_STEP_MS: u64 = 10_000;
pub const MANAGEMENT_HISTORY_MAX_POINTS: usize = 1_000;
pub const MANAGEMENT_HISTORY_MAX_SERIES: usize = 24;
pub const MANAGEMENT_HISTORY_MAX_RESPONSE_BYTES: usize = 256 * 1_024;
pub const MANAGEMENT_HISTORY_MAX_RESOLVED_ADDRESSES: usize = 16;
pub const MANAGEMENT_HISTORY_MAX_TOKEN_BYTES: u64 = 4_096;
pub const MANAGEMENT_HISTORY_MAX_CONCURRENCY: usize = 2;
pub const MANAGEMENT_HISTORY_DEADLINE: Duration = Duration::from_secs(2);

/// Only these reviewed queries can cross the adapter boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagementHistoryQueryId {
    ReplicationSuccess,
    ReplicationFailure,
    CacheEntries,
    AdmissionQueueDepth,
}

impl ManagementHistoryQueryId {
    fn promql(self) -> &'static str {
        match self {
            Self::ReplicationSuccess => "sum(hydracache_replication_success_total)",
            Self::ReplicationFailure => "sum(hydracache_replication_failure_total)",
            Self::CacheEntries => "sum(hydracache_cache_estimated_entries)",
            Self::AdmissionQueueDepth => "max(hydracache_admission_queue_depth)",
        }
    }
}

/// Browser-controlled range values. No URL, PromQL, header, or credential field exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagementHistoryRequest {
    pub query_id: ManagementHistoryQueryId,
    pub start_ms: u64,
    pub end_ms: u64,
    pub step_ms: u64,
}

impl ManagementHistoryRequest {
    pub fn validate(&self) -> Result<(), ManagementHistoryError> {
        if self.start_ms >= self.end_ms {
            return Err(ManagementHistoryError::InvalidRange);
        }
        let range = self.end_ms.saturating_sub(self.start_ms);
        if range > MANAGEMENT_HISTORY_MAX_RANGE_MS
            || self.step_ms < MANAGEMENT_HISTORY_MIN_STEP_MS
            || self.step_ms > range
        {
            return Err(ManagementHistoryError::InvalidRange);
        }
        let points = range / self.step_ms + 1;
        if points > MANAGEMENT_HISTORY_MAX_POINTS as u64 {
            return Err(ManagementHistoryError::InvalidRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagementHistoryState {
    Available,
    NoAdapter,
    NoData,
    Partial,
    Timeout,
    UpstreamError,
    Malformed,
    Oversize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ManagementHistoryPoint {
    pub timestamp_unix_ms: u64,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ManagementHistorySeries {
    pub series_index: u16,
    pub points: Vec<ManagementHistoryPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ManagementHistoryData {
    pub query_id: ManagementHistoryQueryId,
    pub state: ManagementHistoryState,
    pub source: &'static str,
    pub series: Vec<ManagementHistorySeries>,
    pub truncated: bool,
}

impl ManagementHistoryData {
    pub fn unavailable(query_id: ManagementHistoryQueryId, state: ManagementHistoryState) -> Self {
        Self {
            query_id,
            state,
            source: "prometheus_fixed_query",
            series: Vec::new(),
            truncated: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagementHistoryError {
    InvalidRange,
    UnsafeDestination,
    CredentialUnavailable,
    Timeout,
    Upstream,
    Malformed,
    Oversize,
}

/// Cloneable bounded adapter. Configuration is fixed at server construction.
#[derive(Debug, Clone)]
pub struct ManagementHistoryService {
    config: ManagementHistoryConfig,
    origin: reqwest::Url,
    concurrency: Arc<Semaphore>,
}

impl ManagementHistoryService {
    pub fn from_config(
        config: &ManagementHistoryConfig,
    ) -> Result<Option<Self>, ManagementHistoryError> {
        if !config.enabled {
            return Ok(None);
        }
        let origin = reqwest::Url::parse(
            config
                .origin
                .as_deref()
                .ok_or(ManagementHistoryError::UnsafeDestination)?,
        )
        .map_err(|_| ManagementHistoryError::UnsafeDestination)?;
        Ok(Some(Self {
            config: config.clone(),
            origin,
            concurrency: Arc::new(Semaphore::new(MANAGEMENT_HISTORY_MAX_CONCURRENCY)),
        }))
    }

    pub async fn query(
        &self,
        request: ManagementHistoryRequest,
    ) -> Result<ManagementHistoryData, ManagementHistoryError> {
        request.validate()?;
        tokio::time::timeout(MANAGEMENT_HISTORY_DEADLINE, self.query_bounded(request))
            .await
            .map_err(|_| ManagementHistoryError::Timeout)?
    }

    async fn query_bounded(
        &self,
        request: ManagementHistoryRequest,
    ) -> Result<ManagementHistoryData, ManagementHistoryError> {
        let _permit = self
            .concurrency
            .acquire()
            .await
            .map_err(|_| ManagementHistoryError::Upstream)?;
        let host = self
            .origin
            .host_str()
            .ok_or(ManagementHistoryError::UnsafeDestination)?;
        let port = self
            .origin
            .port_or_known_default()
            .ok_or(ManagementHistoryError::UnsafeDestination)?;
        let addresses = tokio::net::lookup_host((host, port))
            .await
            .map_err(|_| ManagementHistoryError::UnsafeDestination)?
            .take(MANAGEMENT_HISTORY_MAX_RESOLVED_ADDRESSES + 1)
            .collect::<Vec<_>>();
        validate_resolved_addresses(&self.config, &addresses)?;
        let pinned = addresses[0];
        let mut client_builder =
            reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());
        if host.parse::<IpAddr>().is_err() {
            client_builder = client_builder.resolve(host, pinned);
        }
        let client = client_builder
            .build()
            .map_err(|_| ManagementHistoryError::Upstream)?;
        let endpoint = self
            .origin
            .join("/api/v1/query_range")
            .map_err(|_| ManagementHistoryError::UnsafeDestination)?;
        let mut builder = client.get(endpoint).query(&[
            ("query", request.query_id.promql().to_owned()),
            ("start", format_seconds(request.start_ms)),
            ("end", format_seconds(request.end_ms)),
            ("step", format_seconds(request.step_ms)),
        ]);
        if let Some(path) = self.config.bearer_token_file.as_deref() {
            builder = builder.bearer_auth(read_bounded_token(path)?);
        }
        let response = builder
            .send()
            .await
            .map_err(|_| ManagementHistoryError::Upstream)?;
        if response.status().is_redirection() || !response.status().is_success() {
            return Err(ManagementHistoryError::Upstream);
        }
        if response
            .content_length()
            .is_some_and(|length| length > MANAGEMENT_HISTORY_MAX_RESPONSE_BYTES as u64)
        {
            return Err(ManagementHistoryError::Oversize);
        }
        let mut stream = response.bytes_stream();
        let mut body = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| ManagementHistoryError::Upstream)?;
            if body.len().saturating_add(chunk.len()) > MANAGEMENT_HISTORY_MAX_RESPONSE_BYTES {
                return Err(ManagementHistoryError::Oversize);
            }
            body.extend_from_slice(&chunk);
        }
        parse_prometheus_response(request.query_id, &body)
    }
}

/// Reject rebinding or unsafe address sets before an HTTP client is constructed.
pub fn validate_resolved_addresses(
    config: &ManagementHistoryConfig,
    addresses: &[SocketAddr],
) -> Result<(), ManagementHistoryError> {
    if addresses.is_empty() || addresses.len() > MANAGEMENT_HISTORY_MAX_RESOLVED_ADDRESSES {
        return Err(ManagementHistoryError::UnsafeDestination);
    }
    if !config.allow_private_networks
        && addresses
            .iter()
            .any(|address| !is_public_destination(address.ip()))
    {
        return Err(ManagementHistoryError::UnsafeDestination);
    }
    Ok(())
}

fn is_public_destination(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_multicast()
                && !ip.is_unspecified()
                && ip != Ipv4Addr::BROADCAST
                && !is_carrier_grade_nat(ip)
                && !is_benchmark(ip)
        }
        IpAddr::V6(ip) => {
            !ip.is_loopback()
                && !ip.is_unspecified()
                && !ip.is_unique_local()
                && !ip.is_unicast_link_local()
                && !ip.is_multicast()
        }
    }
}

fn is_carrier_grade_nat(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

fn is_benchmark(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 198 && matches!(octets[1], 18 | 19)
}

fn format_seconds(milliseconds: u64) -> String {
    format!("{}.{:03}", milliseconds / 1_000, milliseconds % 1_000)
}

fn read_bounded_token(path: &std::path::Path) -> Result<String, ManagementHistoryError> {
    let metadata =
        std::fs::metadata(path).map_err(|_| ManagementHistoryError::CredentialUnavailable)?;
    if !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MANAGEMENT_HISTORY_MAX_TOKEN_BYTES
    {
        return Err(ManagementHistoryError::CredentialUnavailable);
    }
    let token =
        std::fs::read_to_string(path).map_err(|_| ManagementHistoryError::CredentialUnavailable)?;
    let token = token.trim();
    if token.is_empty() || token.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(ManagementHistoryError::CredentialUnavailable);
    }
    Ok(token.to_owned())
}

/// Parse only Prometheus matrix/vector success payloads and discard labels.
pub fn parse_prometheus_response(
    query_id: ManagementHistoryQueryId,
    body: &[u8],
) -> Result<ManagementHistoryData, ManagementHistoryError> {
    if body.len() > MANAGEMENT_HISTORY_MAX_RESPONSE_BYTES {
        return Err(ManagementHistoryError::Oversize);
    }
    let root: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| ManagementHistoryError::Malformed)?;
    if root.get("status").and_then(serde_json::Value::as_str) != Some("success") {
        return Err(ManagementHistoryError::Upstream);
    }
    let data = root.get("data").ok_or(ManagementHistoryError::Malformed)?;
    let result_type = data
        .get("resultType")
        .and_then(serde_json::Value::as_str)
        .ok_or(ManagementHistoryError::Malformed)?;
    let results = data
        .get("result")
        .and_then(serde_json::Value::as_array)
        .ok_or(ManagementHistoryError::Malformed)?;
    let mut truncated = results.len() > MANAGEMENT_HISTORY_MAX_SERIES;
    let mut series = Vec::new();
    for result in results.iter().take(MANAGEMENT_HISTORY_MAX_SERIES) {
        let raw_points = match result_type {
            "matrix" => result
                .get("values")
                .and_then(serde_json::Value::as_array)
                .map(Vec::as_slice),
            "vector" => result.get("value").map(std::slice::from_ref),
            _ => return Err(ManagementHistoryError::Malformed),
        }
        .ok_or(ManagementHistoryError::Malformed)?;
        truncated |= raw_points.len() > MANAGEMENT_HISTORY_MAX_POINTS;
        let mut points = raw_points
            .iter()
            .take(MANAGEMENT_HISTORY_MAX_POINTS)
            .map(parse_point)
            .collect::<Result<Vec<_>, _>>()?;
        points.sort_by_key(|point| point.timestamp_unix_ms);
        points.dedup_by_key(|point| point.timestamp_unix_ms);
        series.push(ManagementHistorySeries {
            series_index: 0,
            points,
        });
    }
    series.sort_by(|left, right| {
        let left_key = left
            .points
            .first()
            .map(|point| (point.timestamp_unix_ms, point.value.to_bits()));
        let right_key = right
            .points
            .first()
            .map(|point| (point.timestamp_unix_ms, point.value.to_bits()));
        left_key.cmp(&right_key)
    });
    for (index, item) in series.iter_mut().enumerate() {
        item.series_index = index as u16;
    }
    let state = if truncated {
        ManagementHistoryState::Partial
    } else if series.iter().all(|item| item.points.is_empty()) {
        ManagementHistoryState::NoData
    } else {
        ManagementHistoryState::Available
    };
    Ok(ManagementHistoryData {
        query_id,
        state,
        source: "prometheus_fixed_query",
        series,
        truncated,
    })
}

fn parse_point(
    value: &serde_json::Value,
) -> Result<ManagementHistoryPoint, ManagementHistoryError> {
    let pair = value
        .as_array()
        .filter(|pair| pair.len() == 2)
        .ok_or(ManagementHistoryError::Malformed)?;
    let timestamp = pair[0]
        .as_f64()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or(ManagementHistoryError::Malformed)?;
    let sample = pair[1]
        .as_str()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .ok_or(ManagementHistoryError::Malformed)?;
    Ok(ManagementHistoryPoint {
        timestamp_unix_ms: (timestamp * 1_000.0).round().clamp(0.0, u64::MAX as f64) as u64,
        value: sample,
    })
}
