#requires -Version 7.2
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskOutput=Join-Path $taskRepo 'target/authority-read-audit-checks-first'
if(Test-Path -LiteralPath $taskOutput){throw 'Never replace first authority audit checks'}
New-Item -ItemType Directory -Path $taskOutput -ErrorAction Stop|Out-Null
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskOutput "$Name.log"
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash -LiteralPath $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Authority audit check refused: $Name ($taskCode)"}
}
try{
    Record 'audit-controls' { & python -B tools/offline-authority-read-audit-074/test_audit_read.py }
    Record 'prior-inline-controls' { & python -B tools/sample-boot-inline-074/test_review_inline.py }
    Record 'prior-boot-controls' { & python -B tools/sample-boot-baseline-074/test_review_boot.py }
    Record 'prior-proc-controls' { & python -B tools/proc-document-baseline-074/test_review.py }
    Record 'prior-reconciliation-controls' { & python -B tools/offline-reserved-reconciliation-074/test_reconcile.py }
    Record 'canaries' { & cargo test --offline --locked -p xtask --test sample_boot_inline_074 --test sample_boot_baseline_074 --test proc_document_baseline_074 --test collector_peer_capacity_074 --test collector_peer_baseline_074 --test trust_component_stack_074 --test trust_component_baseline_074 --test context_document_stack_074 --test context_document_baseline_074 --test projection_groups_074 --test pidfd_info_path_074 --test held_material_live_074 --test held_material_074 --test held_unit_projection_074 --test boot_operations_074 --test held_preparation_cost_074 }
    Record 'format' { & cargo fmt -p xtask --check }
    Record 'check' { & cargo check --offline --locked -p xtask --all-targets }
    Record 'clippy' { & cargo clippy --offline --locked -p xtask --all-targets --all-features -- -D warnings }
    Record 'docs' { & cargo xtask doc-check }
    Record 'registry' { & cargo xtask gated-test-check }
    Record 'performance' { & cargo xtask performance-contract-check --release 0.74 }
    Record 'protected-source' { & git diff --exit-code 3f48a7c9 -- tools/long-run-supervisor-074 crates Cargo.toml Cargo.lock docs/COMPAT.md docs/plans/releases.toml docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml }
}catch{$taskFailure=$_.Exception.Message}
@{steps=$taskSteps;failure=$taskFailure;new_measurements=0;runtime_candidate_implemented=$false;original_owner_profiles=0;server=$false;product=$false}|ConvertTo-Json -Depth 8|Out-File (Join-Path $taskOutput 'result.json') -Encoding utf8 -NoClobber
if($taskFailure){throw $taskFailure}
