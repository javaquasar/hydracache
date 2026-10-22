#requires -Version 7.2
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskOutput=Join-Path $taskRepo 'target/authority-operator-argv-first'
if(Test-Path -LiteralPath $taskOutput){throw 'Never replace first native argument control'}
New-Item -ItemType Directory -Path $taskOutput|Out-Null
. (Join-Path $PSScriptRoot 'selection.ps1')
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskOutput "$Name.log"
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash -LiteralPath $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Native argv control failed: $Name"}
}
try {
    $taskExe=Join-Path $taskOutput 'argv.exe'
    Record 'compile' { & rustc +1.94.0 tools/authority-operator-order-074/argv.rs -o $taskExe }
    $taskBroken=if($false){@('one','two')}else{@('model::candidate_controls::candidate_')}
    Record 'broken-candidate' { & $taskExe --nocapture --test-threads=1 @taskBroken }
    [string[]]$taskReference=Get-OperatorControlSelection -Phase reference
    Record 'fixed-reference' { & $taskExe --nocapture --test-threads=1 @taskReference }
    [string[]]$taskCandidate=Get-OperatorControlSelection -Phase candidate
    Record 'fixed-candidate' { & $taskExe --nocapture --test-threads=1 @taskCandidate }
    $taskObserved=@(Get-Content (Join-Path $taskOutput 'fixed-candidate.log')|ForEach-Object {$_|ConvertFrom-Json})
    if($taskObserved.Count -ne 3 -or $taskObserved[2] -cne 'model::candidate_controls::candidate_'){throw 'Candidate native argv mismatch'}
    $taskObservedReference=@(Get-Content (Join-Path $taskOutput 'fixed-reference.log')|ForEach-Object {$_|ConvertFrom-Json})
    if($taskObservedReference.Count -ne 4 -or $taskObservedReference[2] -cne 'model::operator_controls::legacy_counterexamples' -or $taskObservedReference[3] -cne '--exact'){throw 'Reference native argv mismatch'}
}catch{$taskFailure=$_.Exception.Message}
@{steps=$taskSteps;failure=$taskFailure;native_argv_only=$true;model_image_executed=$false;old_controls_executed=$false;server=$false;product=$false}|ConvertTo-Json -Depth 8|Out-File (Join-Path $taskOutput 'result.json') -Encoding utf8 -NoClobber
if($taskFailure){throw $taskFailure}
