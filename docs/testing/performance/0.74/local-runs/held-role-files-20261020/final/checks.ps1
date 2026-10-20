param([Parameter(Mandatory)][ValidateSet('194','188','windows','repository')][string]$Mode,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../../..')).Path
Push-Location $taskRoot
try {
    function Frozen {
        if((& git rev-parse HEAD).Trim() -cne $Source){throw 'Source changed'}
        $taskPaths=@('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests','crates/xtask/src','crates/xtask/tests','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
        & git diff --exit-code $Source -- @taskPaths | Out-Null
        if($LASTEXITCODE -ne 0 -or (& git status --porcelain -- @taskPaths)){throw 'Selected inputs changed'}
        if((Get-FileHash docs/testing/performance/0.74/qualification-manifest.toml -Algorithm SHA256).Hash.ToLowerInvariant() -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Qualification changed'}
    }
    function Run([string]$Name,[string]$Program,[string[]]$CommandArgs,[int]$Passed=-1){
        $taskLog=Join-Path $PSScriptRoot "$Mode-$Name.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Retain existing log'}
        Write-Host "Running $Mode-$Name"
        & $Program @CommandArgs 2>&1 | Tee-Object -FilePath $taskLog | Out-Null
        if($LASTEXITCODE -ne 0){throw "Failed: $Mode-$Name"}
        if((Get-Content -Raw -LiteralPath $taskLog) -match 'compiler unexpectedly panicked|internal compiler error|thread .rustc. .*panicked'){throw "Compiler panic: $Mode-$Name"}
        if($Passed -ge 0 -and -not (Get-Content -Raw -LiteralPath $taskLog).Contains("test result: ok. $Passed passed; 0 failed; 0 ignored;")){throw 'Wrong test selection'}
    }
    Frozen
    $taskRecipe=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
    [ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;original_join=$false;role_receipt_issued=$false;original_send=$false;server=$false;supervisor=$false;product=$false;full_joined_sigkill=$false;clock_changed=$false} | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot "$Mode-identity.json") -Encoding utf8NoBOM
    if($Mode -ceq 'repository'){
        Run 'source-guards' 'cargo' @('test','--locked','-p','xtask','--test','diagnostic_artifact_fixture_074') 12
        Run 'old-bridge' 'cargo' @('test','--locked','-p','xtask','--test','performance_contract_074','fixed_slice_') 3
        Run 'clippy' 'cargo' @('clippy','--locked','-p','hydracache-long-run-supervisor-074','-p','xtask','--all-targets','--all-features','--','-D','warnings')
        Run 'registry' 'cargo' @('xtask','gated-test-check')
        Run 'docs' 'cargo' @('xtask','doc-check')
        Run 'contract' 'cargo' @('xtask','performance-contract-check','--release','0.74')
        Run 'fmt' 'cargo' @('fmt','-p','xtask','-p','hydracache-long-run-supervisor-074','--','--check')
        $taskTests=12;$taskDocs=0
    }else{
        if($Mode -ceq 'windows'){
            $taskProgram='cargo';$taskPrefix=@();$taskTests=16;$taskDocs=2
            Run 'compiler' 'rustc' @('-Vv')
        }else{
            $taskVersion=if($Mode -ceq '194'){'1.94.0'}else{'1.88.0'}
            $taskProgram='wsl.exe';$taskTests=26;$taskDocs=4
            $taskPrefix=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Mode-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
            Run 'compiler' 'wsl.exe' @('-d','Ubuntu','--','/home/artur/.cargo/bin/rustc',"+$taskVersion",'-Vv')
        }
        Run 'build' $taskProgram ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074','--test','held_build','--no-run','--message-format=json'))
        $taskRows=@(Get-Content (Join-Path $PSScriptRoot "$Mode-build.log") | Where-Object {$_.StartsWith('{',[StringComparison]::Ordinal)} | ForEach-Object {$_|ConvertFrom-Json})
        if(@($taskRows | Where-Object {$_.reason -ceq 'build-finished' -and $_.success -ceq $true}).Count -ne 1){throw 'Incomplete build'}
        $taskImages=@($taskRows | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'held_build' -and $_.profile.test -ceq $true -and $_.executable})
        if($taskImages.Count -ne 1){throw 'Wrong image selection'}
        $taskImage=$taskImages[0].executable
        function ImageHash {
            if($Mode -ceq 'windows'){return (Get-FileHash $taskImage -Algorithm SHA256).Hash.ToLowerInvariant()}
            $taskLine=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
            if($LASTEXITCODE -ne 0 -or $taskLine -notmatch '^[0-9a-f]{64}  /'){throw 'Missing image'}
            $taskLine.Substring(0,64)
        }
        $taskHash=ImageHash
        [ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;path=$taskImage;sha256=$taskHash;role_receipt_issued=$false} | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot "$Mode-image.json") -Encoding utf8NoBOM
        foreach($taskRound in 1..2){
            if($Mode -ceq 'windows'){
                Run "tests-$taskRound" $taskImage @('--nocapture','--test-threads=1') $taskTests
            }else{
                Run "tests-$taskRound" 'wsl.exe' @('-d','Ubuntu','--',$taskImage,'--nocapture','--test-threads=1') $taskTests
                if(-not (Get-Content -Raw (Join-Path $PSScriptRoot "$Mode-tests-$taskRound.log")).Contains('held_role_files seed=0x0074f11e corruptions=32')){throw 'Missing seeded controls'}
            }
        }
        Run 'capability-docs' $taskProgram ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074','--doc','diagnostic_builder::held_build::','--','--nocapture')) $taskDocs
        if($Mode -cne 'windows'){
            Run 'clippy' $taskProgram ($taskPrefix+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
        }
        if((ImageHash) -cne $taskHash){throw 'Image changed'}
    }
    Frozen
    [ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;passed=$true;tests=$taskTests;docs=$taskDocs;rounds=$(if($Mode -ceq 'repository'){1}else{2});original_join=$false;role_receipt_issued=$false;original_send=$false;server=$false;supervisor=$false;product=$false;full_joined_sigkill=$false;clock_changed=$false} | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot "$Mode-receipt.json") -Encoding utf8NoBOM
}finally{Pop-Location}
