#![allow(dead_code)] // Exact additive views, never a replacement validation body.
pub fn hook(name: &str) -> Option<&'static str> {
    match name {
        "diagnostic_bootstrap_installed_entry_owner.rs"=>Some("\n#[path = \"diagnostic_bootstrap_sandbox_owner.rs\"]\nmod sandbox_owner;\n"),
        "diagnostic_manager_channel.rs"=>Some("\n#[path = \"diagnostic_manager_sandbox_query.rs\"]\nmod sandbox_query;\n"),
        "diagnostic_process.rs"=>Some("\n#[cfg(target_arch = \"x86_64\")]\n#[path = \"diagnostic_sandbox_process.rs\"]\npub(crate) mod sandbox;\n"),
        "diagnostic_bootstrap_installed_entry_census.rs"=>Some("\nimpl InstalledBindings {\n    pub(crate) fn sandbox_image_identity(&self) -> [u64; 3] {\n        [\n            self.installed.stat[0],\n            self.installed.stat[1],\n            self.installed.stat[6],\n        ]\n    }\n}\n"),
        _=>None,
    }
}
#[path = "bootstrap_held_delivery_guard.rs"]
pub mod held;
pub fn source_view(name: &str, input: &str) -> String {
    let newer = held::source_view(name, input);
    let raw = newer.as_str();
    if let Some(hook) = hook(name) {
        assert_eq!(raw.matches(hook).count(), 1);
        raw.replacen(hook, "", 1)
    } else {
        raw.into()
    }
}
