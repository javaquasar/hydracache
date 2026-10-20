$ErrorActionPreference='Stop'
$taskSource='bc53b78fbabd7481d8f2f6bbbb5cc964e380fd65'
foreach($taskCheck in @(
    @('format',@('fmt','-p','xtask','-p','hydracache-long-run-supervisor-074','--','--check')),
    @('xtask-lint',@('clippy','--locked','-p','xtask','--all-targets','--all-features','--','-D','warnings')),
    @('doc-check',@('xtask','doc-check')),
    @('registry',@('xtask','gated-test-check')),
    @('contract-check',@('xtask','performance-contract-check','--release','0.74'))
)){
    $taskName=$taskCheck[0];$taskArgs=$taskCheck[1]
    if(Test-Path "$PSScriptRoot/$taskName-exit.json"){throw 'Never replay a recorded check'}
    & cargo @taskArgs 2>&1|Tee-Object "$PSScriptRoot/$taskName.log"|Out-Null
    $taskExit=$LASTEXITCODE
    [ordered]@{source=$taskSource;exit=$taskExit;server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$taskName-exit.json"
    Write-Output "$taskName exit=$taskExit"
    if($taskExit -ne 0){throw 'Local check refused'}
}
