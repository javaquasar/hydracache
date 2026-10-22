#requires -Version 7.2
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskOutput=Join-Path $taskRepo 'target/authority-read-baseline-final-checks-first'
if(Test-Path -LiteralPath $taskOutput){throw 'Never replace final baseline checks'}
New-Item -ItemType Directory -Path $taskOutput -ErrorAction Stop|Out-Null
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskOutput "$Name.log"
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash -LiteralPath $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Final read baseline check refused: $Name ($taskCode)"}
}
try{
    Record 'format' { & rustfmt --check --edition 2021 tools/authority-read-baseline-074/probe.rs crates/xtask/tests/authority_read_baseline_074.rs }
    Record 'workspace-format' { & cargo fmt -p xtask --check }
    Record 'review-controls' { & python -B tools/authority-read-baseline-074/test_review_read.py }
    Record 'prior-audit-controls' { & python -B tools/offline-authority-read-audit-074/test_audit_read.py }
    Record 'prior-inline-controls' { & python -B tools/sample-boot-inline-074/test_review_inline.py }
    Record 'prior-boot-controls' { & python -B tools/sample-boot-baseline-074/test_review_boot.py }
    Record 'prior-proc-controls' { & python -B tools/proc-document-baseline-074/test_review.py }
    Record 'prior-reconciliation-controls' { & python -B tools/offline-reserved-reconciliation-074/test_reconcile.py }
    Record 'derived-facts' { & python -B tools/authority-read-baseline-074/review_read.py --output docs/testing/performance/0.74/local-runs/authority-read-baseline-20261022/facts.json --verify }
    Record 'docs' { & cargo xtask doc-check }
    Record 'registry' { & cargo xtask gated-test-check }
    Record 'performance' { & cargo xtask performance-contract-check --release 0.74 }
    Record 'protected-source' { & git diff --exit-code 94114c62 -- tools/long-run-supervisor-074 crates Cargo.toml Cargo.lock docs/COMPAT.md docs/plans/releases.toml docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml ':(exclude)crates/xtask/tests/authority_read_baseline_074.rs' }
}catch{$taskFailure=$_.Exception.Message}
@{steps=$taskSteps;failure=$taskFailure;runtime_candidate_implemented=$false;original_owner_profiles=0;server=$false;product=$false}|ConvertTo-Json -Depth 8|Out-File (Join-Path $taskOutput 'result.json') -Encoding utf8 -NoClobber
if($taskFailure){throw $taskFailure}
