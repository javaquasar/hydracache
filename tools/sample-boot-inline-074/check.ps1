#requires -Version 7.2
param([Parameter(Mandatory)][ValidateSet('reference','candidate')][string]$Phase,[Parameter(Mandatory)][ValidateSet('host','194','188')][string]$Lane)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskOutput=Join-Path $taskRepo "target/sample-boot-inline-checks/$Phase"
if(-not(Test-Path -LiteralPath $taskOutput)){New-Item -ItemType Directory -Path $taskOutput -Force|Out-Null}
if(Test-Path -LiteralPath (Join-Path $taskOutput "$Lane-result.json")){throw 'Never replace first receipt'}
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskOutput "$Lane-$Name.log"
    if(Test-Path -LiteralPath $taskLog){throw 'Never replace first log'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash -LiteralPath $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Check refused: $Phase/$Lane/$Name ($taskCode)"}
}
try{
    if($Lane -ceq 'host'){
        Record 'canaries' { & cargo test --offline --locked -p xtask --test sample_boot_inline_074 --test sample_boot_baseline_074 --test proc_document_baseline_074 --test collector_peer_capacity_074 --test collector_peer_baseline_074 --test trust_component_stack_074 --test trust_component_baseline_074 --test context_document_stack_074 --test context_document_baseline_074 --test projection_groups_074 --test pidfd_info_path_074 --test held_material_live_074 --test held_material_074 --test held_unit_projection_074 --test boot_operations_074 --test held_preparation_cost_074 }
        Record 'check' { & cargo check --offline --locked -p xtask --all-targets }
        Record 'clippy' { & cargo clippy --offline --locked -p xtask --all-targets --all-features -- -D warnings }
    }else{
        $taskCompiler=if($Lane -ceq '194'){'+1.94.0'}else{'+1.88.0'}
        $taskTarget=Join-Path $taskRepo "target/sample-boot-inline-$Lane"
        Record 'compiler' { & rustc $taskCompiler -vV }
        if($Phase -ceq 'reference'){
            Record 'controls' { & cargo $taskCompiler test --manifest-path tools/sample-boot-inline-074/Cargo.toml --offline --locked --release --target-dir $taskTarget --lib reference_cost_preflight -- --nocapture --test-threads=1 }
        }else{
            Record 'controls' { & cargo $taskCompiler test --manifest-path tools/sample-boot-inline-074/Cargo.toml --offline --locked --release --target-dir $taskTarget --lib -- --skip reference_cost_preflight --nocapture --test-threads=1 }
        }
        Record 'check' { & cargo $taskCompiler check --manifest-path tools/sample-boot-inline-074/Cargo.toml --offline --locked --all-targets --target-dir $taskTarget }
        Record 'clippy' { & cargo $taskCompiler clippy --manifest-path tools/sample-boot-inline-074/Cargo.toml --offline --locked --all-targets --all-features --target-dir $taskTarget -- -D warnings }
    }
}catch{$taskFailure=$_.Exception.Message}
@{phase=$Phase;cohort='first';lane=$Lane;steps=$taskSteps;failure=$taskFailure;platform='windows-msvc';model_only=$true;prototype_only=$true;runtime_candidate_implemented=$false;original_owner_profiles=0;ignored_tests_run=$false;server=$false;product=$false}|ConvertTo-Json -Depth 8|Out-File (Join-Path $taskOutput "$Lane-result.json") -Encoding utf8 -NoClobber
if($taskFailure){throw $taskFailure}
