use std::collections::{HashMap, HashSet};

use tokio::sync::RwLock;

const MAX_GENERATION_TOMBSTONES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadGenerationSnapshot {
    pub(crate) global: u64,
    pub(crate) key: String,
    pub(crate) key_generation: u64,
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
    key_generations: HashMap<String, u64>,
    global_generation: u64,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TagIndexRetainedState {
    pub(crate) tags: usize,
    pub(crate) memberships: usize,
    pub(crate) tag_generations: usize,
    pub(crate) key_generations: usize,
    pub(crate) string_capacity_bytes: usize,
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
        if !guard.generations.contains_key(tag)
            && guard.generations.len() >= MAX_GENERATION_TOMBSTONES
        {
            rotate_generation_epoch(&mut guard);
        }
        let generation = guard.generations.entry(tag.to_owned()).or_default();
        *generation = generation.wrapping_add(1);

        guard
            .keys_by_tag
            .remove(tag)
            .map(|keys| keys.into_iter().collect())
            .unwrap_or_default()
    }

    pub(crate) async fn snapshot(&self, key: &str, tags: &[String]) -> LoadGenerationSnapshot {
        let guard = self.state.read().await;
        LoadGenerationSnapshot {
            global: guard.global_generation,
            key: key.to_owned(),
            key_generation: guard.key_generations.get(key).copied().unwrap_or(0),
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
            && guard
                .key_generations
                .get(&snapshot.key)
                .copied()
                .unwrap_or(0)
                == snapshot.key_generation
            && snapshot.tags.iter().all(|(tag, generation)| {
                guard.generations.get(tag).copied().unwrap_or(0) == *generation
            })
    }

    pub(crate) async fn advance_key(&self, key: &str) {
        let mut guard = self.state.write().await;
        if !guard.key_generations.contains_key(key)
            && guard.key_generations.len() >= MAX_GENERATION_TOMBSTONES
        {
            rotate_generation_epoch(&mut guard);
        }
        let generation = guard.key_generations.entry(key.to_owned()).or_default();
        *generation = generation.wrapping_add(1);
    }

    pub(crate) async fn clear(&self) {
        let mut guard = self.state.write().await;
        guard.keys_by_tag.clear();
        guard.generations.clear();
        guard.key_generations.clear();
        guard.global_generation = guard.global_generation.wrapping_add(1);
    }

    #[cfg(test)]
    pub(crate) async fn retained_state(&self) -> TagIndexRetainedState {
        let guard = self.state.read().await;
        TagIndexRetainedState {
            tags: guard.keys_by_tag.len(),
            memberships: guard.keys_by_tag.values().map(HashSet::len).sum(),
            tag_generations: guard.generations.len(),
            key_generations: guard.key_generations.len(),
            string_capacity_bytes: guard
                .keys_by_tag
                .iter()
                .map(|(tag, keys)| {
                    tag.capacity() + keys.iter().map(String::capacity).sum::<usize>()
                })
                .sum::<usize>()
                + guard
                    .generations
                    .keys()
                    .map(String::capacity)
                    .sum::<usize>()
                + guard
                    .key_generations
                    .keys()
                    .map(String::capacity)
                    .sum::<usize>(),
        }
    }
}

fn rotate_generation_epoch(state: &mut TagIndexState) {
    state.generations.clear();
    state.key_generations.clear();
    state.global_generation = state.global_generation.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unregister_releases_empty_tag_memberships() {
        let index = TagIndex::default();
        let tags = vec!["alpha".to_owned(), "beta".to_owned()];

        index.register("key", &tags).await;
        let retained = index.retained_state().await;
        assert_eq!(retained.tags, 2);
        assert_eq!(retained.memberships, 2);
        assert_eq!(retained.tag_generations, 0);
        assert_eq!(retained.key_generations, 0);
        assert!(retained.string_capacity_bytes >= 15);

        index.unregister("key", &tags).await;
        assert_eq!(index.retained_state().await.tags, 0);
        assert_eq!(index.retained_state().await.memberships, 0);
    }

    #[tokio::test]
    async fn invalidation_generations_are_observable_until_flush() {
        let index = TagIndex::default();

        index.take_tag("tag-a").await;
        index.advance_key("key-a").await;

        let retained = index.retained_state().await;
        assert_eq!(retained.tag_generations, 1);
        assert_eq!(retained.key_generations, 1);
        assert!(retained.string_capacity_bytes >= "tag-a".len() + "key-a".len());
    }

    #[tokio::test]
    async fn clear_releases_all_index_maps_and_fences_old_loads() {
        let index = TagIndex::default();
        let tags = vec!["tag-a".to_owned()];
        index.register("key-a", &tags).await;
        let before = index.snapshot("key-a", &tags).await;
        index.take_tag("orphan-tag").await;
        index.advance_key("orphan-key").await;

        index.clear().await;

        assert_eq!(
            index.retained_state().await,
            TagIndexRetainedState {
                tags: 0,
                memberships: 0,
                tag_generations: 0,
                key_generations: 0,
                string_capacity_bytes: 0,
            }
        );
        assert!(!index.is_current(&before).await);
    }

    #[tokio::test]
    async fn unique_key_and_tag_tombstones_rotate_into_a_bounded_epoch() {
        let index = TagIndex::default();
        let old_key_snapshot = index.snapshot("old-key", &[]).await;
        let old_tag_snapshot = index.snapshot("old-tag-key", &["old-tag".to_owned()]).await;

        for value in 0..10_000 {
            index.advance_key(&format!("key-{value}")).await;
            index.take_tag(&format!("tag-{value}")).await;
        }

        let retained = index.retained_state().await;
        assert!(retained.key_generations <= MAX_GENERATION_TOMBSTONES);
        assert!(retained.tag_generations <= MAX_GENERATION_TOMBSTONES);
        assert!(!index.is_current(&old_key_snapshot).await);
        assert!(!index.is_current(&old_tag_snapshot).await);
    }
}
