$ErrorActionPreference='Stop'
Push-Location (Resolve-Path "$PSScriptRoot/../../../../../..")
try {
    foreach($taskCheck in @(
        @('format',@('fmt','-p','xtask','-p','hydracache-long-run-supervisor-074','--','--check')),
        @('doc-check',@('xtask','doc-check')),
        @('registry',@('xtask','gated-test-check')),
        @('contract-check',@('xtask','performance-contract-check','--release','0.74'))
    )){
        $taskName=$taskCheck[0];$taskArgs=$taskCheck[1]
        if(Test-Path "$PSScriptRoot/$taskName.log"){throw 'Never replay a recorded check'}
        & cargo @taskArgs 2>&1|Tee-Object "$PSScriptRoot/$taskName.log"|Out-Null
        $taskExit=$LASTEXITCODE
        [ordered]@{source=(& git rev-parse HEAD).Trim();recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();exit=$taskExit;server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$taskName-exit.json"
        Write-Output "$taskName exit=$taskExit"
        if($taskExit -ne 0){throw 'Local check refused'}
    }
} finally {Pop-Location}
