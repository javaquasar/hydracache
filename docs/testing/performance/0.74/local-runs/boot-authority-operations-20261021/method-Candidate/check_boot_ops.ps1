param([Parameter(Mandatory)][ValidateSet('Baseline','BaselineRepair','BaselineCanary','Candidate','CandidateRepair')][string]$Stage,
      [Parameter(Mandatory)][ValidateSet('194','188','xtask','repository')][string]$Lane)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/boot-authority-operations-20261021'
if(-not(Test-Path $taskPacket)){New-Item -ItemType Directory -Path $taskPacket|Out-Null}
$taskPrefix=Join-Path $taskPacket "$Stage-check-$Lane"
if(Test-Path "$taskPrefix-exit.json"){throw 'Never repeat a preflight in this prefix'}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call){
    $taskLog="$taskPrefix-$Name.log"
    if(Test-Path $taskLog){throw 'Never overwrite'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Hash $taskLog)}
    if($taskCode -ne 0){throw "Failed $Name"}
}
try{
    if($Lane -in @('194','188')){
        $taskVersion=if($Lane -ceq '194'){'1.94.0'}else{'1.88.0'}
        $taskTarget="CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Lane-check"
        foreach($taskAction in @('check','clippy')){
            $taskArgs=@('--offline','--locked','-p','hydracache-long-run-supervisor-074','--all-targets')
            if($taskAction -ceq 'clippy'){$taskArgs+=@('--all-features','--','-D','warnings')}
            Record $taskAction { & wsl.exe -d Ubuntu -- env RUSTUP_AUTO_INSTALL=0 $taskTarget /home/artur/.cargo/bin/cargo "+$taskVersion" $taskAction @taskArgs }
        }
        foreach($taskSelector in @('boot_operations_','authority_','validation_leaf_requests_','boot_','fixed_leaf_path_','worker_trust_input_','kernel_time_')){
            Record $taskSelector { & wsl.exe -d Ubuntu -- env RUSTUP_AUTO_INSTALL=0 $taskTarget /home/artur/.cargo/bin/cargo "+$taskVersion" test --offline --locked --release -p hydracache-long-run-supervisor-074 --features fixed-leaf-alloc-probe --lib $taskSelector -- --test-threads=1 }
        }
        Record 'build' { & wsl.exe -d Ubuntu -- env RUSTUP_AUTO_INSTALL=0 $taskTarget /home/artur/.cargo/bin/cargo "+$taskVersion" test --offline --locked --release -p hydracache-long-run-supervisor-074 --features fixed-leaf-alloc-probe --lib --no-run --message-format=json }
    }elseif($Lane -ceq 'xtask'){
        $taskTargets=@('diagnostic_artifact_fixture_074','fresh_kernel_cost_074','held_preparation_cost_074','performance_contract_074','pid_path_cost_074','custody_preparation_cost_074','document_scratch_cost_074','remaining_preparation_cost_074','starting_effects_cost_074','validation_round_cost_074','validation_leaf_requests_074','authority_key_hex_074','authority_context_074','authority_leaf_name_074','authority_inventory_074','authority_name_capacity_074','authority_expected_name_074','authority_breakdown_074','authority_fd_path_074','authority_owners_074','authority_cpu_074','authority_leaf_cpu_074','authority_open_name_074','boot_operations_074')
        if($Stage -like 'Candidate*' -or $Stage -ceq 'BaselineCanary'){
            $taskTargets=@('boot_operations_074','authority_open_name_074','authority_leaf_cpu_074','authority_cpu_074','authority_owners_074','authority_breakdown_074','authority_inventory_074','validation_leaf_requests_074')
        }
        $taskArgs=@();foreach($taskTarget in $taskTargets){$taskArgs+=@('--test',$taskTarget)}
        Record 'tests' { & cargo test --locked -p xtask @taskArgs -- --test-threads=4 }
        Record 'check' { & cargo check --locked -p xtask --all-targets }
        Record 'clippy' { & cargo clippy --locked -p xtask --all-targets --all-features -- -D warnings }
    }else{
        Record 'format' { & wsl.exe -d Ubuntu -- env RUSTUP_AUTO_INSTALL=0 /home/artur/.cargo/bin/cargo +1.94.0 fmt --all -- --check }
        Record 'docs' { & cargo xtask doc-check }
        Record 'registry' { & cargo xtask gated-test-check }
        Record 'performance' { & cargo xtask performance-contract-check --release 0.74 }
        Record 'diff' { & git diff --check }
    }
}catch{$taskFailure=$_.Exception.Message}
$taskInputs=@(foreach($taskPath in @('tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs','tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs','tools/long-run-supervisor-074/tests/support/boot_operations.rs','tools/long-run-supervisor-074/tests/support/authority_open_name_controls.rs','crates/xtask/tests/boot_operations_074.rs','crates/xtask/tests/authority_open_name_074.rs','crates/xtask/tests/support/boot_operations_guard.rs','crates/xtask/tests/support/authority_open_name_guard.rs','crates/xtask/tests/support/boot_operations_gate.txt','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')){@{path=$taskPath;sha256=(Hash $taskPath)}})
@{stage=$Stage;lane=$Lane;head=(& git rev-parse HEAD).Trim();recipe_sha256=(Hash $PSCommandPath);inputs=$taskInputs;steps=$taskSteps;failure=$taskFailure;server=$false;product=$false}|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
if($taskFailure){throw $taskFailure}
