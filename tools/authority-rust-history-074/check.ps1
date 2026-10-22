#requires -Version 7.2
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskOutput=Join-Path $taskRepo 'target/authority-rust-history-first-checks'
if(Test-Path -LiteralPath $taskOutput){throw 'Never replace first historical Rust checks'}
New-Item -ItemType Directory -Path $taskOutput -ErrorAction Stop|Out-Null
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskOutput "$Name.log"
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash -LiteralPath $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Historical Rust check refused: $Name ($taskCode)"}
}
try{
    Record 'format' { & rustfmt --check --edition 2021 tools/authority-operator-order-074/probe.rs tools/authority-operator-order-074/argv.rs crates/xtask/tests/authority_operator_order_074.rs }
    Record 'source-format' { & rustfmt --check --edition 2021 --config skip_children=true tools/authority-storage-source-074/candidate.rs tools/authority-storage-source-074/observer.rs tools/authority-storage-source-074/contract.rs crates/xtask/tests/authority_storage_source_074.rs tools/authority-rust-history-074/contract.rs crates/xtask/tests/support/authority_history_io.rs crates/xtask/tests/authority_rust_history_074.rs }
    Record 'source-controls' { & cargo test --offline --locked -p xtask --test authority_rust_history_074 --test authority_storage_source_074 }
    Record 'workspace-format' { & cargo fmt -p xtask --check }
    Record 'history-controls' { & python -B tools/authority-history-view-074/test_view.py }
    Record 'history-receipt-controls' { & python -B tools/authority-history-view-074/test_review_first.py }
    Record 'current-baseline' { & python -B -c "import sys; sys.path.insert(0,'tools/authority-history-view-074'); from pathlib import Path; import view_inputs; print(view_inputs.current_fixture_identity(Path.cwd()))" }
    Record 'derived-facts' { & python -B tools/authority-history-view-074/review_first.py --output docs/testing/performance/0.74/local-runs/authority-history-view-20261022/facts.json --verify }
    Record 'docs' { & cargo xtask doc-check }
    Record 'registry' { & cargo xtask gated-test-check }
    Record 'performance' { & cargo xtask performance-contract-check --release 0.74 }
    Record 'canaries' { & cargo test --offline --locked -p xtask --test authority_operator_order_074 --test authority_owned_journal_074 --test authority_scratch_reuse_074 --test authority_read_baseline_074 --test sample_boot_inline_074 --test sample_boot_baseline_074 --test proc_document_baseline_074 --test collector_peer_capacity_074 --test collector_peer_baseline_074 --test trust_component_stack_074 --test trust_component_baseline_074 --test context_document_stack_074 --test context_document_baseline_074 --test projection_groups_074 --test pidfd_info_path_074 --test held_material_live_074 --test held_material_074 --test held_unit_projection_074 --test boot_operations_074 --test held_preparation_cost_074 --test authority_cpu_074 --test authority_breakdown_074 --test authority_inventory_074 --test authority_fd_path_074 --test authority_expected_name_074 --test authority_leaf_cpu_074 --test authority_owners_074 --test authority_leaf_name_074 --test authority_open_name_074 --test authority_name_capacity_074 }
    Record 'check' { & cargo check --offline --locked -p xtask --all-targets }
    Record 'clippy' { & cargo clippy --offline --locked -p xtask --all-targets --all-features -- -D warnings }
    Record 'protected-source' { & git diff --exit-code 33c54b0c -- tools/long-run-supervisor-074 Cargo.toml Cargo.lock docs/COMPAT.md docs/plans/releases.toml docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml }
}catch{$taskFailure=$_.Exception.Message}
@{source=(git rev-parse HEAD);steps=$taskSteps;failure=$taskFailure;historical_io_migrated=$true;raw_old_python_controls_rerun=$false;current_runtime_admitted=$false;runtime_candidate_implemented=$false;original_owner_profiles=0;server=$false;product=$false}|ConvertTo-Json -Depth 8|Out-File (Join-Path $taskOutput 'result.json') -Encoding utf8 -NoClobber
if($taskFailure){throw $taskFailure}
