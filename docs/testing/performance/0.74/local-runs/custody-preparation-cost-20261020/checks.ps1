param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
foreach($taskCheck in @(
    @('format',@('fmt','-p','xtask','-p','hydracache-long-run-supervisor-074','--','--check')),
    @('xtask-lint',@('clippy','--locked','-p','xtask','--all-targets','--all-features','--','-D','warnings')),
    @('doc-check',@('xtask','doc-check')),
    @('registry',@('xtask','gated-test-check')),
    @('contract-check',@('xtask','performance-contract-check','--release','0.74'))
)){
    $taskName=$taskCheck[0];$taskArgs=$taskCheck[1]
    if(Test-Path "$PSScriptRoot/$taskName-exit.json"){throw 'Never replay a check'}
    & cargo @taskArgs 2>&1|Tee-Object "$PSScriptRoot/$taskName.log"|Out-Null
    $taskExit=$LASTEXITCODE
    [ordered]@{source=$Source;exit=$taskExit;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$taskName-exit.json"
    Write-Output "$taskName exit=$taskExit"
    if($taskExit -ne 0){throw 'Local check refused'}
}
