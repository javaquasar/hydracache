//! IO-only adapter allowlist; no authority/runtime normalization.
pub const SOURCE: &str = "33c54b0cda8c330b115009cdf092ad002077cedf";
pub const PREFIX: &str = "#[path = \"support/authority_history_io.rs\"]\nmod history;\n";
pub const GUARDS: [(&str, usize, usize); 30] = [
    ("authority_operator_order_074", 1, 2),
    ("authority_owned_journal_074", 1, 2),
    ("authority_scratch_reuse_074", 1, 2),
    ("authority_read_baseline_074", 1, 2),
    ("sample_boot_inline_074", 1, 3),
    ("sample_boot_baseline_074", 1, 2),
    ("proc_document_baseline_074", 1, 2),
    ("collector_peer_capacity_074", 1, 1),
    ("collector_peer_baseline_074", 1, 1),
    ("trust_component_stack_074", 1, 1),
    ("trust_component_baseline_074", 1, 1),
    ("context_document_stack_074", 1, 1),
    ("context_document_baseline_074", 1, 1),
    ("projection_groups_074", 1, 1),
    ("pidfd_info_path_074", 2, 2),
    ("held_material_live_074", 1, 1),
    ("held_material_074", 1, 1),
    ("held_unit_projection_074", 1, 1),
    ("boot_operations_074", 1, 1),
    ("held_preparation_cost_074", 1, 1),
    ("authority_cpu_074", 1, 1),
    ("authority_breakdown_074", 1, 1),
    ("authority_inventory_074", 1, 1),
    ("authority_fd_path_074", 1, 1),
    ("authority_expected_name_074", 1, 1),
    ("authority_leaf_cpu_074", 2, 2),
    ("authority_owners_074", 1, 1),
    ("authority_leaf_name_074", 1, 1),
    ("authority_open_name_074", 1, 1),
    ("authority_name_capacity_074", 1, 1),
];
pub fn migrated(original: &str, reads: usize, commands: usize) -> String {
    assert_eq!(original.matches("std::fs::read_to_string").count(), reads);
    assert_eq!(
        original
            .matches("std::process::Command::new(\"git\")")
            .count(),
        commands
    );
    assert!(!original.contains("mod history;"));
    let mut output = original
        .replace("std::fs::read_to_string", "history::read_to_string")
        .replace("std::process::Command::new(\"git\")", "history::command()");
    // Keep inner documentation first and the existing module declaration order.
    let at = if original.starts_with("//!") {
        original
            .lines()
            .take_while(|line| line.starts_with("//!"))
            .map(|line| line.len() + 1)
            .sum()
    } else if original.starts_with("#[path =") {
        original.find("mod guard;\n").unwrap() + "mod guard;\n".len()
    } else {
        0
    };
    output.insert_str(at, PREFIX);
    output
}
pub fn accepts(original: &str, current: &str, reads: usize, commands: usize) -> bool {
    let expected = migrated(original, reads, commands);
    syn::parse_file(current).is_ok()
        && expected
            .parse::<proc_macro2::TokenStream>()
            .unwrap()
            .to_string()
            == current
                .parse::<proc_macro2::TokenStream>()
                .map(|value| value.to_string())
                .unwrap_or_default()
}
