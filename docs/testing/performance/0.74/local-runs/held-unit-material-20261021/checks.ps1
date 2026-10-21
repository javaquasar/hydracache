param([ValidateSet('194','188','host')][string]$Lane,[ValidateSet('initial','final')][string]$Cohort='initial')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskOutput=$PSScriptRoot
if($Cohort -ceq 'final'){
    $taskOutput=Join-Path $PSScriptRoot 'final'
    if(-not(Test-Path -LiteralPath $taskOutput)){New-Item -ItemType Directory -Path $taskOutput|Out-Null}
}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function WriteNew([string]$Name,$Value){
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject $Value -Depth 30)+"`n")
    $taskStream=[IO.File]::Open((Join-Path $taskOutput $Name),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
function Context {
    $taskPaths=@(& git ls-files --cached --others --exclude-standard -- tools/long-run-supervisor-074/src tools/long-run-supervisor-074/tests crates/xtask/src crates/xtask/tests)
    if($LASTEXITCODE -ne 0){throw 'git source list'}
    $taskPaths+=@('Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
    @($taskPaths|Sort-Object -Unique|ForEach-Object {@{path=$_;sha256=(Hash $_)}})
}
$taskBefore=Context
WriteNew "$Lane-before.json" $taskBefore
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskOutput "$Lane-$Name.log"
    if(Test-Path -LiteralPath $taskLog){throw 'Never overwrite check evidence'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Hash $taskLog)}
    if($taskCode -ne 0){throw "Check refused: $Lane $Name ($taskCode)"}
}
try {
    if($Lane -eq 'host'){
        Record 'canaries' { & cargo test --offline --locked -p xtask --test held_material_074 --test held_unit_projection_074 --test boot_operations_074 --test held_preparation_cost_074 }
        Record 'check' { & cargo check --offline --locked -p xtask -p hydracache-long-run-supervisor-074 --all-targets }
        Record 'clippy' { & cargo clippy --offline --locked -p xtask -p hydracache-long-run-supervisor-074 --all-targets --all-features -- -D warnings }
        Record 'format' { & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 /home/artur/.cargo/bin/cargo +1.94.0 fmt --all -- --check }
        Record 'docs' { & cargo xtask doc-check }
        Record 'registry' { & cargo xtask gated-test-check }
        Record 'performance' { & cargo xtask performance-contract-check --release 0.74 }
        Record 'diff' { & git diff --check }
    }else{
        $taskCompiler=if($Lane -eq '194'){'+1.94.0'}else{'+1.88.0'}
        $taskTarget="CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Lane-check"
        foreach($taskCase in @(@('material','held_material',12),@('projection','held_unit_projection',10),@('consuming','consuming_join',10),@('reply','dispatch_reply',12))){
            $taskFilter=$taskCase[1]
            Record $taskCase[0] { & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 $taskTarget /home/artur/.cargo/bin/cargo $taskCompiler test --offline --locked --release -p hydracache-long-run-supervisor-074 --lib $taskFilter -- --nocapture --test-threads=1 }
            if((Get-Content (Join-Path $taskOutput "$Lane-$($taskCase[0]).log") -Raw) -notmatch "test result: ok\. $($taskCase[2]) passed; 0 failed; 0 ignored;"){throw 'Selected test count'}
        }
        Record 'roles' { & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 $taskTarget /home/artur/.cargo/bin/cargo $taskCompiler test --offline --locked --release -p hydracache-long-run-supervisor-074 --test held_build -- --nocapture --test-threads=1 }
        Record 'check' { & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 $taskTarget /home/artur/.cargo/bin/cargo $taskCompiler check --offline --locked -p hydracache-long-run-supervisor-074 --all-targets }
        Record 'clippy' { & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 $taskTarget /home/artur/.cargo/bin/cargo $taskCompiler clippy --offline --locked -p hydracache-long-run-supervisor-074 --all-targets --all-features -- -D warnings }
    }
}catch{$taskFailure=$_.Exception.Message}
$taskAfter=Context
WriteNew "$Lane-after.json" $taskAfter
if((ConvertTo-Json -InputObject $taskBefore -Depth 30 -Compress) -cne (ConvertTo-Json -InputObject $taskAfter -Depth 30 -Compress)){$taskFailure='Source changed during checks'}
WriteNew "$Lane-checks.json" @{base='c9cc5a2e';lane=$Lane;steps=$taskSteps;failure=$taskFailure;new_original_live_fixture=$false;server=$false;original_send=$false;product=$false;performance_measurement=$false}
if($taskFailure){throw $taskFailure}
