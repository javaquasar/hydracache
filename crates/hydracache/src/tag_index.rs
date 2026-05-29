use std::collections::{HashMap, HashSet};

use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadGenerationSnapshot {
    pub(crate) global: u64,
    pub(crate) tags: Vec<(String, u64)>,
}

#[derive(Debug, Default)]
pub(crate) struct TagIndex {
    state: RwLock<TagIndexState>,
}

#[derive(Debug, Default)]
struct TagIndexState {
    keys_by_tag: HashMap<String, HashSet<String>>,
    generations: HashMap<String, u64>,
    global_generation: u64,
}

impl TagIndex {
    pub(crate) async fn register(&self, key: &str, tags: &[String]) {
        if tags.is_empty() {
            return;
        }

        let mut guard = self.state.write().await;
        for tag in tags {
            guard
                .keys_by_tag
                .entry(tag.clone())
                .or_default()
                .insert(key.to_owned());
        }
    }

    pub(crate) async fn unregister(&self, key: &str, tags: &[String]) {
        if tags.is_empty() {
            return;
        }

        let mut guard = self.state.write().await;
        for tag in tags {
            if let Some(keys) = guard.keys_by_tag.get_mut(tag) {
                keys.remove(key);
                if keys.is_empty() {
                    guard.keys_by_tag.remove(tag);
                }
            }
        }
    }

    pub(crate) async fn take_tag(&self, tag: &str) -> Vec<String> {
        let mut guard = self.state.write().await;
        let generation = guard.generations.entry(tag.to_owned()).or_default();
        *generation = generation.wrapping_add(1);

        guard
            .keys_by_tag
            .remove(tag)
            .map(|keys| keys.into_iter().collect())
            .unwrap_or_default()
    }

    pub(crate) async fn snapshot(&self, tags: &[String]) -> LoadGenerationSnapshot {
        let guard = self.state.read().await;
        LoadGenerationSnapshot {
            global: guard.global_generation,
            tags: tags
                .iter()
                .map(|tag| {
                    (
                        tag.clone(),
                        guard.generations.get(tag).copied().unwrap_or(0),
                    )
                })
                .collect(),
        }
    }

    pub(crate) async fn is_current(&self, snapshot: &LoadGenerationSnapshot) -> bool {
        let guard = self.state.read().await;
        guard.global_generation == snapshot.global
            && snapshot.tags.iter().all(|(tag, generation)| {
                guard.generations.get(tag).copied().unwrap_or(0) == *generation
            })
    }

    pub(crate) async fn clear(&self) {
        let mut guard = self.state.write().await;
        guard.keys_by_tag.clear();
        guard.global_generation = guard.global_generation.wrapping_add(1);
    }
}
