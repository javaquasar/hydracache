param([ValidateSet('Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Value,[string]$Message){if(-not $Value){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){[System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $A -Depth 60 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $B -Depth 60 -Compress)))}
function WriteNew([string]$Name,$Value){
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject $Value -Depth 30)+"`n")
    $taskStream=[IO.File]::Open((Join-Path $PSScriptRoot $Name),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
$taskExpected=@{host=8;'194'=6;'188'=6}
$taskContext=Json '194-before.json'
foreach($taskLane in @('194','188','host')){
    $taskBefore=Json "$taskLane-before.json"; $taskAfter=Json "$taskLane-after.json"
    Require ((Same $taskBefore $taskAfter) -and (Same $taskContext $taskBefore)) 'Source context changed'
    foreach($taskRow in $taskBefore){Require ((Hash $taskRow.path) -ceq $taskRow.sha256) "Source changed: $($taskRow.path)"}
    $taskChecks=Json "$taskLane-checks.json"
    Require (-not $taskChecks.failure -and $taskChecks.steps.Count -eq $taskExpected.$taskLane) 'Check cohort incomplete'
    foreach($taskFlag in @('new_original_live_fixture','server','original_send','product','performance_measurement')){
        Require ($taskChecks.$taskFlag -is [bool] -and -not $taskChecks.$taskFlag) "Scope changed: $taskFlag"
    }
    foreach($taskRow in $taskChecks.steps){
        Require ($taskRow.exit -eq 0 -and (Hash (Join-Path $PSScriptRoot $taskRow.log)) -ceq $taskRow.sha256) 'Check log changed'
    }
}
foreach($taskLane in @('194','188')){
    foreach($taskCase in @(@('projection',10),@('consuming',10),@('reply',12),@('roles',38))){
        $taskText=Get-Content (Join-Path $PSScriptRoot "$taskLane-$($taskCase[0]).log") -Raw
        Require ($taskText -match "test result: ok\. $($taskCase[1]) passed; 0 failed; 0 ignored;") "Wrong count: $taskLane $($taskCase[0])"
    }
}
Require ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification changed'
$taskFinal=Json 'final-checks.json'
Require ($taskFinal.steps.Count -eq 2) 'Final documentation checks incomplete'
foreach($taskRow in $taskFinal.steps){Require ($taskRow.exit -eq 0 -and (Hash (Join-Path $PSScriptRoot $taskRow.log)) -ceq $taskRow.sha256) 'Final check log changed'}
$taskPrior='docs/testing/performance/0.74/local-runs/original-delivery-writers-audit-20261021'
Require ((Hash "$taskPrior/inventory.json") -ceq '5726fc5fcd5b9561fb7a607ab9ebb6c4052b12e2851e85af99610978d41a3ee9') 'Prior audit seal changed'
$taskOld=Get-Content "$taskPrior/inventory.json" -Raw|ConvertFrom-Json
Require ($taskOld.files.Count -eq 41) 'Prior audit member count'
foreach($taskRow in $taskOld.files){Require ((Hash "$taskPrior/$($taskRow.path)") -ceq $taskRow.sha256) 'Prior audit evidence changed'}
if($Mode -ceq 'Seal'){
    $taskReports=@('docs/testing/performance/0.74/held-unit-projection-20261021.md',
        'docs/articles/007-optimization-starts-with-measurement.md','docs/testing/performance/0.74/README.md',
        'docs/testing/performance/0.74/diagnostic-server-readiness-intermediate-plan-20261012.md')
    WriteNew 'binding.json' @{runtime_source=(& git rev-parse HEAD).Trim();
        recipe_sha256=(Hash (Join-Path $PSScriptRoot 'checks.ps1'));
        reports=@($taskReports|ForEach-Object {@{path=$_;sha256=(Hash $_)}});
        original_windows_seconds=@(300,60,10);semantic_projection=$true;materialized_unit=$false;
        original_send=$false;all_writers_proven=$false;full_sigkill_matrix=$false;performance_claim=$false}
}
$taskBinding=Json 'binding.json'
Require ($taskBinding.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'checks.ps1'))) 'Recipe changed'
Require ((Same $taskBinding.original_windows_seconds @(300,60,10)) -and $taskBinding.semantic_projection -eq $true) 'Projection/window changed'
foreach($taskFlag in @('materialized_unit','original_send','all_writers_proven','full_sigkill_matrix','performance_claim')){
    Require ($taskBinding.$taskFlag -is [bool] -and -not $taskBinding.$taskFlag) 'Admission scope changed'
}
foreach($taskRow in $taskBinding.reports){Require ((Hash $taskRow.path) -ceq $taskRow.sha256) 'Bound report changed'}
function Inventory{@{files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object {@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}}
if($Mode -ceq 'Seal'){WriteNew 'inventory.json' (Inventory)}
Require (Same (Json 'inventory.json') (Inventory)) 'Evidence inventory changed'
@{result='pass';members=(Json 'inventory.json').files.Count;inventory_sha256=(Hash (Join-Path $PSScriptRoot 'inventory.json'));runtime_source=$taskBinding.runtime_source;original_send=$false;server=$false;product=$false}|ConvertTo-Json
