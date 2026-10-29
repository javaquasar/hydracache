#requires -Version 7.2
param([Parameter(Mandatory)][ValidateSet('194','188')][string]$Lane)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskOutput=Join-Path $taskRepo "target/authority-operator-targeted-first/$Lane"
if(Test-Path -LiteralPath $taskOutput){throw 'Never replace a first isolated cohort'}
New-Item -ItemType Directory -Path $taskOutput -Force|Out-Null
. (Join-Path $taskRepo 'tools/authority-operator-order-074/selection.ps1')
[string[]]$taskSelection=Get-OperatorControlSelection -Phase candidate
if($taskSelection.Count -ne 1 -or $taskSelection[0] -cne 'model::candidate_controls::candidate_'){throw 'Wrong typed candidate filter'}
[string[]]$taskListArgs=@('--list')+$taskSelection
[string[]]$taskControlArgs=@('--nocapture','--test-threads=1')+$taskSelection
[string[]]$taskExpected=@(
    'model::candidate_controls::candidate_initial_fault_priority',
    'model::candidate_controls::candidate_publication_and_external_latch',
    'model::candidate_controls::candidate_state_priority'
)
$taskCompiler=if($Lane -ceq '194'){'+1.94.0'}else{'+1.88.0'}
$taskTarget=Join-Path $taskRepo "target/authority-operator-targeted-$Lane"
$taskExe=Join-Path $taskOutput 'argv.exe'
$taskSteps=@();$taskFailure=$null;$taskModelTestsStarted=$false
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskOutput "$Name.log"
    if(Test-Path -LiteralPath $taskLog){throw 'Never replace a first isolated log'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log="$Name.log";sha256=(Get-FileHash -LiteralPath $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Isolated correctness refused: $Lane/$Name ($taskCode)"}
}
function Assert-Argv([string]$Name,[string[]]$Expected){
    $taskObserved=@(Get-Content -LiteralPath (Join-Path $taskOutput "$Name.log")|ForEach-Object {$_|ConvertFrom-Json})
    if($taskObserved.Count -ne $Expected.Count){throw 'Native argv count differs'}
    for($taskIndex=0;$taskIndex -lt $Expected.Count;$taskIndex++){
        if($taskObserved[$taskIndex] -cne $Expected[$taskIndex]){throw 'Native argv value differs'}
    }
}
try{
    Record 'compiler' { & rustc $taskCompiler -vV }
    Record 'argv-compile' { & rustc $taskCompiler tools/authority-operator-order-074/argv.rs -o $taskExe }
    Record 'argv-list' { & $taskExe @taskListArgs }
    Assert-Argv 'argv-list' $taskListArgs
    Record 'argv-controls' { & $taskExe @taskControlArgs }
    Assert-Argv 'argv-controls' $taskControlArgs
    Record 'listing' { & cargo $taskCompiler test --manifest-path tools/authority-operator-order-074/Cargo.toml --offline --locked --release --target-dir $taskTarget --lib -- @taskListArgs }
    $taskListing=Get-Content -Raw -LiteralPath (Join-Path $taskOutput 'listing.log')
    $taskListed=@([regex]::Matches($taskListing,'(?m)^(\S+): test\r?$')|ForEach-Object {$_.Groups[1].Value})
    if($taskListed.Count -ne 3 -or -not $taskListing.Contains('3 tests, 0 benchmarks')){throw 'Refuse unexpected listing count'}
    for($taskIndex=0;$taskIndex -lt 3;$taskIndex++){
        if($taskListed[$taskIndex] -cne $taskExpected[$taskIndex]){throw 'Refuse unexpected listed test'}
    }
    $taskModelTestsStarted=$true
    Record 'controls' { & cargo $taskCompiler test --manifest-path tools/authority-operator-order-074/Cargo.toml --offline --locked --release --target-dir $taskTarget --lib -- @taskControlArgs }
    $taskControls=Get-Content -Raw -LiteralPath (Join-Path $taskOutput 'controls.log')
    if($taskControls -match 'HC074-OWNED-|HC074-OP-LEGACY|test result: FAILED' -or -not $taskControls.Contains('3 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out;')){throw 'Refuse unexpected controls scope'}
    Record 'check' { & cargo $taskCompiler check --manifest-path tools/authority-operator-order-074/Cargo.toml --offline --locked --all-targets --target-dir $taskTarget }
    Record 'clippy' { & cargo $taskCompiler clippy --manifest-path tools/authority-operator-order-074/Cargo.toml --offline --locked --all-targets --all-features --target-dir $taskTarget -- -D warnings }
}catch{$taskFailure=$_.Exception.Message}
@{schema='authority-operator-targeted-first-v1';lane=$Lane;source=(git rev-parse HEAD);selection=$taskSelection;listing_args=$taskListArgs;control_args=$taskControlArgs;expected_tests=$taskExpected;steps=$taskSteps;failure=$taskFailure;model_tests_started=$taskModelTestsStarted;model_only=$true;runtime_candidate_implemented=$false;original_owner_profiles=0;old_controls_executed=$false;server=$false;product=$false}|ConvertTo-Json -Depth 8|Out-File (Join-Path $taskOutput 'result.json') -Encoding utf8 -NoClobber
if($taskFailure){throw $taskFailure}
