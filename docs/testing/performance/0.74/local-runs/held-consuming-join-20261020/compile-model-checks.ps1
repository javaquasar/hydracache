param([Parameter(Mandatory)][ValidateSet('194','188')][string]$Mode,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOutput=Join-Path $PSScriptRoot 'compile-model'
if(-not (Test-Path $taskOutput)){New-Item -ItemType Directory -Path $taskOutput | Out-Null}
Push-Location $taskRoot
try{
    function Frozen{
        if((& git rev-parse HEAD).Trim() -cne $Source){throw 'Source changed'}
        $taskPaths=@('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests','crates/xtask/src','crates/xtask/tests','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
        & git diff --exit-code $Source -- @taskPaths | Out-Null
        if($LASTEXITCODE -ne 0 -or (& git status --porcelain -- @taskPaths)){throw 'Selected source changed'}
    }
    function Run([string]$Name,[string[]]$CommandArgs,[int]$Count=-1){
        $taskLog=Join-Path $taskOutput "$Mode-$Name.log";if(Test-Path $taskLog){throw 'Retain old log'}
        Write-Host "Running compile-model $Mode-$Name"
        & wsl.exe @CommandArgs 2>&1 | Tee-Object $taskLog | Out-Null
        if($LASTEXITCODE -ne 0){throw "Compile/model check failed: $Mode-$Name"}
        $taskRaw=Get-Content -Raw $taskLog
        if($taskRaw -match 'compiler unexpectedly panicked|internal compiler error|thread .rustc. .*panicked'){throw 'Compiler panic'}
        if($Count -ge 0 -and -not $taskRaw.Contains("test result: ok. $Count passed; 0 failed; 0 ignored;")){throw 'Wrong model/doc selection'}
    }
    Frozen
    $taskVersion=if($Mode -ceq '194'){'1.94.0'}else{'1.88.0'}
    $taskPrefix=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Mode-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
    Run 'build' ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074','--lib','--no-run','--message-format=json'))
    $taskRows=@(Get-Content (Join-Path $taskOutput "$Mode-build.log")|Where-Object {$_.StartsWith('{',[StringComparison]::Ordinal)}|ForEach-Object {$_|ConvertFrom-Json})
    $taskImages=@($taskRows|Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
    if($taskImages.Count -ne 1){throw 'Wrong model image'};$taskImage=$taskImages[0].executable
    function ImageHash{$taskValue=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim();if($LASTEXITCODE -ne 0 -or $taskValue -notmatch '^[0-9a-f]{64}  /'){throw 'Model image missing'};$taskValue.Substring(0,64)}
    $taskHash=ImageHash;$taskRecipe=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
    [ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;image=$taskImage;image_sha256=$taskHash;model_only=$true;real_join_gate_passed=$false}|ConvertTo-Json|Set-Content (Join-Path $taskOutput "$Mode-image.json") -Encoding utf8NoBOM
    foreach($taskRound in 1..2){
        Run "models-$taskRound" @('-d','Ubuntu','--',$taskImage,'consuming_held_tests::','--nocapture','--test-threads=1') 10
        Run "fixed-$taskRound" @('-d','Ubuntu','--',$taskImage,'fixed_slice::tests::fixed_placement_','--nocapture','--test-threads=1') 5
    }
    Run 'clippy' ($taskPrefix+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
    Run 'held-docs' ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074','--doc','diagnostic_builder::held_build::','--','--nocapture')) 6
    Run 'trust-docs' ($taskPrefix+@('test','--locked','-p','hydracache-long-run-supervisor-074','--doc','held_runtime::','--','--nocapture')) 5
    Frozen;if((ImageHash) -cne $taskHash){throw 'Model image changed'}
    [ordered]@{source=$Source;mode=$Mode;recipe=$taskRecipe;passed=$true;models=10;old_fixed_models=5;rounds=2;capability_docs=11;strict_lint=$true;model_only=$true;real_join_gate_passed=$false;unrestricted_cpu_gate_passed=$false;server=$false;supervisor=$false;product=$false}|ConvertTo-Json|Set-Content (Join-Path $taskOutput "$Mode-receipt.json") -Encoding utf8NoBOM
}finally{Pop-Location}
