param([ValidateSet('Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/fresh-kernel-authority-20261021'
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $taskPacket $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){[System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $A -Depth 90 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $B -Depth 90 -Compress)))}
$taskCapture=Json 'capture-exit.json'
Require (-not $taskCapture.failure -and $taskCapture.source -cmatch '^[0-9a-f]{40}$' -and
    ($taskCapture.steps.name -join ',') -ceq 'prior-review,windows-parent,windows-tests,linux-parent,linux-tests,derive,reconstruct' -and
    @($taskCapture.steps|Where-Object exit -ne 0).Count -eq 0) 'Capture incomplete'
Require ($taskCapture.recipe_sha256 -ceq (Hash (Join-Path $taskPacket 'method/capture_fresh.ps1'))) 'Capture recipe changed'
foreach($taskName in @('attribute_partial.py','attribute_fresh.py','test_attribute_fresh.py','capture_fresh.ps1','check_fresh.ps1','review_fresh.ps1')){
    $taskStored=(& git hash-object --no-filters -- (Join-Path $taskPacket "method/$taskName")).Trim()
    $taskExpected=(& git rev-parse "$($taskCapture.source):tools/local-profile-session-074/$taskName").Trim()
    Require ($taskStored -ceq $taskExpected) 'Frozen method changed'
    Require ((Hash (Join-Path $PSScriptRoot $taskName)) -ceq (Hash (Join-Path $taskPacket "method/$taskName"))) 'Current method differs'
}
& git diff --quiet b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d HEAD -- tools/long-run-supervisor-074 crates Cargo.lock
Require ($LASTEXITCODE -eq 0) 'Live runtime/observers changed'
Require ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification changed'
& ./docs/testing/performance/0.74/local-runs/observer-topology-cost-20261021/review.ps1 -Mode Review|Out-Null
& python -B (Join-Path $taskPacket 'method/attribute_fresh.py') --input ./docs/testing/performance/0.74/local-runs/single-session-rounds-bound-20261021 --verify --output (Join-Path $taskPacket 'facts.json')
Require ($LASTEXITCODE -eq 0) 'Fresh reconstruction failed'
$taskFacts=Json 'facts.json'
foreach($taskFlag in @('runtime_changed','observer_changed','native_gate_open','server','product')){
    Require ($taskFacts.$taskFlag -is [bool] -and -not $taskFacts.$taskFlag) 'Facts scope changed'
    Require ($taskCapture.$taskFlag -is [bool] -and -not $taskCapture.$taskFlag) 'Capture scope changed'
}
Require ($taskFacts.new_profile_invocations -eq 0 -and $taskCapture.new_profile_invocations -eq 0) 'Unexpected profile'
$taskFinal=Json 'final-check-exit.json'
Require (-not $taskFinal.failure -and ($taskFinal.steps.name -join ',') -ceq 'format,docs,registry,performance,diff' -and
    @($taskFinal.steps|Where-Object exit -ne 0).Count -eq 0 -and
    $taskFinal.recipe_sha256 -ceq (Hash (Join-Path $taskPacket 'method/check_fresh.ps1'))) 'Repository checks incomplete'
foreach($taskStep in @($taskCapture.steps)+@($taskFinal.steps)){
    Require ($taskStep.sha256 -ceq (Hash (Join-Path $taskPacket $taskStep.log))) 'Log changed'
}
foreach($taskLane in @('windows','linux')){
    foreach($taskKind in @('parent','tests')){
        $taskCount=if($taskKind -ceq 'parent'){18}else{10}
        $taskRaw=Get-Content (Join-Path $taskPacket "$taskLane-$taskKind.log") -Raw
        Require ($taskRaw.Contains("Ran $taskCount tests") -and $taskRaw.Contains("`nOK")) 'Test controls incomplete'
    }
}
function Inventory{@{files=@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object {@{path=$_.FullName.Substring($taskPacket.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}}
if($Mode -ceq 'Seal'){
    Require (-not(Test-Path (Join-Path $taskPacket 'inventory.json'))) 'Never reseal'
    Inventory|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM (Join-Path $taskPacket 'inventory.json')
}else{Require (Same (Inventory) (Json 'inventory.json')) 'Inventory changed'}
@{mode=$Mode;integrity_verified=$true;prior_packets_verified=24;new_profile_invocations=0;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
