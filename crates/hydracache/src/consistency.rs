use std::time::{Duration, Instant};

use serde::{de::DeserializeOwned, Serialize};
use tokio::time::sleep;

use hydracache_core::{CacheOptions, Result};

use crate::{CacheError, HydraCache};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ConsistencyMode {
    #[default]
    Eventual,
    LocalReadYourWrites,
    ClusterReadYourWrites {
        timeout: Duration,
    },
    Quorum {
        timeout: Duration,
    },
    Leader,
    FailClosed,
    DegradedOk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyToken {
    generation: u64,
    namespace: String,
    origin_node: String,
}

impl ConsistencyToken {
    pub fn new(
        generation: u64,
        namespace: impl Into<String>,
        origin_node: impl Into<String>,
    ) -> Self {
        Self {
            generation,
            namespace: namespace.into(),
            origin_node: origin_node.into(),
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn origin_node(&self) -> &str {
        &self.origin_node
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DegradeReason {
    Timeout,
    UnsupportedMode(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsistencyOutcome<T> {
    Fresh(T),
    Degraded { value: T, reason: DegradeReason },
    TimedOut,
    FailedClosed { reason: DegradeReason },
}

#[derive(Debug, Clone)]
pub struct ConsistencyInvalidate<C = hydracache_core::PostcardCodec>
where
    C: hydracache_core::CacheCodec,
{
    cache: HydraCache<C>,
    tag: String,
    namespace: String,
}

impl<C> ConsistencyInvalidate<C>
where
    C: hydracache_core::CacheCodec,
{
    pub(crate) fn new(cache: HydraCache<C>, tag: impl Into<String>) -> Self {
        Self {
            cache,
            tag: tag.into(),
            namespace: "local".to_owned(),
        }
    }

    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub async fn consistency(self, _mode: ConsistencyMode) -> Result<ConsistencyToken> {
        self.cache.invalidate_tag(&self.tag).await?;
        let generation = self
            .cache
            .inner
            .consistency_generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            .saturating_add(1);
        Ok(ConsistencyToken::new(
            generation,
            self.namespace,
            self.cache.inner.invalidation_node_id.clone(),
        ))
    }
}

impl<C> HydraCache<C>
where
    C: hydracache_core::CacheCodec,
{
    pub fn invalidate_after_write(&self, tag: impl Into<String>) -> ConsistencyInvalidate<C> {
        ConsistencyInvalidate::new(self.clone(), tag)
    }

    pub fn consistency_generation(&self) -> u64 {
        self.inner
            .consistency_generation
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub async fn get_with_consistency<T, E, F, Fut>(
        &self,
        key: &str,
        token: &ConsistencyToken,
        mode: ConsistencyMode,
        options: CacheOptions,
        loader: F,
    ) -> Result<ConsistencyOutcome<T>>
    where
        T: Serialize + DeserializeOwned,
        E: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        match mode {
            ConsistencyMode::Eventual | ConsistencyMode::LocalReadYourWrites => {
                if self.wait_local(token, Duration::ZERO).await {
                    self.inner
                        .stats
                        .consistency_wait_successes
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Ok(ConsistencyOutcome::Fresh(
                        self.get_or_load(key, options, loader).await?,
                    ))
                } else {
                    self.inner
                        .stats
                        .consistency_fail_closed
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Ok(ConsistencyOutcome::FailedClosed {
                        reason: DegradeReason::Timeout,
                    })
                }
            }
            ConsistencyMode::ClusterReadYourWrites { timeout } => {
                self.get_after_wait_or_timeout(key, token, timeout, options, loader)
                    .await
            }
            ConsistencyMode::DegradedOk => {
                if self.wait_local(token, Duration::ZERO).await {
                    self.inner
                        .stats
                        .consistency_wait_successes
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Ok(ConsistencyOutcome::Fresh(
                        self.get_or_load(key, options, loader).await?,
                    ))
                } else if let Some(value) = self.get(key).await? {
                    self.inner
                        .stats
                        .consistency_degraded_reads
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Ok(ConsistencyOutcome::Degraded {
                        value,
                        reason: DegradeReason::Timeout,
                    })
                } else {
                    self.inner
                        .stats
                        .consistency_wait_timeouts
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Ok(ConsistencyOutcome::TimedOut)
                }
            }
            ConsistencyMode::FailClosed => {
                if self.wait_local(token, Duration::ZERO).await {
                    self.inner
                        .stats
                        .consistency_wait_successes
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Ok(ConsistencyOutcome::Fresh(
                        self.get_or_load(key, options, loader).await?,
                    ))
                } else {
                    self.inner
                        .stats
                        .consistency_fail_closed
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Ok(ConsistencyOutcome::FailedClosed {
                        reason: DegradeReason::Timeout,
                    })
                }
            }
            ConsistencyMode::Quorum { .. } => {
                self.inner
                    .stats
                    .consistency_fail_closed
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(ConsistencyOutcome::FailedClosed {
                    reason: DegradeReason::UnsupportedMode("quorum"),
                })
            }
            ConsistencyMode::Leader => {
                self.inner
                    .stats
                    .consistency_fail_closed
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(ConsistencyOutcome::FailedClosed {
                    reason: DegradeReason::UnsupportedMode("leader"),
                })
            }
        }
    }

    async fn get_after_wait_or_timeout<T, E, F, Fut>(
        &self,
        key: &str,
        token: &ConsistencyToken,
        timeout: Duration,
        options: CacheOptions,
        loader: F,
    ) -> Result<ConsistencyOutcome<T>>
    where
        T: Serialize + DeserializeOwned,
        E: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        if self.wait_local(token, timeout).await {
            self.inner
                .stats
                .consistency_wait_successes
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(ConsistencyOutcome::Fresh(
                self.get_or_load(key, options, loader).await?,
            ))
        } else {
            self.inner
                .stats
                .consistency_wait_timeouts
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(ConsistencyOutcome::TimedOut)
        }
    }

    async fn wait_local(&self, token: &ConsistencyToken, timeout: Duration) -> bool {
        if self.consistency_generation() >= token.generation {
            return true;
        }
        if timeout.is_zero() {
            return false;
        }

        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            sleep(Duration::from_millis(2)).await;
            if self.consistency_generation() >= token.generation {
                return true;
            }
        }
        false
    }

    pub fn unsupported_consistency_mode_error(mode: &'static str) -> CacheError {
        CacheError::Backend(format!(
            "consistency mode {mode} is not implemented by this local runtime"
        ))
    }
}
