#requires -Version 7.2
param([ValidateSet('host','194','188')][string]$Lane,[ValidateSet('initial','corrected')][string]$Cohort='initial')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskOutput=Join-Path $taskRepo 'target/context-document-baseline-checks'
if($Cohort -ceq 'corrected'){$taskOutput=Join-Path $taskOutput 'corrected'}
if(-not(Test-Path $taskOutput)){New-Item -ItemType Directory -Path $taskOutput|Out-Null}
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskOutput "$Lane-$Name.log"
    if(Test-Path $taskLog){throw 'Never replace baseline check evidence'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash -LiteralPath $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Baseline check refused: $Lane/$Name ($taskCode)"}
}
try{
    if($Lane -ceq 'host'){
        Record 'canaries' { & cargo test --offline --locked -p xtask --test context_document_baseline_074 --test projection_groups_074 --test pidfd_info_path_074 --test held_material_live_074 --test held_material_074 --test held_unit_projection_074 --test boot_operations_074 --test held_preparation_cost_074 }
        Record 'check' { & cargo check --offline --locked -p xtask -p hydracache-long-run-supervisor-074 --all-targets }
        Record 'clippy' { & cargo clippy --offline --locked -p xtask -p hydracache-long-run-supervisor-074 --all-targets --all-features -- -D warnings }
    }else{
        $taskCompiler=if($Lane -ceq '194'){'+1.94.0'}else{'+1.88.0'}
        $taskTarget="CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Lane-check"
        foreach($taskFilter in @('context_document_buffer_baseline','fixed_leaf_alloc_counter')){
            Record $taskFilter { & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 $taskTarget /home/artur/.cargo/bin/cargo $taskCompiler test --offline --locked --release --features fixed-leaf-alloc-probe -p hydracache-long-run-supervisor-074 --lib $taskFilter -- --nocapture --test-threads=1 }
        }
        Record 'check' { & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 $taskTarget /home/artur/.cargo/bin/cargo $taskCompiler check --offline --locked -p hydracache-long-run-supervisor-074 --all-targets }
        Record 'clippy' { & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 $taskTarget /home/artur/.cargo/bin/cargo $taskCompiler clippy --offline --locked -p hydracache-long-run-supervisor-074 --all-targets --all-features -- -D warnings }
    }
}catch{$taskFailure=$_.Exception.Message}
@{lane=$Lane;steps=$taskSteps;failure=$taskFailure;runtime_candidate_implemented=$false;ignored_tests_run=$false;original_owner_profiles=0;server=$false;product=$false}|ConvertTo-Json -Depth 8|Out-File (Join-Path $taskOutput "$Lane-result.json") -Encoding utf8 -NoClobber
if($taskFailure){throw $taskFailure}
