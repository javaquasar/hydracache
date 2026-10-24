//! Exact additive census view, never a historical current-source replacement.
pub fn material_view(source: &str) -> String {
    let mut source = source.to_owned();
    for hook in [
        "\n#[path = \"diagnostic_material_role_census.rs\"]\nmod role_census;\n",
        "    retained_roles: role_census::CompleteBindings,\n",
        "        let retained_roles = role_census::capture(self, &context)?;\n",
        "        let retained_roles = kernel::complete_roles(&roles, &descriptors, retained_roles)?;\n",
        "            retained_roles,\n",
    ] {
        assert_eq!(source.matches(hook).count(), 1, "exact census addition: {hook}");
        source = source.replacen(hook, "", 1);
    }
    source
}
