#requires -Version 7.2
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskOutput=Join-Path $taskRepo 'target/sample-boot-baseline-final-checks-first'
if(Test-Path -LiteralPath $taskOutput){throw 'Never replace final baseline checks'}
New-Item -ItemType Directory -Path $taskOutput -ErrorAction Stop|Out-Null
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskOutput "$Name.log"
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash -LiteralPath $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Final baseline check refused: $Name ($taskCode)"}
}
try{
    Record 'format' { & rustfmt --check --edition 2021 tools/sample-boot-baseline-074/probe.rs crates/xtask/tests/sample_boot_baseline_074.rs }
    Record 'review-controls' { & python -B tools/sample-boot-baseline-074/test_review_boot.py }
    Record 'prior-proc-controls' { & python -B tools/proc-document-baseline-074/test_review.py }
    Record 'prior-reconciliation-controls' { & python -B tools/offline-reserved-reconciliation-074/test_reconcile.py }
    Record 'derived-facts' { & python -B tools/sample-boot-baseline-074/review_boot.py --output docs/testing/performance/0.74/local-runs/sample-boot-baseline-20261022/facts.json --verify }
    Record 'docs' { & cargo xtask doc-check }
    Record 'registry' { & cargo xtask gated-test-check }
    Record 'performance' { & cargo xtask performance-contract-check --release 0.74 }
    Record 'protected-source' { & git diff --exit-code 79dcf64b -- tools/long-run-supervisor-074 crates Cargo.toml Cargo.lock docs/COMPAT.md docs/plans/releases.toml docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml ':(exclude)crates/xtask/tests/sample_boot_baseline_074.rs' }
}catch{$taskFailure=$_.Exception.Message}
@{steps=$taskSteps;failure=$taskFailure;runtime_candidate_implemented=$false;representation_changed=$false;original_owner_profiles=0;server=$false;product=$false}|ConvertTo-Json -Depth 8|Out-File (Join-Path $taskOutput 'result.json') -Encoding utf8 -NoClobber
if($taskFailure){throw $taskFailure}
