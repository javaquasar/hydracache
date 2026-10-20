param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask)){throw 'Selected source changed'}
foreach($taskCheck in @(
    @('format',@('fmt','-p','xtask','-p','hydracache-long-run-supervisor-074','--','--check')),
    @('xtask-lint',@('clippy','--locked','-p','xtask','--all-targets','--all-features','--','-D','warnings')),
    @('source-contracts',@('test','--locked','-p','xtask','--test','diagnostic_artifact_fixture_074','--test','fresh_kernel_cost_074','--test','held_preparation_cost_074','--test','performance_contract_074','--test','pid_path_cost_074','--test','custody_preparation_cost_074','--test','document_scratch_cost_074','--test','remaining_preparation_cost_074','--','--test-threads=4')),
    @('doc-check',@('xtask','doc-check')),
    @('registry',@('xtask','gated-test-check')),
    @('contract-check',@('xtask','performance-contract-check','--release','0.74'))
)){
    $taskName="review-$($taskCheck[0])";$taskArgs=$taskCheck[1]
    if(Test-Path "$PSScriptRoot/$taskName-exit.json"){throw 'Never replace a check'}
    & cargo @taskArgs *> "$PSScriptRoot/$taskName.log"
    $taskExit=[int]$LASTEXITCODE
    [ordered]@{source=$Source;exit=$taskExit;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$taskName-exit.json"
    Write-Output "$taskName exit=$taskExit"
    if($taskExit -ne 0){throw 'Local check refused'}
}
