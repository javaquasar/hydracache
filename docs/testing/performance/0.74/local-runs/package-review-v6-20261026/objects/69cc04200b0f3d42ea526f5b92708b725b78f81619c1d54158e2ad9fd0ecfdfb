//! Test-process-local immutable historical blobs; never current files or trust.
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

type Key = (String, String);
type Value = Arc<OnceLock<String>>;

#[derive(Default)]
pub struct Cache {
    entries: Mutex<BTreeMap<Key, Value>>,
}
impl Cache {
    pub fn get(&self, revision: &str, path: &str, load: impl FnOnce() -> String) -> String {
        let value = {
            let mut entries = self.entries.lock().expect("historical blob cache poisoned");
            Arc::clone(
                entries
                    .entry((revision.to_owned(), path.to_owned()))
                    .or_default(),
            )
        };
        // Do not hold the index lock while the exact historical loader runs.
        // Failed loaders panic as before and cannot publish partial/empty data.
        value.get_or_init(load).clone()
    }
}
