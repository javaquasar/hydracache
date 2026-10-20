$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Push-Location $taskRepo
try {
    $taskPaths=@('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests','crates/xtask/src','crates/xtask/tests','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
    & git diff --exit-code f07dcca31e2e69796644d70fef695404b22d1519 -- @taskPaths
    if($LASTEXITCODE -ne 0 -or (& git status --porcelain -- @taskPaths)){throw 'Frozen selected inputs changed'}
    if((Get-FileHash docs/testing/performance/0.74/qualification-manifest.toml -Algorithm SHA256).Hash.ToLowerInvariant() -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Qualification changed'}
    function Run([string]$Name,[string[]]$CommandArgs){
        $taskPath=Join-Path $PSScriptRoot "local-$Name.log"
        if(Test-Path -LiteralPath $taskPath){throw 'Retain existing result'}
        & cargo @CommandArgs 2>&1 | Tee-Object -FilePath $taskPath
        if($LASTEXITCODE -ne 0){throw "Failed local check: $Name"}
    }
    Run 'guards' @('test','--locked','-p','xtask','--test','diagnostic_artifact_fixture_074')
    Run 'predecessor' @('test','--locked','-p','xtask','--test','performance_contract_074','fixed_slice_')
    Run 'lint' @('clippy','--locked','-p','hydracache-long-run-supervisor-074','-p','xtask','--all-targets','--all-features','--','-D','warnings')
    Run 'registry' @('xtask','gated-test-check')
    Run 'docs' @('xtask','doc-check')
    Run 'contract' @('xtask','performance-contract-check','--release','0.74')
    Run 'fmt' @('fmt','-p','xtask','-p','hydracache-long-run-supervisor-074','--','--check')
    [ordered]@{source='f07dcca31e2e69796644d70fef695404b22d1519';passed=$true;checks=@('guards','predecessor','lint','registry','docs','contract','fmt');recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()}|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot 'local-checks.json') -Encoding utf8NoBOM
}finally{Pop-Location}
