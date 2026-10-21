#requires -Version 7.2
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskOutput=Join-Path $taskRepo 'target/reserved-leaf-audit-checks'
New-Item -ItemType Directory -Path $taskOutput -ErrorAction Stop|Out-Null
$taskSteps=@()
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskOutput "$Name.log"
    if(Test-Path $taskLog){throw 'Never replace audit check evidence'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    if($taskCode -ne 0){throw "Offline audit check refused: $Name ($taskCode)"}
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash -LiteralPath $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
}
Record 'audit-controls' { & python -B tools/offline-reserved-leaf-audit-074/test_audit.py }
Record 'round-controls' { & python -B tools/local-profile-session-074/test_attribute_material_rounds.py }
Record 'reader-controls' { & python -B tools/local-profile-session-074/test_material_live.py }
Record 'arithmetic-controls' { & python -B tools/local-profile-session-074/test_attribute_partial.py }
Record 'derived-facts' { & python -B tools/offline-reserved-leaf-audit-074/audit.py --output docs/testing/performance/0.74/local-runs/reserved-leaf-audit-20261021/facts.json --verify }
Record 'source-canaries' { & cargo test --offline --locked -p xtask --test projection_groups_074 --test pidfd_info_path_074 --test held_material_live_074 --test held_material_074 --test held_unit_projection_074 --test boot_operations_074 --test held_preparation_cost_074 }
Record 'check' { & cargo check --offline --locked -p xtask --all-targets }
Record 'clippy' { & cargo clippy --offline --locked -p xtask --all-targets --all-features -- -D warnings }
Record 'docs' { & cargo xtask doc-check }
Record 'registry' { & cargo xtask gated-test-check }
Record 'performance' { & cargo xtask performance-contract-check --release 0.74 }
Record 'protected-source' { & git diff --exit-code e59190ffa431f36d9fc37998e60b368bcc2a1623 -- tools/long-run-supervisor-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml }
@{steps=$taskSteps;new_profile_invocations=0;runtime_changed=$false;observer_changed=$false;server=$false;product=$false}|ConvertTo-Json -Depth 8|Out-File (Join-Path $taskOutput 'result.json') -Encoding utf8 -NoClobber
