//! Approved current text before the immutable twelve-path phase; no new IO.
use super::contract;
#[path = "../authority-reader-phase-074/current_main.rs"]
mod terminal_current;

pub const MANIFEST: &str = "crates/hydracache-redis-compat/Cargo.toml";
pub const MAIN: &str = terminal_current::SOURCE;
pub const PRODUCER: &str = "63cb4d7b9bca1c901b2f7420f86effd3309a6882";
const CURRENT: &str = "b28b823581bf37c9b828a8b166098e0a13add0a487e6d139a61a381dba0da652";
const PREVIOUS: &str = "86548e7516a7a73b05099f4c7ff7342949d5656f748b914c298dc02997297b3b";
const EDGES: &str = "webpki-root-certs = \"=1.0.10\"\nwebpki-roots = \"=1.0.10\"\nwebpki-roots-legacy = { package = \"webpki-roots\", version = \"=0.26.11\" }\n";

pub fn project(path: &str, raw: String) -> String {
    if path == MAIN {
        return terminal_current::project(raw);
    }
    if path != MANIFEST {
        return raw;
    }
    let raw = raw.replace("\r\n", "\n");
    assert_eq!(
        contract::digest(raw.as_bytes()),
        CURRENT,
        "approved manifest at {PRODUCER}"
    );
    assert_eq!(raw.matches(EDGES).count(), 1, "exact three dev-only edges");
    let previous = raw.replacen(EDGES, "", 1);
    assert_eq!(
        contract::digest(previous.as_bytes()),
        PREVIOUS,
        "complete pre-roots manifest"
    );
    previous
}

pub fn view(path: &str, raw: String) -> String {
    assert!(contract::known(path), "only the declared frontier");
    if path == MAIN {
        let previous = contract::project(path, project(path, raw.clone()));
        assert_eq!(previous, crate::runtime::expected(path).unwrap());
        assert!(crate::runtime::accepts(path, &previous));
        // The immutable old phase checks its whole expected body above. The
        // unchanged current gate must separately receive the original current
        // observation, never that already projected historical representation.
        crate::runtime::view(path, raw)
    } else {
        crate::runtime::view(path, contract::project(path, project(path, raw)))
    }
}
