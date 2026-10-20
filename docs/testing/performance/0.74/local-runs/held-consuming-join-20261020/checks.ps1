param([Parameter(Mandatory)][ValidateSet('194','188','windows','repository')][string]$Mode,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOutput=Join-Path $PSScriptRoot 'affinity-smoke'
if(-not (Test-Path -LiteralPath $taskOutput)){New-Item -ItemType Directory -Path $taskOutput | Out-Null}
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
        $taskLog=Join-Path $taskOutput "$Mode-$Name.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Retain existing log'}
        Write-Host "Running $Mode-$Name"
        & $Program @CommandArgs 2>&1 | Tee-Object -FilePath $taskLog | Out-Null
        $taskExit=$LASTEXITCODE
        if($taskExit -ne 0){throw "Failed: $Mode-$Name"}
        $taskRaw=if(Test-Path -LiteralPath $taskLog){Get-Content -Raw -LiteralPath $taskLog}else{''}
        if(-not (Test-Path -LiteralPath $taskLog)){
            [ordered]@{source=$Source;mode=$Mode;check=$Name;native_exit=$taskExit;empty_output=$true} | ConvertTo-Json | Set-Content (Join-Path $taskOutput "$Mode-$Name-empty-output.json") -Encoding utf8NoBOM
        }
        if($taskRaw -match 'compiler unexpectedly panicked|internal compiler error|thread .rustc. .*panicked'){throw "Compiler panic: $Mode-$Name"}
        if($Passed -ge 0 -and -not $taskRaw.Contains("test result: ok. $Passed passed; 0 failed; 0 ignored;")){throw 'Wrong test selection'}
    }
    Frozen
    $taskRecipe=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $taskFlags=[ordered]@{local_cpu_affinity=@(if($Mode -in @('194','188')){0});unrestricted_cpu_gate_passed=$false;typed_consuming_join=($Mode -in @('194','188'));kernel_role_bridge=($Mode -in @('194','188'));privileged_full_join=$false;late_original_join=$false;production_enrollment=$false;production_role_receipt_issued=$false;original_send=$false;server=$false;supervisor=$false;product=$false;full_joined_sigkill=$false;clock_changed=$false}
    $taskIdentity=[ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe}
    foreach($taskFlag in $taskFlags.Keys){$taskIdentity[$taskFlag]=$taskFlags[$taskFlag]}
    $taskIdentity | ConvertTo-Json | Set-Content (Join-Path $taskOutput "$Mode-identity.json") -Encoding utf8NoBOM
    if($Mode -ceq 'repository'){
        Run 'source-guards' 'cargo' @('test','--locked','-p','xtask','--test','diagnostic_artifact_fixture_074') 18
        Run 'old-bridge' 'cargo' @('test','--locked','-p','xtask','--test','performance_contract_074','fixed_slice_') 3
        Run 'clippy' 'cargo' @('clippy','--locked','-p','hydracache-long-run-supervisor-074','-p','xtask','--all-targets','--all-features','--','-D','warnings')
        Run 'registry' 'cargo' @('xtask','gated-test-check')
        Run 'docs' 'cargo' @('xtask','doc-check')
        Run 'contract' 'cargo' @('xtask','performance-contract-check','--release','0.74')
        Run 'fmt' 'cargo' @('fmt','-p','xtask','-p','hydracache-long-run-supervisor-074','--','--check')
        $taskNewTests=0;$taskOldTests=18;$taskDocs=0
    }else{
        if($Mode -ceq 'windows'){
            $taskProgram='cargo';$taskPrefix=@();$taskNewTests=0;$taskOldTests=28;$taskDocs=4
            Run 'compiler' 'rustc' @('-Vv')
        }else{
            $taskVersion=if($Mode -ceq '194'){'1.94.0'}else{'1.88.0'}
            $taskProgram='wsl.exe';$taskNewTests=17;$taskOldTests=38;$taskDocs=11
            $taskPrefix=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Mode-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
            Run 'compiler' 'wsl.exe' @('-d','Ubuntu','--','/home/artur/.cargo/bin/rustc',"+$taskVersion",'-Vv')
        }
        $taskSelections=@([pscustomobject]@{name='old';target='held_build';args=@('--test','held_build');filter=@();count=$taskOldTests})
        if($Mode -cne 'windows'){$taskSelections+=([pscustomobject]@{name='new';target='hydracache_long_run_supervisor_074';args=@('--lib');filter=@('consuming_held');count=$taskNewTests})}
        foreach($taskSelection in $taskSelections){
            Run "$($taskSelection.name)-build" $taskProgram ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074')+$taskSelection.args+@('--no-run','--message-format=json'))
            $taskRows=@(Get-Content (Join-Path $taskOutput "$Mode-$($taskSelection.name)-build.log") | Where-Object {$_.StartsWith('{',[StringComparison]::Ordinal)} | ForEach-Object {$_|ConvertFrom-Json})
            if(@($taskRows | Where-Object {$_.reason -ceq 'build-finished' -and $_.success -ceq $true}).Count -ne 1){throw 'Incomplete build'}
            $taskImages=@($taskRows | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq $taskSelection.target -and $_.profile.test -ceq $true -and $_.executable})
            if($taskImages.Count -ne 1){throw 'Wrong image selection'}
            $taskImage=$taskImages[0].executable
            function ImageHash {
                if($Mode -ceq 'windows'){return (Get-FileHash $taskImage -Algorithm SHA256).Hash.ToLowerInvariant()}
                $taskLine=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
                if($LASTEXITCODE -ne 0 -or $taskLine -notmatch '^[0-9a-f]{64}  /'){throw 'Missing image'}
                $taskLine.Substring(0,64)
            }
            $taskHash=ImageHash
            [ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;selection=$taskSelection.name;path=$taskImage;sha256=$taskHash} | ConvertTo-Json | Set-Content (Join-Path $taskOutput "$Mode-$($taskSelection.name)-image.json") -Encoding utf8NoBOM
            foreach($taskRound in 1..2){
                $taskArgs=$taskSelection.filter+@('--nocapture','--test-threads=1')
                if($Mode -ceq 'windows'){Run "$($taskSelection.name)-tests-$taskRound" $taskImage $taskArgs $taskSelection.count}
                else{Run "$($taskSelection.name)-tests-$taskRound" 'wsl.exe' (@('-d','Ubuntu','--','taskset','-c','0',$taskImage)+$taskArgs) $taskSelection.count}
                if($taskSelection.name -ceq 'new'){
                    Run "trust-$taskRound" 'wsl.exe' @('-d','Ubuntu','--','taskset','-c','0',$taskImage,'held_runtime::tests::','--skip','consuming_held_bridge','--nocapture','--test-threads=1') 26
                    Run "fixed-$taskRound" 'wsl.exe' @('-d','Ubuntu','--','taskset','-c','0',$taskImage,'fixed_slice::tests::fixed_placement_','--nocapture','--test-threads=1') 5
                }
            }
            if((ImageHash) -cne $taskHash){throw 'Image changed'}
        }
        Run 'old-capability-docs' $taskProgram ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074','--doc','diagnostic_builder::held_build::','--','--nocapture')) $(if($Mode -ceq 'windows'){4}else{6})
        if($Mode -cne 'windows'){
            Run 'new-capability-docs' $taskProgram ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074','--doc','held_runtime::','--','--nocapture')) 5
            Run 'clippy' $taskProgram ($taskPrefix+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
        }
    }
    Frozen
    $taskReceipt=[ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;passed=$true;new_tests=$taskNewTests;old_tests=$taskOldTests;docs=$taskDocs;rounds=$(if($Mode -ceq 'repository'){1}else{2})}
    foreach($taskFlag in $taskFlags.Keys){$taskReceipt[$taskFlag]=$taskFlags[$taskFlag]}
    $taskReceipt | ConvertTo-Json | Set-Content (Join-Path $taskOutput "$Mode-receipt.json") -Encoding utf8NoBOM
}finally{Pop-Location}
