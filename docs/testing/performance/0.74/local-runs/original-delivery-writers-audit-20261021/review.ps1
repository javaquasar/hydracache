param([ValidateSet('Seal','Review','Audit')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Value,[string]$Message){if(-not $Value){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){[System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $A -Depth 60 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $B -Depth 60 -Compress)))}
function Scope($Facts){
    foreach($taskFlag in @('runtime_changed','live_predicates_changed','original_delivery_proven','all_production_writers_proven','full_sigkill_matrix_ready','durable_original_completion_proven','production_enrollment_proven','performance_claim','root_execution','original_send','native_gate_open','server','product')){
        Require ($Facts.$taskFlag -is [bool] -and -not $Facts.$taskFlag) "Scope changed $taskFlag"
    }
    foreach($taskFlag in @('reply_flag_compatibility_implemented','four_cell_coordinator_model_implemented')){
        Require ($Facts.$taskFlag -is [bool] -and $Facts.$taskFlag) "Existing implementation erased $taskFlag"
    }
    Require ((Same $Facts.original_windows_seconds @(300,60,10)) -and $Facts.new_original_fixture_invocations -eq 0 -and $Facts.selected_existing_controls_per_compiler -eq 32 -and $Facts.blockers.Count -eq 6) 'Audit scope/count/window changed'
}
if($Mode -ceq 'Audit'){
    Scope (Json 'conclusions.json')
    $taskRows=@()
    foreach($taskName in @('numeric-native','delivery','writers','matrix','completion','renewal','erase-compatibility','erase-model')){
        $taskFacts=Json 'conclusions.json'
        switch($taskName){
            'numeric-native'{$taskFacts.native_gate_open=0}
            'delivery'{$taskFacts.original_delivery_proven=$true}
            'writers'{$taskFacts.all_production_writers_proven=$true}
            'matrix'{$taskFacts.full_sigkill_matrix_ready=$true}
            'completion'{$taskFacts.durable_original_completion_proven=$true}
            'renewal'{$taskFacts.original_windows_seconds[2]=11}
            'erase-compatibility'{$taskFacts.reply_flag_compatibility_implemented=$false}
            'erase-model'{$taskFacts.four_cell_coordinator_model_implemented=$false}
        }
        $taskRejected=$false;try{Scope $taskFacts}catch{$taskRejected=$true}
        Require $taskRejected "Audit mutation accepted $taskName"
        $taskRows+=@{mutation=$taskName;refused=$true}
    }
    @{mutations=$taskRows;server=$false;product=$false}|ConvertTo-Json -Depth 8
    exit
}
$taskSource='3107fe33360f97a17ae336f2da6714832924dd68'
$taskBefore=Json 'context-before.json';$taskAfter=Json 'context-after.json'
Require ((Same $taskBefore $taskAfter) -and $taskBefore.source -ceq $taskSource -and $taskBefore.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -and $taskBefore.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'checks.ps1'))) 'Audit source/context changed'
foreach($taskRow in $taskBefore.source_files){Require ((Hash $taskRow.path) -ceq $taskRow.sha256) 'Audited source changed'}
Require ((Hash 'tools/long-run-supervisor-074/tests/support/consuming_held_live.rs') -ceq $taskBefore.fixture_sha256) 'Original fixture changed'
& git diff --quiet 3d3a2180c6b54ea9156ae2bac609f36966359908 HEAD -- tools/long-run-supervisor-074 crates Cargo.lock docs/testing/gated-test-registry.toml
Require ($LASTEXITCODE -eq 0) 'Committed runtime changed'
& git diff --quiet HEAD -- tools/long-run-supervisor-074 crates Cargo.lock docs/testing/gated-test-registry.toml
Require ($LASTEXITCODE -eq 0) 'Working runtime changed'
$taskSupplement=Json 'source-supplement.json'
foreach($taskRow in $taskSupplement.files){Require ((Hash $taskRow.path) -ceq $taskRow.sha256) 'Supplemental audit source changed'}
Require ($taskSupplement.source -ceq $taskSource -and (Hash 'docs/testing/performance/0.74/original-delivery-writers-prerequisites-audit-20261021.md') -ceq $taskSupplement.report_sha256) 'Audit report changed'
$taskChecks=Json 'checks.json'
Require ($taskChecks.source -ceq $taskSource -and -not $taskChecks.failure -and $taskChecks.steps.Count -eq 23 -and @($taskChecks.steps|Where-Object exit -ne 0).Count -eq 0 -and $taskChecks.new_original_fixture_invocations -eq 0 -and $taskChecks.root_execution -is [bool] -and -not $taskChecks.root_execution) 'Selected audit controls incomplete'
foreach($taskRow in $taskChecks.steps){Require ((Hash (Join-Path $PSScriptRoot $taskRow.log)) -ceq $taskRow.sha256) 'Audit control log changed'}
foreach($taskLane in @('194','188')){
    $taskTotal=0
    foreach($taskSelector in $taskChecks.selectors){
        $taskText=Get-Content (Join-Path $PSScriptRoot "$taskLane-$($taskSelector[0]).log") -Raw
        Require ($taskText -match "test result: ok\. $($taskSelector[2]) passed; 0 failed; 0 ignored;") 'Selected result changed'
        $taskTotal+=$taskSelector[2]
    }
    Require ($taskTotal -eq 32) 'Selected tests total changed'
}
Scope (Json 'conclusions.json')
$taskScope=Json 'scope-audit.json'
Require ($taskScope.mutations.Count -eq 8 -and @($taskScope.mutations|Where-Object refused -ne $true).Count -eq 0) 'Audit scope controls incomplete'
$taskRepository=Json 'repository-checks.json'
Require (-not $taskRepository.failure -and $taskRepository.steps.Count -eq 5 -and @($taskRepository.steps|Where-Object exit -ne 0).Count -eq 0) 'Repository checks incomplete'
foreach($taskRow in $taskRepository.steps){Require ((Hash (Join-Path $PSScriptRoot $taskRow.log)) -ceq $taskRow.sha256) 'Repository check log changed'}
& pwsh -NoProfile -File docs/testing/performance/0.74/local-runs/held-original-faults-20261021/final-review.ps1 -Mode Review
Require ($LASTEXITCODE -eq 0) 'Preceding evidence chain refused'
Require ((Hash 'docs/testing/performance/0.74/local-runs/held-original-faults-20261021/inventory.json') -ceq '80e78ab80251b43ea405eea99234595b226497dfe5c2e4367581c8db8219d96b') 'Preceding seal changed'
function Inventory{@{files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object {@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}}
$taskSeal=Join-Path $PSScriptRoot 'inventory.json'
if($Mode -ceq 'Seal'){
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject (Inventory) -Depth 8)+"`n")
    $taskStream=[IO.File]::Open($taskSeal,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
$taskInventory=Json 'inventory.json'
foreach($taskRow in $taskInventory.files){
    $taskParts=$taskRow.path.Replace([char]92,[char]47).Split([char]47)
    Require (-not [IO.Path]::IsPathRooted($taskRow.path) -and -not($taskParts -contains '..')) 'Unsafe audit evidence member'
    $taskPath=Join-Path $PSScriptRoot $taskRow.path
    Require ((Get-Item -LiteralPath $taskPath).Length -eq $taskRow.bytes -and (Hash $taskPath) -ceq $taskRow.sha256) 'Audit evidence changed'
}
Require (@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File).Count -eq $taskInventory.files.Count+1) 'Audit membership changed'
@{mode=$Mode;members=$taskInventory.files.Count;inventory_sha256=(Hash $taskSeal);prior_packets_verified=28;controls_per_compiler=32;new_original_fixture_invocations=0;full_sigkill_matrix_ready=$false;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
