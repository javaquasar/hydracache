$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Push-Location $taskRoot
try {
    $taskSource=(& git rev-parse HEAD).Trim()
    [ordered]@{source=$taskSource;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();source_tests=6;windows_lib_tests=60;windows_doc_tests=20;original_join=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'checks-identity.json') -Encoding utf8NoBOM
    function Run([string]$Name,[string[]]$Arguments){
        $taskLog=Join-Path $PSScriptRoot "$Name.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Retain existing check'}
        Write-Host "Running $Name"
        & cargo @Arguments 2>&1 | Tee-Object -FilePath $taskLog | Out-Null
        if($LASTEXITCODE -ne 0){throw "Failed $Name"}
    }
    Run 'final-clippy' @('clippy','--locked','-p','hydracache-long-run-supervisor-074','-p','xtask','--all-targets','--all-features','--','-D','warnings')
    Run 'final-source-guard' @('test','--locked','-p','xtask','--test','diagnostic_artifact_fixture_074','--','--test-threads=1')
    Run 'final-windows-lib' @('test','--locked','-p','hydracache-long-run-supervisor-074','--lib')
    Run 'final-windows-doc' @('test','--locked','-p','hydracache-long-run-supervisor-074','--doc')
    Run 'final-registry' @('xtask','gated-test-check')
    Run 'final-docs' @('xtask','doc-check')
    Run 'final-contract' @('xtask','performance-contract-check','--release','0.74')
    & cargo fmt -p xtask -p hydracache-long-run-supervisor-074 -- --check
    if($LASTEXITCODE -ne 0){throw 'Formatting'}
    foreach($taskPair in @(@('final-source-guard',6),@('final-windows-lib',60),@('final-windows-doc',20))){
        if(-not (Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$($taskPair[0]).log")).Contains("test result: ok. $($taskPair[1]) passed; 0 failed; 0 ignored;")){throw 'Incomplete test target'}
    }
    if((& git rev-parse HEAD).Trim() -cne $taskSource -or (& git status --porcelain -- tools/long-run-supervisor-074/src tools/long-run-supervisor-074/tests crates/xtask/tests docs/testing/gated-test-registry.toml Cargo.lock)){throw 'Source drift'}
    [ordered]@{source=$taskSource;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();source_tests=6;windows_lib_tests=60;windows_doc_tests=20;strict_lint=$true;format=$true;registry=$true;docs=$true;local_contract=$true;original_join=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'checks-receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
