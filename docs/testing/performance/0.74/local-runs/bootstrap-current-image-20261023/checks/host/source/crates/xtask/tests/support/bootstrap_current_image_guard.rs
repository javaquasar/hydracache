#![allow(dead_code)] // Exact additive views shared by preceding source guards.
pub const BLOCK: &str = r#"
<!-- BEGIN BOOTSTRAP CURRENT IMAGE V1 -->
## Bootstrap current executable consistency v1

--systemd-bootstrap-image-v1 is a separate opt-in refusal-only bootstrap mode.
Older binaries reject it. Entry-only/census-only modes, HCBT0741/HCHL0741 bytes,
activation assertions and role names remain unchanged. The new mode repeats
the complete census, with two current executable observations around its old
revalidation. Fixed procfs thread-self/exe lookup must identify the same sealed
readonly regular inode as bootstrap FD13 and its asserted device/inode/size.
Metadata, seals, flags and zero offsets are checked without reading executable
content again. Owned inspection FDs close before the next closed-set census.

This sequential image consistency does not enroll procfs namespaces, credentials,
signed build/policy or original manager/root/guardians. A matching forged context
still exits 2 with authentication closed; mismatches also exit 2. No send/start,
launcher handoff, role repair, default activation, installation, migration,
writer cessation or server admission is enabled. Old absolute windows remain.
<!-- END BOOTSTRAP CURRENT IMAGE V1 -->
"#;
pub const GATE: &str = r#"
# BEGIN BOOTSTRAP CURRENT IMAGE GATE
[[gate]]
id = "cfg.hydracache-long-run-supervisor-074.bootstrap-current-image"
kind = "cfg_test_target"
source = "tools/long-run-supervisor-074/tests/bootstrap_current_image.rs"
package = "hydracache-long-run-supervisor-074"
target = "bootstrap_current_image"
test = ""
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "local synthetic sealed bootstrap executable consistency/refusal; no original authentication or admission"
tier = "fast"
required_features = []
required_env = []
required_tools = ["cargo","/bin/sh","/bin/sleep"]
timeout_seconds = 180
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []
[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"
[gate.command]
program = "cargo"
args = ["test","--locked","-p","hydracache-long-run-supervisor-074","--test","bootstrap_current_image"]
cwd = "."
platform = "linux"
[gate.command.env]
# END BOOTSTRAP CURRENT IMAGE GATE
"#;
fn remove(raw: &str, addition: &str) -> String {
    assert_eq!(raw.matches(addition).count(), 1);
    raw.replacen(addition, "", 1)
}
pub fn compat_view(raw: &str) -> String {
    remove(raw, BLOCK)
}
pub fn registry_view(raw: &str) -> String {
    remove(raw, GATE)
}
pub fn source_view(name: &str, raw: &str) -> String {
    match name {
        "bin/diagnostic_held_bootstrap.rs" => {
            let raw = remove(raw, "\n#[path = \"../diagnostic_bootstrap_image_entry.rs\"]\nmod image_entry;\n");
            remove(&raw, "    if image_entry::requested() {\n        if let Err(error) = image_entry::run() {\n            eprintln!(\"Bootstrap refused: {error}; no start, send or executable handoff.\");\n        }\n        std::process::exit(2);\n    }\n")
        }
        "diagnostic_bootstrap_child_reader.rs" => remove(raw,
            "\n#[path = \"diagnostic_bootstrap_current_image.rs\"]\npub(super) mod current_image;\n"),
        _ => raw.into(),
    }
}
