use super::*;
pub(super) fn scratch(registry: &FixtureAuthorityRegistry) -> (usize, usize) {
    let mut slots = 0; let mut bytes = 0;
    for entry in &registry.entries { for leaf in [&entry.record, &entry.marker] {
        if let Some(raw) = &leaf.scratch { slots += 1; bytes += raw.capacity(); }
    }}
    (slots, bytes)
}
