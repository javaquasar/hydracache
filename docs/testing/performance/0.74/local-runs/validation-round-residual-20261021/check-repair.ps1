param([Parameter(Mandatory)][ValidateSet('194','188')][string]$Lane,[Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask Cargo.lock docs/testing/gated-test-registry.toml)){throw 'Source drift'}
$taskPrefix=Join-Path $PSScriptRoot "check-Verified-$Lane"
if(Test-Path "$taskPrefix-exit.json"){throw 'Never overwrite checks'}
$taskSteps=@();$taskFailure=$null
function Run([string]$Name,[scriptblock]$Call){
    $taskLog="$taskPrefix-$($Name.Replace(':','_')).log"
    if(Test-Path $taskLog){throw 'Never overwrite step'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Focused check refused: $Name"}
}
$taskVersion=if($Lane -ceq '194'){'1.94.0'}else{'1.88.0'}
$taskTarget="CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Lane-check"
try{
    Run 'check' { & wsl.exe -d Ubuntu -- env $taskTarget /home/artur/.cargo/bin/cargo "+$taskVersion" check --locked -p hydracache-long-run-supervisor-074 --all-targets }
    Run 'lint' { & wsl.exe -d Ubuntu -- env $taskTarget /home/artur/.cargo/bin/cargo "+$taskVersion" clippy --locked -p hydracache-long-run-supervisor-074 --all-targets --all-features -- -D warnings }
    Run 'build' { & wsl.exe -d Ubuntu -- env $taskTarget /home/artur/.cargo/bin/cargo "+$taskVersion" test --locked --release -p hydracache-long-run-supervisor-074 --lib --no-run --message-format=json }
    foreach($taskSelector in @('round_cost_','validation_frame_','collector_starting_','consuming_held','diagnostic_kernel_admission::','held_runtime::tests::','preparation_cost_observer','custody_boundary_capture')){
        $taskSkip=if($taskSelector -ceq 'held_runtime::tests::'){@('--skip','consuming_held_bridge')}else{@()}
        Run $taskSelector { & wsl.exe -d Ubuntu -- env $taskTarget /home/artur/.cargo/bin/cargo "+$taskVersion" test --locked --release -p hydracache-long-run-supervisor-074 --lib $taskSelector -- @taskSkip --test-threads=1 }
    }
}catch{$taskFailure=$_.Exception.Message}
@{source=$Source;lane=$Lane;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();steps=$taskSteps;failure=$taskFailure;allocator_probe=$false;server=$false;product=$false}|ConvertTo-Json -Depth 7|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
if($taskFailure){throw $taskFailure}
