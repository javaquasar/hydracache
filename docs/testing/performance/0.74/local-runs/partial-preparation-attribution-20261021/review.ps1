param([ValidateSet('Seal','Review','Audit')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){[System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $A -Depth 90 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $B -Depth 90 -Compress)))}
function Scope($Facts){
    foreach($taskFlag in @('runtime_changed','observer_changed','complete_profile_cohort','native_gate_open','server','product')){
        Require ($Facts.$taskFlag -is [bool] -and -not $Facts.$taskFlag) "Boolean scope changed: $taskFlag"
    }
    Require (($Facts.new_profile_invocations -is [long] -or $Facts.new_profile_invocations -is [int]) -and $Facts.new_profile_invocations -eq 0) 'New profile count changed'
    foreach($taskFlag in @('overhead_subtracted','compiler_comparison','timing_speedup_claim','kernel_outside_is_cpu','phase_sum_is_issued_age','includes_child_cpu')){
        Require ($Facts.interpretation.$taskFlag -is [bool] -and -not $Facts.interpretation.$taskFlag) "Interpretation changed: $taskFlag"
    }
}
if($Mode -ceq 'Audit'){
    Scope (Json 'facts.json')
    $taskMutations=@()
    foreach($taskName in @('numeric-gate','completion','new-profile','runtime','overhead')){
        $taskChanged=Json 'facts.json'
        switch($taskName){
            'numeric-gate'{$taskChanged.native_gate_open=0}
            'completion'{$taskChanged.complete_profile_cohort=$true}
            'new-profile'{$taskChanged.new_profile_invocations=1}
            'runtime'{$taskChanged.runtime_changed=$true}
            'overhead'{$taskChanged.interpretation.overhead_subtracted=$true}
        }
        $taskRejected=$false;try{Scope $taskChanged}catch{$taskRejected=$true}
        Require $taskRejected "Accepted scope mutation $taskName"
        $taskMutations+=@{mutation=$taskName;refused=$true}
    }
    @{mutations=$taskMutations;new_profile_invocations=0;server=$false;product=$false}|ConvertTo-Json -Depth 6
    exit
}
$taskCapture=Json 'capture-exit.json'
Require (-not $taskCapture.failure -and $taskCapture.source -cmatch '^[0-9a-f]{40}$' -and $taskCapture.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'capture.ps1')) -and ($taskCapture.steps.name -join ',') -ceq 'prior-review,windows-tests,linux-tests,derive,reconstruct' -and @($taskCapture.steps|Where-Object exit -ne 0).Count -eq 0) 'Capture incomplete'
foreach($taskStep in $taskCapture.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Capture log changed'}
foreach($taskFlag in @('new_profile_invocations','runtime_changed','observer_changed','native_gate_open','server','product')){Require (-not $taskCapture.$taskFlag) 'Scope changed'}
& git diff --quiet b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d $taskCapture.source -- tools/long-run-supervisor-074 crates Cargo.lock
Require ($LASTEXITCODE -eq 0) 'Rust bytes changed'
foreach($taskName in @('attribute_partial.py','test_attribute_partial.py')){
    $taskStored=(& git hash-object --no-filters -- (Join-Path $PSScriptRoot "method/$taskName")).Trim()
    $taskExpected=(& git rev-parse "$($taskCapture.source):tools/local-profile-session-074/$taskName").Trim()
    Require ($taskStored -ceq $taskExpected) 'Frozen analyzer method changed'
}
& ./tools/local-profile-session-074/review.ps1 -Mode Review|Out-Null
& python -B (Join-Path $PSScriptRoot 'method/attribute_partial.py') --input (Join-Path (Split-Path $PSScriptRoot) 'single-session-rounds-bound-20261021') --verify --output (Join-Path $PSScriptRoot 'facts.json')
Require ($LASTEXITCODE -eq 0) 'Offline reconstruction failed'
$taskFacts=Json 'facts.json'
Scope $taskFacts
Require ($taskFacts.analyzer_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'method/attribute_partial.py'))) 'Analyzer identity changed'
foreach($taskLane in @('windows','linux')){
    $taskRaw=Get-Content (Join-Path $PSScriptRoot "$taskLane-tests.log") -Raw
    Require ($taskRaw.Contains('Ran 18 tests') -and $taskRaw.Contains("`nOK")) 'Offline controls incomplete'
}
$taskFinal=Json 'final-check-exit.json'
Require (-not $taskFinal.failure -and ($taskFinal.steps.name -join ',') -ceq 'format,docs,registry,performance,diff' -and @($taskFinal.steps|Where-Object exit -ne 0).Count -eq 0 -and $taskFinal.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'final-check.ps1'))) 'Repository checks incomplete'
foreach($taskStep in $taskFinal.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Repository log changed'}
$taskAudit=Json 'scope-audit.json'
Require (($taskAudit.mutations.mutation -join ',') -ceq 'numeric-gate,completion,new-profile,runtime,overhead' -and @($taskAudit.mutations|Where-Object refused -ne $true).Count -eq 0) 'Scope falsifying controls incomplete'
function Inventory{@{files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object {@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}}
if($Mode -ceq 'Seal'){
    Require (-not(Test-Path (Join-Path $PSScriptRoot 'inventory.json'))) 'Never reseal'
    Inventory|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'inventory.json')
}else{Require (Same (Inventory) (Json 'inventory.json')) 'Inventory changed'}
@{mode=$Mode;integrity_verified=$true;prior_packets_verified=22;new_profile_invocations=0;complete_profile_cohort=$false;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
