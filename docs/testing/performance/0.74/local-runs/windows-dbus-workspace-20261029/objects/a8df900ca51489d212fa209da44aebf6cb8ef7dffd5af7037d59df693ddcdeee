//! Independent public embedded API, not ClientSurfaceState or a wire adapter.
use crate::{
    native::{Dataset, Operation},
    target::{PreloadOutcome, Target, TargetError, TargetOutcome, TargetRequest},
};
use async_trait::async_trait;
use bytes::Bytes;
use hydracache::{CacheOptions, HydraCache};
use std::fmt::Write;
use tokio::sync::Mutex;

type Result<T> = std::result::Result<T, String>;

pub struct EmbeddedControl {
    cache: HydraCache,
    entries: Vec<(String, Bytes)>,
    digest: String,
    operation: Operation,
    clients: Vec<Mutex<()>>,
}
impl EmbeddedControl {
    pub async fn start(slots: usize, dataset: Dataset, operation: Operation) -> Result<Self> {
        if ![1, 8, 32, 128].contains(&slots) {
            return Err("unsupported embedded client slots".to_owned());
        }
        if !matches!(operation, Operation::Get | Operation::Put) {
            return Err("embedded control does not claim native batch atomicity".to_owned());
        }
        let entries = dataset
            .entries()
            .into_iter()
            .map(|(key, value)| {
                let mut mapped = String::from("hc074-embedded:");
                for byte in key {
                    write!(&mut mapped, "{byte:02x}").expect("String writer");
                }
                (mapped, value)
            })
            .collect();
        let control = Self {
            cache: HydraCache::local().max_capacity(128 * 1024 * 1024).build(),
            entries,
            digest: dataset.digest(),
            operation,
            clients: (0..slots).map(|_| Mutex::new(())).collect(),
        };
        control.refill_dataset().await?;
        Ok(control)
    }
    pub fn dataset_digest(&self) -> &str {
        &self.digest
    }
    pub fn client_slots(&self) -> usize {
        self.clients.len()
    }
    pub async fn verify(&self) -> Result<String> {
        for (key, expected) in &self.entries {
            if self
                .cache
                .get_encoded(key)
                .await
                .map_err(|e| e.to_string())?
                .as_ref()
                != Some(expected)
            {
                return Err("embedded value drift".to_owned());
            }
        }
        if self
            .cache
            .get_encoded("hc074-embedded:missing")
            .await
            .map_err(|e| e.to_string())?
            .is_some()
        {
            return Err("embedded missing key became a hit".to_owned());
        }
        // Public approximate diagnostic count after its maintenance barrier is
        // a semantic sanity check, never an allocator-retention measurement.
        if self.cache.diagnostics().await.estimated_entries != self.entries.len() as u64 {
            return Err("embedded dataset entry-count drift".to_owned());
        }
        Ok(self.digest.clone())
    }
    pub async fn refill_dataset(&self) -> Result<()> {
        for (key, value) in &self.entries {
            self.cache
                .put_encoded(key, value.clone(), CacheOptions::new())
                .await
                .map_err(|e| e.to_string())?;
        }
        self.verify().await.map(|_| ())
    }
    pub async fn delete_dataset(&self) -> Result<()> {
        for (key, _) in &self.entries {
            if !self.cache.remove(key).await.map_err(|e| e.to_string())? {
                return Err("embedded DELETE was not applied".to_owned());
            }
        }
        for (key, _) in &self.entries {
            if self
                .cache
                .get_encoded(key)
                .await
                .map_err(|e| e.to_string())?
                .is_some()
            {
                return Err("embedded deleted value remained visible".to_owned());
            }
        }
        if self.cache.diagnostics().await.estimated_entries != 0 {
            return Err("embedded deleted entries remained".to_owned());
        }
        Ok(())
    }
    pub async fn shutdown(self) -> Result<()> {
        self.cache.flush().await.map_err(|e| e.to_string())?;
        if self.cache.diagnostics().await.estimated_entries != 0 {
            return Err("embedded shutdown entries remained".to_owned());
        }
        // No tool listener/actor was ever created; dropping the public cache
        // owner is not proof about allocator/OS retained memory.
        Ok(())
    }
}
#[async_trait]
impl Target for EmbeddedControl {
    async fn reset(&self) -> std::result::Result<String, TargetError> {
        Err(TargetError::Reset(
            "single-fixture embedded owner; construct a fresh control".to_owned(),
        ))
    }
    async fn preload(&self) -> std::result::Result<PreloadOutcome, TargetError> {
        self.refill_dataset().await.map_err(TargetError::Preload)?;
        Ok(PreloadOutcome {
            operations: self.entries.len() as u64,
            state_digest: self.digest.clone(),
        })
    }
    async fn state_digest(&self) -> std::result::Result<String, TargetError> {
        self.verify().await.map_err(TargetError::Measurement)
    }
    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        let _client = self.clients[request.sequence as usize % self.clients.len()]
            .lock()
            .await;
        let (key, expected) = &self.entries[request.sequence as usize % self.entries.len()];
        match self.operation {
            Operation::Get => match self.cache.get_encoded(key).await {
                Ok(Some(value)) if value == *expected => TargetOutcome::Success,
                _ => TargetOutcome::Error,
            },
            Operation::Put => match self
                .cache
                .put_encoded(key, expected.clone(), CacheOptions::new())
                .await
            {
                Ok(()) => TargetOutcome::Success,
                Err(_) => TargetOutcome::Error,
            },
            _ => unreachable!("validated embedded operation"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn changed_or_missing_value_is_not_success_or_a_retried_read() {
        let control = EmbeddedControl::start(1, Dataset::new(1, 256).unwrap(), Operation::Get)
            .await
            .unwrap();
        let key = &control.entries[0].0;
        control
            .cache
            .put_encoded(key, Bytes::from_static(b"wrong"), CacheOptions::new())
            .await
            .unwrap();
        assert_eq!(
            control.execute(TargetRequest { sequence: 0 }).await,
            TargetOutcome::Error
        );
        assert!(control.verify().await.is_err());
        assert!(control.reset().await.is_err());
        control.delete_dataset().await.unwrap();
        assert_eq!(
            control.execute(TargetRequest { sequence: 1 }).await,
            TargetOutcome::Error
        );
        control.refill_dataset().await.unwrap();
        assert_eq!(
            control.execute(TargetRequest { sequence: 2 }).await,
            TargetOutcome::Success
        );
        control.shutdown().await.unwrap();
    }
}
