param([Parameter(Mandatory)][ValidateSet('Before','After')][string]$Stage,
      [Parameter(Mandatory)][ValidateSet('194','188','xtask','contracts','repository')][string]$Lane)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskPrefix=Join-Path $PSScriptRoot "$Stage-check-$Lane"
if((Test-Path "$taskPrefix.log") -or (Test-Path "$taskPrefix-exit.json")){throw 'Never overwrite evidence'}
$taskSource=(& git rev-parse HEAD).Trim()
$taskSteps=@()
function Invoke-Recorded([string]$Name,[scriptblock]$Call){
    $taskLog="$taskPrefix-$Name.log"
    if(Test-Path $taskLog){throw 'Never overwrite step'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Failed $Name ($taskCode)"}
}
$taskFailure=$null
try{
    if($Lane -in @('194','188')){
        $taskVersion=if($Lane -ceq '194'){'1.94.0'}else{'1.88.0'}
        $taskTarget="CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Lane-check"
        foreach($taskSelector in @('authority_context_','authority_hex_','authority_', 'validation_leaf_requests_','fixed_leaf_alloc_counter','fixed_leaf_path_differential','fixed_leaf_path_preserves','kernel_time_','kernel_worker_context_','authority_fixed_','authority_fixture_','worker_trust_input_')){
            Invoke-Recorded $taskSelector { & wsl.exe -d Ubuntu -- env $taskTarget /home/artur/.cargo/bin/cargo "+$taskVersion" test --locked --release -p hydracache-long-run-supervisor-074 --features fixed-leaf-alloc-probe --lib $taskSelector -- --test-threads=1 }
        }
        Invoke-Recorded 'check' { & wsl.exe -d Ubuntu -- env $taskTarget /home/artur/.cargo/bin/cargo "+$taskVersion" check --locked -p hydracache-long-run-supervisor-074 --all-targets }
        Invoke-Recorded 'lint' { & wsl.exe -d Ubuntu -- env $taskTarget /home/artur/.cargo/bin/cargo "+$taskVersion" clippy --locked -p hydracache-long-run-supervisor-074 --all-targets --all-features -- -D warnings }
        Invoke-Recorded 'build' { & wsl.exe -d Ubuntu -- env $taskTarget /home/artur/.cargo/bin/cargo "+$taskVersion" test --locked --release -p hydracache-long-run-supervisor-074 --features fixed-leaf-alloc-probe --lib --no-run --message-format=json }
    }elseif($Lane -ceq 'xtask'){
        Invoke-Recorded 'focused' { & cargo test --locked -p xtask --test validation_leaf_requests_074 --test authority_key_hex_074 --test authority_context_074 -- --test-threads=2 }
        Invoke-Recorded 'check' { & cargo check --locked -p xtask --all-targets }
        Invoke-Recorded 'lint' { & cargo clippy --locked -p xtask --all-targets --all-features -- -D warnings }
    }elseif($Lane -ceq 'contracts'){
        Invoke-Recorded 'historical' { & cargo test --locked -p xtask --test diagnostic_artifact_fixture_074 --test fresh_kernel_cost_074 --test held_preparation_cost_074 --test performance_contract_074 --test pid_path_cost_074 --test custody_preparation_cost_074 --test document_scratch_cost_074 --test remaining_preparation_cost_074 --test starting_effects_cost_074 --test validation_round_cost_074 --test validation_leaf_requests_074 --test authority_key_hex_074 --test authority_context_074 -- --test-threads=4 }
    }else{
        Invoke-Recorded 'format' { & wsl.exe -d Ubuntu -- /home/artur/.cargo/bin/cargo +1.94.0 fmt --all -- --check }
        Invoke-Recorded 'docs' { & cargo xtask doc-check }
        Invoke-Recorded 'registry' { & cargo xtask gated-test-check }
        Invoke-Recorded 'performance' { & cargo xtask performance-contract-check --release 0.74 }
    }
}catch{$taskFailure=$_.Exception.Message}
[ordered]@{stage=$Stage;source=$taskSource;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();steps=$taskSteps;failure=$taskFailure;server=$false;product=$false}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
if($taskFailure){throw $taskFailure}
