param([ValidateSet('WriteSummary','Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
Require ((Hash (Join-Path $PSScriptRoot 'review.ps1')) -ceq '4ceb5404f3c95fe2f1c3f80edb9516cc336883de6548b89da6d9db2529092eb1') 'Frozen reviewer changed'
$taskCandidate=Json 'Candidate-194-exit.json'
$taskBaseline=Json 'Baseline-194-exit.json'
$taskFinal=Json 'final-check-exit.json'
Require (-not $taskFinal.failure -and $taskFinal.source -ceq $taskCandidate.source -and ($taskFinal.steps.name -join ',') -ceq 'format,docs,registry,performance,diff' -and @($taskFinal.steps|Where-Object exit -ne 0).Count -eq 0) 'Final checks incomplete'
Require ($taskFinal.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'final-check.ps1'))) 'Final check recipe changed'
foreach($taskStep in $taskFinal.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Final log changed'}
foreach($taskFlag in @('server','product')){Require ($taskFinal.$taskFlag -is [bool] -and -not $taskFinal.$taskFlag) 'Final scope changed'}
foreach($taskLane in @('xtask','repository')){
    $taskCheck=Json "BaselineRepair-check-$taskLane-exit.json"
    $taskCount=if($taskLane -ceq 'xtask'){3}else{4}
    Require (-not $taskCheck.failure -and $taskCheck.steps.Count -eq $taskCount -and @($taskCheck.steps|Where-Object exit -ne 0).Count -eq 0) 'Baseline ancillary checks incomplete'
    Require ($taskCheck.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'check-repair.ps1'))) 'Baseline check recipe changed'
    foreach($taskStep in $taskCheck.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Baseline log changed'}
}
$taskEnvironment=Json 'environment.json'
Require ($taskEnvironment.source -ceq $taskBaseline.source -and $taskEnvironment.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'environment.ps1'))) 'Environment source/recipe changed'
Require (($taskEnvironment.commands.name -join ',') -ceq 'kernel,cpu,online,distribution,rust194,rust188' -and @($taskEnvironment.commands|Where-Object exit -ne 0).Count -eq 0) 'Environment incomplete'
Require ($taskEnvironment.commands[4].output.Contains('rustc 1.94.0') -and $taskEnvironment.commands[5].output.Contains('rustc 1.88.0')) 'Compiler identity differs'
foreach($taskFlag in @('affinity_pinned','services_changed','clocks_changed','server','product')){Require ($taskEnvironment.$taskFlag -is [bool] -and -not $taskEnvironment.$taskFlag) 'Environment scope changed'}
$taskAudit=Json 'review-audit.json'
Require (($taskAudit.mutations.mutation -join ',') -ceq 'requests,scope,difference,order' -and @($taskAudit.mutations|Where-Object refused -ne $true).Count -eq 0) 'Reviewer mutation controls incomplete'
& (Join-Path $PSScriptRoot 'review.ps1') -Mode $Mode
