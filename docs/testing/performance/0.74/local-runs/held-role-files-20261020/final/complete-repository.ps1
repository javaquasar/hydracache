$ErrorActionPreference='Stop'
$taskSource='82570db8589065df5711f2faf51d02406168cf79'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../../..')).Path
Push-Location $taskRoot
try {
    function Frozen {
        if((& git rev-parse HEAD).Trim() -cne $taskSource){throw 'Source changed'}
        $taskPaths=@('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests','crates/xtask/src','crates/xtask/tests','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
        & git diff --exit-code $taskSource -- @taskPaths | Out-Null
        if($LASTEXITCODE -ne 0 -or (& git status --porcelain -- @taskPaths)){throw 'Selected inputs changed'}
        if((Get-FileHash docs/testing/performance/0.74/qualification-manifest.toml -Algorithm SHA256).Hash.ToLowerInvariant() -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Qualification changed'}
    }
    Frozen
    $taskReceiptPath=Join-Path $PSScriptRoot 'repository-receipt.json'
    $taskFormatPath=Join-Path $PSScriptRoot 'repository-format-recheck.json'
    if((Test-Path $taskReceiptPath) -or (Test-Path $taskFormatPath)){throw 'Retain existing receipt'}
    $taskRecipe=(Get-FileHash (Join-Path $PSScriptRoot 'checks.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
    $taskCompletion=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $taskIdentity=Get-Content -Raw (Join-Path $PSScriptRoot 'repository-identity.json')|ConvertFrom-Json
    if($taskIdentity.source -cne $taskSource -or $taskIdentity.recipe -cne $taskRecipe -or $taskIdentity.mode -cne 'repository'){throw 'Original cohort binding'}
    foreach($taskFlag in @('original_join','role_receipt_issued','original_send','server','supervisor','product','full_joined_sigkill','clock_changed')){
        if($taskIdentity.$taskFlag -isnot [bool] -or $taskIdentity.$taskFlag -cne $false){throw 'Scope changed'}
    }
    # These checks completed before the original wrapper reached its silent fmt.
    foreach($taskPair in @(@('source-guards','test result: ok. 12 passed; 0 failed; 0 ignored;'),@('old-bridge','test result: ok. 3 passed; 0 failed; 0 ignored;'),@('clippy','Finished `dev` profile'),@('registry','gated-test-check: OK (186 discovered gates, all registered)'),@('docs','doc-check: OK'),@('contract','performance-contract-check 0.74: OK (local, non-promotable)'))){
        $taskRaw=Get-Content -Raw (Join-Path $PSScriptRoot "repository-$($taskPair[0]).log")
        if(-not $taskRaw.Contains($taskPair[1]) -or $taskRaw -match 'compiler unexpectedly panicked|internal compiler error|thread .rustc. .*panicked'){throw "Incomplete predecessor check: $($taskPair[0])"}
    }
    & cargo fmt -p xtask -p hydracache-long-run-supervisor-074 -- --check
    $taskExit=$LASTEXITCODE
    if($taskExit -ne 0){throw 'Formatting refused'}
    Frozen
    [ordered]@{source=$taskSource;recipe=$taskCompletion;format_exit=$taskExit;passed=$true;selected_inputs_unchanged=$true} | ConvertTo-Json | Set-Content $taskFormatPath -Encoding utf8NoBOM
    [ordered]@{source=$taskSource;mode='repository';recipe=$taskRecipe;completion_recipe=$taskCompletion;completion='silent-format-recheck-after-log-wrapper-refusal';passed=$true;tests=12;docs=0;rounds=1;original_join=$false;role_receipt_issued=$false;original_send=$false;server=$false;supervisor=$false;product=$false;full_joined_sigkill=$false;clock_changed=$false} | ConvertTo-Json | Set-Content $taskReceiptPath -Encoding utf8NoBOM
}finally{Pop-Location}
