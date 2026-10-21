$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskSteps=@()
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $PSScriptRoot "check-$Name.log"
    if(Test-Path $taskLog){throw 'Never overwrite check evidence'}
    & $Call *> $taskLog
    $taskExit=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskExit;log=[IO.Path]::GetFileName($taskLog);sha256=(Hash $taskLog)}
    if($taskExit -ne 0){throw "Check refused $Name"}
}
$taskFailure=$null
try{
    Record 'windows-runner' { & python -B tools/local-profile-session-074/test_runner.py }
    Record 'windows-impact' { & python -B tools/local-profile-session-074/test_preparation_impact.py }
    Record 'windows-age-audit' { & python -B tools/local-profile-session-074/audit_preparation_impact.py --test }
    foreach($taskSpec in @(@('runner','test_runner.py'),@('impact','test_preparation_impact.py'),@('age-audit','audit_preparation_impact.py'))){
        $taskArguments=if($taskSpec[0] -ceq 'age-audit'){@('--test')}else{@()}
        Record "linux-$($taskSpec[0])" { & wsl.exe -d Ubuntu -u artur -- /usr/bin/python3 -B "/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/local-profile-session-074/$($taskSpec[1])" @taskArguments }
    }
    Record 'source-canaries' { & cargo test --locked -p xtask --test boot_operations_074 --test authority_open_name_074 -- --test-threads=4 }
    Record 'format' { & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 /home/artur/.cargo/bin/cargo +1.94.0 fmt --all -- --check }
    Record 'docs' { & cargo xtask doc-check }
    Record 'registry' { & cargo xtask gated-test-check }
    Record 'performance' { & cargo xtask performance-contract-check --release 0.74 }
    Record 'diff' { & git diff --check }
}catch{$taskFailure=$_.Exception.Message}
$taskReceipt=Join-Path $PSScriptRoot 'checks.json'
if(Test-Path $taskReceipt){throw 'Never overwrite check receipt'}
@{source=(& git rev-parse HEAD).Trim();steps=$taskSteps;failure=$taskFailure;server=$false;product=$false;native_gate_open=$false}|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM $taskReceipt
if($taskFailure){throw $taskFailure}
