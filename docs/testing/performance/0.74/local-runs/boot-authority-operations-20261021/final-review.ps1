param([ValidateSet('Seal','Review','Audit')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){[System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $A -Depth 95 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $B -Depth 95 -Compress)))}
function Scope($Facts){
    foreach($taskFlag in @('native_gate_open','server','product','cpu_speedup_claim','overhead_subtracted','exclusive_cross_observer_cpu_known')){
        Require ($Facts.$taskFlag -is [bool] -and -not $Facts.$taskFlag) "Scope changed: $taskFlag"
    }
    foreach($taskLane in @('194','188')){
        Require ($Facts.comparison.$taskLane.allocation_reduction_verified -is [bool] -and $Facts.comparison.$taskLane.allocation_reduction_verified) 'Allocation result not verified'
        Require ($Facts.comparison.$taskLane.cpu_nonregression_proven -is [bool] -and -not $Facts.comparison.$taskLane.cpu_nonregression_proven) 'CPU claim changed'
    }
}
if($Mode -ceq 'Audit'){
    Scope (Json 'summary.json')
    $taskMutations=@()
    foreach($taskName in @('numeric-gate','speedup','overhead','numeric-allocation','cpu-nonregression')){
        $taskFacts=Json 'summary.json'
        switch($taskName){
            'numeric-gate'{$taskFacts.native_gate_open=0}
            'speedup'{$taskFacts.cpu_speedup_claim=$true}
            'overhead'{$taskFacts.overhead_subtracted=$true}
            'numeric-allocation'{$taskFacts.comparison.'194'.allocation_reduction_verified=1}
            'cpu-nonregression'{$taskFacts.comparison.'188'.cpu_nonregression_proven=$true}
        }
        $taskRejected=$false;try{Scope $taskFacts}catch{$taskRejected=$true}
        Require $taskRejected "Mutation accepted $taskName"
        $taskMutations+=@{mutation=$taskName;refused=$true}
    }
    @{mutations=$taskMutations;server=$false;product=$false;native_gate_open=$false}|ConvertTo-Json -Depth 6
    exit
}
$taskBaseline=Json 'Baseline-metadata.json';$taskCandidate=Json 'Candidate-metadata.json'
Require ($taskBaseline.source -ceq 'a0cb7fe3ebf199d01e067adb389e3fb06557c152' -and $taskCandidate.source -cmatch '^[0-9a-f]{40}$') 'Source identity changed'
function GitText([string]$Source,[string]$Path){
    $taskLines=@(& git show "${Source}:$Path")
    Require ($LASTEXITCODE -eq 0) 'Missing committed source'
    ($taskLines -join "`n")+"`n"
}
$taskCore='tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs'
$taskBefore=GitText $taskBaseline.source $taskCore
$taskAfter=GitText $taskCandidate.source $taskCore
$taskOld='if !directory && name.len() < short_name.len()'
$taskNew='if name.len() < short_name.len()'
Require (($taskBefore.Split($taskOld,[StringSplitOptions]::None).Count-1) -eq 1 -and $taskBefore.Replace($taskOld,$taskNew) -ceq $taskAfter) 'Candidate is not the exact directory encoding change'
$taskChanges=@(& git diff --name-only $taskBaseline.source $taskCandidate.source)
Require (($taskChanges -join ',') -ceq 'tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs,tools/long-run-supervisor-074/tests/support/authority_open_name_controls.rs') 'Candidate contains another hypothesis'
& git diff --quiet $taskCandidate.source HEAD -- tools/long-run-supervisor-074 crates Cargo.lock docs/testing/gated-test-registry.toml
Require ($LASTEXITCODE -eq 0) 'Measured candidate source changed'
Require ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification changed'
Require ((Hash (Join-Path $PSScriptRoot 'predecessor-review.log')) -ceq '7d44df541b361de95634d585ae28ceed22b2114231fbdfcede827c2d12d2e1fb') 'Predecessor proof changed'
$taskMethodPaths=@('tools/local-profile-session-074/check_boot_ops.ps1','tools/local-profile-session-074/run_boot_ops.py','tools/local-profile-session-074/review_boot_ops.py','tools/local-profile-session-074/test_boot_ops.py','tools/local-profile-session-074/launch_boot_ops.ps1','tools/long-run-supervisor-074/tests/support/boot_operations.rs','crates/xtask/tests/boot_operations_074.rs','crates/xtask/tests/support/boot_operations_guard.rs','docs/testing/performance/0.74/boot-authority-operations-plan-20261021.md')
foreach($taskStage in @('Baseline','Candidate')){
    $taskMetadata=Json "$taskStage-metadata.json";$taskLaunch=Json "$taskStage-launch.json"
    Require ((Same $taskMetadata (Json "$taskStage-postflight.json")) -and $taskMetadata.uid -gt 0 -and -not $taskMetadata.dirty -and $taskLaunch.exit -eq 0 -and $taskLaunch.source -ceq $taskMetadata.source) 'Session failed or drifted'
    foreach($taskPath in $taskMethodPaths){
        $taskCopy=Join-Path $PSScriptRoot "method-$taskStage/$([IO.Path]::GetFileName($taskPath))"
        Require ((& git hash-object --no-filters -- $taskCopy).Trim() -ceq (& git rev-parse "$($taskMetadata.source):$taskPath").Trim()) 'Frozen method changed'
        Require ((Hash $taskCopy) -ceq (Hash $taskPath)) 'Measured method drift'
    }
    foreach($taskLane in @('194','188')){
        foreach($taskSpec in @(@('boot','HC074-BOOT-OPS-ORIGIN ','boot'),@('phases','HC074-AUTHORITY-CPU-ORIGIN ','boot_id'),@('leaf','HC074-LEAF-CPU-ORIGIN ','boot_id'),@('names','HC074-OPEN-NAME-ORIGIN ','boot_id'),@('owners','HC074-AUTHORITY-OWNERS-ORIGIN ','boot_id'),@('full','HC074-VALIDATION-LEAF-ORIGIN ','boot_id'))){
            $taskRows=@(Get-Content -LiteralPath (Join-Path $PSScriptRoot "$taskStage-$taskLane-$($taskSpec[0]).log")|Where-Object {$_.Contains($taskSpec[1])}|ForEach-Object {($_.Substring($_.IndexOf($taskSpec[1])+$taskSpec[1].Length))|ConvertFrom-Json})
            Require ($taskRows.Count -eq 1 -and $taskRows[0].($taskSpec[2]) -ceq $taskMetadata.boot -and $taskRows[0].uid -eq $taskMetadata.uid) 'Child origin drift'
        }
    }
}
foreach($taskReceiptPath in @(Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*-check-*-exit.json')){
    $taskReceipt=Get-Content -LiteralPath $taskReceiptPath.FullName -Raw|ConvertFrom-Json
    $taskFailed=@($taskReceipt.steps|Where-Object exit -ne 0)
    if($taskReceiptPath.Name -ceq 'Baseline-check-xtask-exit.json'){
        Require ($taskReceipt.failure -ceq 'Failed tests' -and $taskFailed.Count -eq 1 -and $taskFailed[0].exit -eq 101) 'Authoring refusal erased'
    }else{Require (-not $taskReceipt.failure -and $taskFailed.Count -eq 0) "Preflight failed: $($taskReceiptPath.Name)"}
    foreach($taskStep in $taskReceipt.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Preflight log changed'}
}
$taskTestBefore=Json 'directory-test-before.json'
Require ($taskTestBefore.exit -eq 0 -and $taskTestBefore.head -ceq $taskBaseline.source -and
    $taskTestBefore.source_sha256 -ceq (Hash 'tools/long-run-supervisor-074/tests/support/authority_open_name_controls.rs') -and
    (Get-Content (Join-Path $PSScriptRoot 'directory-test-before.log') -Raw).Contains('1 passed; 0 failed')) 'Test was not demonstrated before implementation'
& python -B tools/local-profile-session-074/review_boot_ops.py --packet $PSScriptRoot --baseline-only --verify --output (Join-Path $PSScriptRoot 'baseline-facts.json')
Require ($LASTEXITCODE -eq 0) 'Baseline reconstruction failed'
& python -B tools/local-profile-session-074/review_boot_ops.py --packet $PSScriptRoot --verify --output (Join-Path $PSScriptRoot 'summary.json')
Require ($LASTEXITCODE -eq 0) 'Candidate reconstruction failed'
Scope (Json 'summary.json')
$taskAudit=Json 'scope-audit.json'
Require ($taskAudit.mutations.Count -eq 5 -and @($taskAudit.mutations|Where-Object refused -ne $true).Count -eq 0) 'Scope audit incomplete'
$taskPins=@{
    'fresh-kernel-authority-20261021'='f3c8ccba7495ed3db5bb8072fd4918856f592b251409ebadcc6a76a3389a9ef1'
    'observer-topology-cost-20261021'='02689a6f57309a8f56cfb3efabbf7df688095f284bfa9daf0d86ec3fcf7fd3c7'
    'partial-preparation-attribution-20261021'='09f001dd7a2226a9974472efda807d8c5f515d7804103c40bf2bbd5abdfb41ed'
    'single-session-rounds-bound-20261021'='91a6e589235f2552ae9335680f47d4a20a418efeefcc3bc9fd2a05ba96f12afe'
    'validation-round-residual-20261021'='15c86f32b3f56c0c86ac526ad72b54b32aa3cf75625543cea3dd3d9aee9c2807'
    'authority-open-name-20261021'='7d5a88964d1df7498aced1d5ce81fbd8b4ad16fe693645bb5044503717a67ce8'
    'authority-leaf-cpu-20261021'='4a4c317f7e9cc878b57aa205324d865577cd17772bff3dea0d7e513c01b2d805'
    'authority-cpu-20261021'='5169a17fdc585ac15be3447a802be5e8b289346ee59e2eb420fcca8b31828d4e'
    'authority-owners-20261021'='9c561339ebcbb08f379121996306bf0e11a7264d5c5ac5dff7abc678d289fe17'
    'authority-fd-path-20261021'='ff4d9e59bffefb40eb1712f253eaebce701a34b832a5417211ec88b144fa2440'
    'authority-breakdown-20261021'='c758e549722903dd77d20e20da00347a3a950964ee8bb682ad43a10a6782003d'
    'authority-expected-name-20261021'='4249e3cd970a5159951a22b5e40762b4aaa97eee003118d664bb67110376464f'
    'authority-name-capacity-20261021'='2d94764f08a89ed50e4e1d8ce47906382fb6be6c796bd585e7d0fbdc5f628362'
    'authority-inventory-20261021'='101b29bf56b767da04c86bcdd7d14fa9e488de3c319f11e56d718b614effdbef'
    'authority-leaf-name-20261020'='384430c3e8a18f08e521ffb3d7ca4aba8428001f8011347265577a7a5c139a12'
    'authority-context-20261020'='25796efbc05cbcca3a7c646d70624774979c84fb16c4beb84746a4c0cc70a01f'
    'authority-key-hex-20261020'='6d7cadf2d084320d4d68c5fe73a44e6fe964003bb8e6e53d6b8c1af0905413f0'
    'validation-leaf-requests-20261020'='9861517676b8ed06b49ebe9d3ebd3c9d8d4835c74fd81c4ff6703edccd9f9ad3'
    'validation-round-cost-20261020'='b089573e0dc544df769c07b64be41562f107cdc1add64b1cd0c9512df2d882d6'
    'starting-effects-cost-20261020'='69c7057e4994abea20e148a9179fe8229b6f6725ec7d1002e6d80484931a2d3c'
    'remaining-preparation-cost-20261020'='2839ea78850f17f8070807b8853c0726133200311ccfd37f4f811b95ff796fbc'
    'custody-preparation-cost-20261020'='3194244cb4e45c6f4f611b946afaa6a6a8b3aaf83179457c260d0a9e25f93864'
    'held-native-recheck-20261020'='2a7be06fe6f93e3e8761b29afebaf89ac6266d7c405b457ef8ea26d60259bd63'
    'held-composition-recheck-20261020'='0bfa7cef4d3ebd69c9e751fe6e4304a8bade3133122dcc48249c9d1396bfe1c0'
    'pid-path-cost-20261020'='982d9598426830f6b3e44f9a701220b5778ceb3e6b568922d3b2ee405a0e6b05'
}
foreach($taskName in $taskPins.Keys){
    $taskPrior=Join-Path (Split-Path $PSScriptRoot) $taskName
    Require ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq $taskPins[$taskName]) "Prior pin changed $taskName"
    $taskInventory=Get-Content -LiteralPath (Join-Path $taskPrior 'inventory.json') -Raw|ConvertFrom-Json
    foreach($taskFile in $taskInventory.files){
        $taskParts=$taskFile.path.Replace([char]92,[char]47).Split([char]47)
        Require (-not [IO.Path]::IsPathRooted($taskFile.path) -and -not($taskParts -contains '..')) 'Unsafe prior member'
        $taskFilePath=Join-Path $taskPrior $taskFile.path
        Require ((Get-Item -LiteralPath $taskFilePath).Length -eq $taskFile.bytes -and (Hash $taskFilePath) -ceq $taskFile.sha256) "Prior bytes changed $taskName"
    }
    Require (@(Get-ChildItem -LiteralPath $taskPrior -Recurse -File).Count -eq $taskInventory.files.Count+1) "Prior membership changed $taskName"
}
function Inventory{@{files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object {@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}}
if($Mode -ceq 'Seal'){
    Require (-not(Test-Path (Join-Path $PSScriptRoot 'inventory.json'))) 'Never reseal'
    Inventory|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'inventory.json')
}else{Require (Same (Inventory) (Json 'inventory.json')) 'Inventory changed'}
@{mode=$Mode;integrity_verified=$true;prior_packets_verified=25;allocation_reduction_verified=$true;cpu_speedup_claim=$false;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
