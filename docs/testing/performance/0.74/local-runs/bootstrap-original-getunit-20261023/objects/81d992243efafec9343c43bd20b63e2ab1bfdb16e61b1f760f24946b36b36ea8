use crate::entry_guard;
#[allow(dead_code)] // Different current-source targets use different exact views.
pub fn provider_view(name: &str, raw: &str) -> String {
    let raw = entry_guard::source_view(name, raw);
    let hook=match name {
        "diagnostic_held_role_census.rs"=>"\n#[path = \"diagnostic_bootstrap_image_census.rs\"]\npub(crate) mod bootstrap_images;\n",
        "diagnostic_held_role_files.rs"=>"\n#[path = \"diagnostic_bootstrap_image_provider.rs\"]\nmod bootstrap_images;\n",
        "diagnostic_held_unit_material_kernel.rs"=>"\n#[path = \"diagnostic_bootstrap_original_inventory.rs\"]\npub(super) mod bootstrap_inputs;\n",
        _=>return raw,
    };
    assert_eq!(raw.matches(hook).count(), 1);
    raw.replacen(hook, "", 1)
}
#[allow(dead_code)]
pub fn owner_view(raw: &str) -> String {
    let raw = entry_guard::source_view("diagnostic_bootstrap_owner.rs", raw);
    let additions=[
        "use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::BootstrapImageBindings;\n",
        "    census: BootstrapImageBindings,\n",
        "        let original = self\n            .owner\n            .inventory\n            .as_ref()\n            .ok_or(BootstrapError::Binding)?;\n        let contribution = kernel::bootstrap_inputs::capture(&original.descriptors)\n            .map_err(ArtifactError::from)?;\n        let metadata = self.image.metadata().map_err(ArtifactError::from)?;\n        let census = self.owner.owner.owner.roles.bootstrap_image_census(\n            &original.retained_roles,\n            contribution,\n            &self.image,\n            [\n                context.host.identity,\n                context.launcher.identity,\n                [metadata.dev(), metadata.ino(), metadata.len()],\n            ],\n        )?;\n",
    ];
    let mut result = raw.to_string();
    for addition in additions {
        assert_eq!(result.matches(addition).count(), 1);
        result = result.replacen(addition, "", 1);
    }
    let new="        Ok(Observation {\n            context,\n            image,\n            census,\n        })";
    assert_eq!(result.matches(new).count(), 1);
    result.replacen(new, "        Ok(Observation { context, image })", 1)
}
