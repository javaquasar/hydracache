#requires -Version 7.2
param([ValidateSet('194','188')][string]$Lane)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskOutput=Join-Path $taskRepo 'target/projection-groups-controls'
if(-not(Test-Path $taskOutput)){New-Item -ItemType Directory -Path $taskOutput -Force|Out-Null}
$taskCompiler=if($Lane -ceq '194'){'+1.94.0'}else{'+1.88.0'}
$taskTarget="CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Lane-check"
$taskSteps=@()
foreach($taskFilter in @('projection_prefix_differential','projection_fields_differential','kernel_refusal_capture_projection','kernel_admission_seeded_status')){
    $taskLog=Join-Path $taskOutput "$Lane-$taskFilter.log"
    if(Test-Path $taskLog){throw 'Never replace control evidence'}
    & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 $taskTarget /home/artur/.cargo/bin/cargo $taskCompiler test --offline --locked --release -p hydracache-long-run-supervisor-074 --lib $taskFilter -- --nocapture --test-threads=1 *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    if($taskCode -ne 0){throw "Control failed: $Lane/$taskFilter ($taskCode)"}
    if((Get-Content $taskLog -Raw) -notmatch 'test result: ok\. 1 passed; 0 failed; 0 ignored;'){throw 'Expected one ordinary pure test'}
    $taskSteps+=@{filter=$taskFilter;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash -LiteralPath $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
}
@{lane=$Lane;steps=$taskSteps;original_owner_profiles=0;server=$false;product=$false}|ConvertTo-Json -Depth 8|Out-File (Join-Path $taskOutput "$Lane-result.json") -Encoding utf8 -NoClobber
