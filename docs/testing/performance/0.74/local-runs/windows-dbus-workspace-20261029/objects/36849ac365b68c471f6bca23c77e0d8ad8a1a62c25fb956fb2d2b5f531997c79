#requires -Version 7.2
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskOutput=Join-Path $taskRepo 'target/reserved-reconciliation-checks-20261022'
if(Test-Path -LiteralPath $taskOutput){throw 'Never replace reconciliation check evidence'}
New-Item -ItemType Directory -Path $taskOutput -ErrorAction Stop|Out-Null
$taskSteps=@()
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskOutput "$Name.log"
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash -LiteralPath $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Offline reconciliation check refused: $Name ($taskCode)"}
}
Record 'reconciliation-controls' { & python -B tools/offline-reserved-reconciliation-074/test_reconcile.py }
Record 'round-controls' { & python -B tools/local-profile-session-074/test_attribute_material_rounds.py }
Record 'reader-controls' { & python -B tools/local-profile-session-074/test_material_live.py }
Record 'arithmetic-controls' { & python -B tools/local-profile-session-074/test_attribute_partial.py }
Record 'derived-facts' { & python -B tools/offline-reserved-reconciliation-074/reconcile.py --output docs/testing/performance/0.74/local-runs/reserved-reconciliation-20261022/facts.json --verify }
Record 'source-canaries' { & cargo test --offline --locked -p xtask --test collector_peer_capacity_074 --test collector_peer_baseline_074 --test trust_component_stack_074 --test trust_component_baseline_074 --test context_document_stack_074 --test context_document_baseline_074 --test projection_groups_074 --test pidfd_info_path_074 --test held_material_live_074 --test held_material_074 --test held_unit_projection_074 --test boot_operations_074 --test held_preparation_cost_074 }
Record 'check' { & cargo check --offline --locked -p xtask --all-targets }
Record 'clippy' { & cargo clippy --offline --locked -p xtask --all-targets --all-features -- -D warnings }
Record 'docs' { & cargo xtask doc-check }
Record 'registry' { & cargo xtask gated-test-check }
Record 'performance' { & cargo xtask performance-contract-check --release 0.74 }
Record 'protected-source' { & git diff --exit-code 1e5f31d9d443bf6c9b1d901781986501425bf9e9 -- tools/long-run-supervisor-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml }
@{steps=$taskSteps;new_measurements=0;new_profile_invocations=0;runtime_changed=$false;observer_changed=$false;server=$false;product=$false}|ConvertTo-Json -Depth 8|Out-File (Join-Path $taskOutput 'result.json') -Encoding utf8 -NoClobber
