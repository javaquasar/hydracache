#![allow(dead_code)] // Exact additive views reused by current-source contracts.
#[path = "bootstrap_child_guard.rs"]
mod child_guard;
pub fn registry_view(raw: &str) -> String {
    child_guard::registry_view(raw)
}
pub const BLOCK: &str = r#"
<!-- BEGIN BOOTSTRAP ENTRY V1 -->
## Bootstrap child-entry assertions v1 (not enabled for delivery)

HCBT0741 is exactly 1024 little-endian bytes, size u32 at 8, zero padding at
12..16 and 220..224. Offsets: owner[6] u64 at 16, origins[4] at 64, bounds[4]
at 96; attempt/context SHA256 at 128/160; root device/inode/deadline at 192;
nonzero serial u32 at 216; name digests[2] at 224; document digests[7] at 288
(five original documents then bootstrap policy/receipt); image identities[6]
at 512 (host, launcher, bootstrap, worker, sealed config, legacy256); stream
identities[4] at 656 (stdin, stdout, stderr, child release); guardian pid/dev/ino
triples[2] at 720; image digests[5] at 768 (host, launcher, bootstrap, worker,
config); legacy digest at 928, parent-local census fingerprint at 960 and
canonical newline DiagnosticState digest at 992. Names hash UTF-8 bytes without
newline. All digests are raw SHA256. Identities are dev/ino/size u64 triples,
stream pairs dev/ino. Deadline is the original minimum, never a new window.

Wrong length/magic/size/padding, invalid integer shape, zero serial, mismatched
minimum/root deadline or legacy size refuse. Structural decode is NOT
authentication; shape-valid mutations remain assertions. The parent compares
every byte against its own freshly validated original sources and retains the
same readonly/CLOEXEC/four-sealed inode and offset inside unchanged full frames.
The parent fingerprint binds its descriptor values/roles/image stamps; it is
NOT a child FD remapping table, signature or transferable permit.

This is an additive, currently non-delivered format, not a reinterpretation of
HCHL0741. Old readers must refuse it; no migration/default/fallback. It grants
no child credential, manager/guardian authentication, execveat, original start,
writer cessation, completion or server admission. No child-reader/binary or
transport is enabled by this schema.
<!-- END BOOTSTRAP ENTRY V1 -->
"#;
fn remove(raw: &str, addition: &str) -> String {
    assert_eq!(raw.matches(addition).count(), 1);
    raw.replacen(addition, "", 1)
}
pub fn compat_view(raw: &str) -> String {
    remove(&child_guard::compat_view(raw), BLOCK)
}
pub fn source_view(name: &str, raw: &str) -> String {
    let raw = child_guard::source_view(name, raw);
    let raw = raw.as_str();
    match name {
        "diagnostic_bootstrap_build.rs" => remove(raw,"\n#[path = \"diagnostic_bootstrap_entry_format.rs\"]\npub mod entry_format;\n"),
        "diagnostic_bootstrap_image_census.rs" => remove(raw,"\n#[path = \"diagnostic_bootstrap_entry_census.rs\"]\npub(crate) mod entry;\n"),
        "diagnostic_held_unit_material_kernel.rs" => remove(raw,"\n#[path = \"diagnostic_bootstrap_entry_resource.rs\"]\npub(super) mod bootstrap_entry;\n"),
        "diagnostic_bootstrap_owner.rs" => {
            let mut result=raw.to_string();
            for addition in [
                "\n#[path = \"diagnostic_bootstrap_entry_binding.rs\"]\nmod entry_binding;\nuse crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::entry::EntryBindings;\nuse kernel::bootstrap_entry::EntryResource;\n",
                "    entry: EntryBindings,\n", "    entry: Option<EntryResource>,\n",
                "            entry,\n", "            entry: None,\n",
                "        let legacy = self\n            .owner\n            .resources\n            .as_ref()\n            .ok_or(BootstrapError::Binding)?\n            .entry_contribution(&bindings(&context))\n            .map_err(ArtifactError::from)?;\n        let expected = entry_binding::expected(\n            &context,\n            docs,\n            [metadata.dev(), metadata.ino(), metadata.len()],\n            legacy,\n            &census,\n        )?;\n        if self.entry.is_none() {\n            if self.retained.is_some() {\n                return Err(BootstrapError::Binding);\n            }\n            self.entry = Some(EntryResource::create(&expected).map_err(ArtifactError::from)?);\n        }\n        let entry = self\n            .entry\n            .as_ref()\n            .ok_or(BootstrapError::Binding)?\n            .inspect(&expected, &census)\n            .map_err(ArtifactError::from)?;\n",
            ] {result=remove(&result,addition);}
            result
        }
        _=>raw.into(),
    }
}
