$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut=Join-Path $PSScriptRoot 'selected-checks'
if(Test-Path -LiteralPath $taskOut){throw 'Retain existing checks'}
New-Item -ItemType Directory -Path $taskOut | Out-Null
Push-Location $taskRoot
try {
    $taskSource=(& git rev-parse HEAD).Trim()
    $taskRecipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
    [ordered]@{source=$taskSource;recipe=$taskRecipe} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'identity.json') -Encoding utf8NoBOM
    $taskCommands=@(
        @('lint',@('clippy','--locked','-p','hydracache-long-run-supervisor-074','-p','xtask','--all-targets','--all-features','--','-D','warnings')),
        @('source',@('test','--locked','-p','xtask','--test','diagnostic_artifact_fixture_074','--','--test-threads=1')),
        @('windows-lib',@('test','--locked','-p','hydracache-long-run-supervisor-074','--lib')),
        @('windows-doc',@('test','--locked','-p','hydracache-long-run-supervisor-074','--doc')),
        @('registry',@('xtask','gated-test-check')),
        @('docs',@('xtask','doc-check')),
        @('contract',@('xtask','performance-contract-check','--release','0.74'))
    )
    foreach($taskCommand in $taskCommands){
        Write-Host "Running selected $($taskCommand[0])"
        $taskArguments=$taskCommand[1]
        & cargo @taskArguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "$($taskCommand[0]).log") | Out-Null
        if($LASTEXITCODE -ne 0){throw "Failed: $($taskCommand[0])"}
    }
    foreach($taskPair in @(@('source',6),@('windows-lib',60),@('windows-doc',20))){
        if(-not (Get-Content -Raw -LiteralPath (Join-Path $taskOut "$($taskPair[0]).log")).Contains("test result: ok. $($taskPair[1]) passed; 0 failed; 0 ignored;")){throw 'Incomplete target'}
    }
    & cargo fmt -p xtask -p hydracache-long-run-supervisor-074 -- --check
    if($LASTEXITCODE -ne 0){throw 'Formatting'}
    if((& git rev-parse HEAD).Trim() -cne $taskSource -or (& git status --porcelain -- tools/long-run-supervisor-074/src tools/long-run-supervisor-074/tests crates/xtask/tests docs/testing/gated-test-registry.toml Cargo.lock)){throw 'Source drift'}
    [ordered]@{source=$taskSource;recipe=$taskRecipe;source_tests=6;windows_lib_tests=60;windows_doc_tests=20;strict_lint=$true;format=$true;registry=$true;docs=$true;local_contract=$true;original_join=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
