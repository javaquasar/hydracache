#![allow(dead_code)]
pub fn hook(name: &str) -> Option<&'static str> {
    match name {
        "diagnostic_bootstrap_installed_entry_census.rs" => {
            Some("\n#[path = \"diagnostic_held_delivery_wire.rs\"]\npub(crate) mod held_wire;\n")
        }
        "diagnostic_manager_channel.rs" => {
            Some("\n#[path = \"diagnostic_manager_held_delivery.rs\"]\nmod held_delivery;\n")
        }
        "diagnostic_bootstrap_sandbox_owner.rs" => {
            Some("\n#[path = \"diagnostic_original_held_delivery.rs\"]\nmod held_delivery;\n")
        }
        _ => None,
    }
}
pub fn source_view(name: &str, raw: &str) -> String {
    if let Some(hook) = hook(name) {
        assert_eq!(raw.matches(hook).count(), 1);
        raw.replacen(hook, "", 1)
    } else {
        raw.into()
    }
}
