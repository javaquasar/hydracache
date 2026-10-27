# Exact held source security view

This ordinary test helper recognizes exactly the bare name
`diagnostic_manager_channel.rs` and its full repository-relative source path.
It passes those freshly read bytes through the unchanged security projector's
single fixed channel row. Other names, including other security-input paths,
retain their bytes. This is not filesystem authorization or live execution.

The current producer is `2a4a771d48ed6ad54288fd15ea7c75b56f4bd80d`; the whole
security predecessor is `fa9e708868b22999d60c9ccd97b694662910a095`. Whole current
and predecessor hashes and all three exact inverse edits remain enforced by the
original security descriptor. No current observation is cached. The prior
sandbox/delivery hook removals still follow this projection, through unchanged
held/bootstrap guards. The intermediate security predecessor is not itself the
oldest held phase.

Four source readers add this pure step after their original read/normalization
and before the original provider/entry guards. All original case names,
assertions, calls and conditional census reads stay exact. No new filesystem,
Git, clock, transport or pin lookup is introduced into those guards.

The helper uses the caller's `current_security` module. Activation reuses its
already imported bootstrap successor; other targets import the fixed security
module once. The bootstrap facade changes only `mod successor` visibility so
activation can share it. Its functions and read order are byte-identical. This
avoids duplicate module instances without a Clippy exception or product API change.

The focused gate is:

```text
cargo +1.94.0 test --offline --locked --no-fail-fast -p xtask --test held_activation_ingress_074 --test held_bind_alias_refusal_074 --test held_complete_role_census_074 --test held_material_roles_074 --test held_security_view_074 --test bootstrap_current_view_074 --test bootstrap_doc_reader_074 -- --test-threads=4
```

Its 33 cases pass on Windows 1.94, Windows MSRV 1.88 and GNU/Linux 1.94. The
unchanged diagnostic-artifact target separately passes 20 cases per environment;
it needs no new repair here. First baseline, oracle and duplicate-module refusals
are retained in the [report and evidence](../../docs/testing/performance/0.74/held-current-view-result-20261027.md).
These are compatibility/refusal checks, not performance measurements or a complete
package verdict. Product, qualification inputs, live predicates and 300/60/10
origins/windows are unchanged; no server, service, clock or workload is used.
