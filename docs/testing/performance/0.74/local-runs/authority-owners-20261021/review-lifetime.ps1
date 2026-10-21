param([ValidateSet('WriteSummary','Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Text-Hash([string]$Text){[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){
    $taskA=[System.Text.Json.Nodes.JsonNode]::Parse(($A|ConvertTo-Json -Depth 20 -Compress))
    $taskB=[System.Text.Json.Nodes.JsonNode]::Parse(($B|ConvertTo-Json -Depth 20 -Compress))
    [System.Text.Json.Nodes.JsonNode]::DeepEquals($taskA,$taskB)
}
$taskReference=[ordered]@{a=1;b=@(2,3)}
Require (Same $taskReference ([ordered]@{b=@(2,3);a=1})) 'Object order must not change JSON facts'
foreach($taskChanged in @(
    [ordered]@{a=2;b=@(2,3)},
    [ordered]@{a='1';b=@(2,3)},
    [ordered]@{a=1;b=@(3,2)},
    [ordered]@{a=1},
    [ordered]@{a=1;b=$null},
    [ordered]@{a=1;b=@(2,3);c=0},
    [ordered]@{A=1;b=@(2,3)},
    [ordered]@{a=$true;b=@(2,3)}
)){
    Require (-not (Same $taskReference $taskChanged)) 'Changed JSON value, type, fields or array order accepted'
}
function Records([string]$Name,[string]$Marker,[string]$Test){
    $taskRows=@()
    $taskPrefix="test $Test ... "
    foreach($taskLine in Get-Content (Join-Path $PSScriptRoot $Name)){
        if($taskLine.StartsWith($taskPrefix+$Marker+' ')){$taskLine=$taskLine.Substring($taskPrefix.Length)}
        if($taskLine.StartsWith($Marker+' ')){$taskRows+=($taskLine.Substring($Marker.Length+1)|ConvertFrom-Json)}
    }
    $taskRows
}
function Facts{
    $taskPreflight=Json 'authoring-preflight.json'
    Require ($taskPreflight.original_refused -and $taskPreflight.repair_verified -and -not $taskPreflight.measured -and $taskPreflight.original_recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'review.ps1')) -and $taskPreflight.repair_recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'review-repair.ps1'))) 'Authoring refusal or repaired recipe differs'
    $taskRefusal=Json 'Capture-check-xtask-exit.json'
    Require ($taskRefusal.failure -ceq 'Failed focused (101)' -and $taskRefusal.steps.Count -eq 1 -and $taskRefusal.steps[0].exit -eq 101) 'Original punctuation-spacing refusal not retained'
    Require ($taskRefusal.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'check.ps1'))) 'Original failed check recipe changed'
    Require ((Hash (Join-Path $PSScriptRoot $taskRefusal.steps[0].log)) -ceq $taskRefusal.steps[0].sha256) 'Original failed check log changed'
    $taskPhases=@('full','directory','inventory','records','markers','bind','traced-inventory','traced-records','traced-markers')
    $taskCases=@('boot','namespace-user','namespace-mnt','namespace-pid','namespace-time','origin','authority-1','authority-7')
    $taskLanes=@()
    foreach($taskLane in @('194','188')){
        $taskReceipt=Json "Capture-$taskLane-exit.json"
        Require ($taskReceipt.stage -ceq 'Capture' -and $taskReceipt.lane -ceq $taskLane -and $taskReceipt.source -cmatch '^[0-9a-f]{40}$') 'Profile identity differs'
        Require ($taskReceipt.image_sha256 -cmatch '^[0-9a-f]{64}$' -and $taskReceipt.image_sha256 -ceq $taskReceipt.image_after_sha256) 'Image changed'
        Require ($taskReceipt.steps.Count -eq 2 -and ($taskReceipt.steps.name -join ',') -ceq 'breakdown,inspect' -and @($taskReceipt.steps|Where-Object exit -ne 0).Count -eq 0) 'Profile incomplete or failed'
        foreach($taskFlag in @('server','product','native_gate_open')){Require ($taskReceipt.$taskFlag -is [bool] -and -not $taskReceipt.$taskFlag) 'Profile scope differs'}
        Require ($taskReceipt.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'profile-lifetime.ps1'))) 'Profile recipe changed'
        Require ($taskReceipt.inputs.Count -eq 6 -and @($taskReceipt.inputs.path|Select-Object -Unique).Count -eq 6) 'Input set differs'
        foreach($taskInput in $taskReceipt.inputs){
            $taskBlob=(& git rev-parse "$($taskReceipt.source):$($taskInput.path)").Trim()
            Require ($LASTEXITCODE -eq 0 -and $taskBlob -ceq $taskInput.blob) 'Committed input differs'
            $taskText=((& git show "$($taskReceipt.source):$($taskInput.path)") -join "`n")+"`n"
            Require ($LASTEXITCODE -eq 0 -and (Text-Hash $taskText) -ceq $taskInput.normalized_sha256) 'Normalized input differs'
        }
        $taskTest='diagnostic_kernel_admission::origin::fixed_leaf_path_tests::allocations::authority_owners_alloc::authority_owners_allocation_attribution'
        $taskFullTest='diagnostic_kernel_admission::origin::fixed_leaf_path_tests::allocations::validation_leaf_requests::validation_leaf_requests_attribution'
        $taskRows=@(Records "Capture-$taskLane-breakdown.log" 'HC074-AUTHORITY-OWNERS' $taskTest)
        $taskOrigin=@(Records "Capture-$taskLane-breakdown.log" 'HC074-AUTHORITY-OWNERS-ORIGIN' $taskTest)
        $taskFixtures=@(Records "Capture-$taskLane-breakdown.log" 'HC074-AUTHORITY-OWNERS-FIXTURE' $taskTest)
        $taskFull=@(Records "Capture-$taskLane-inspect.log" 'HC074-VALIDATION-LEAF-ALLOC' $taskFullTest)
        $taskIo=@(Records "Capture-$taskLane-inspect.log" 'HC074-VALIDATION-LEAF-IO' $taskFullTest)
        Require ($taskRows.Count -eq 54 -and $taskOrigin.Count -eq 1 -and $taskFixtures.Count -eq 2 -and $taskFull.Count -eq 24 -and $taskIo.Count -eq 24) 'Incomplete observations'
        Require ($taskOrigin[0].uid -gt 0 -and $taskOrigin[0].samples -eq 3 -and $taskOrigin[0].iterations -eq 1024 -and $taskOrigin[0].warmups -eq 32) 'Origin scope differs'
        foreach($taskKind in @('breakdown','inspect')){Require ((Get-Content (Join-Path $PSScriptRoot "Capture-$taskLane-$taskKind.log") -Raw).Contains('1 passed; 0 failed')) 'Native test incomplete'}
        $taskDepths=@()
        foreach($taskDepth in @(1,7)){
            $taskIdentity=@($taskFixtures|Where-Object generations -EQ $taskDepth)
            Require ($taskIdentity.Count -eq 1 -and $taskIdentity[0].revision -eq $taskDepth -and $taskIdentity[0].entries.Count -eq $taskDepth -and $taskIdentity[0].epoch -eq (7+[math]::Floor($taskDepth/2)) -and $taskIdentity[0].root_name_bytes -eq 27) 'Journal identity differs'
            for($taskIndex=0;$taskIndex -lt $taskDepth;$taskIndex++){
                $taskEntry=$taskIdentity[0].entries[$taskIndex]
                Require ($taskEntry.revision -eq $taskIndex+1 -and $taskEntry.record_name_bytes -eq 21 -and $taskEntry.marker_name_bytes -eq 23 -and $taskEntry.marker_bytes -eq 65 -and $taskEntry.record_bytes -gt 0 -and $taskEntry.record_sha256 -cmatch '^[0-9a-f]{64}$' -and $taskEntry.marker_sha256 -cmatch '^[0-9a-f]{64}$') 'Entry identity differs'
            }
            $taskObserved=@()
            foreach($taskPhase in $taskPhases){
                $taskSamples=@($taskRows|Where-Object { $_.generations -eq $taskDepth -and $_.phase -ceq $taskPhase }|Sort-Object sample)
                Require ($taskSamples.Count -eq 3) 'Phase incomplete'
                for($taskSample=0;$taskSample -lt 3;$taskSample++){
                    $taskOne=$taskSamples[$taskSample]
                    $taskPhaseIndex=[array]::IndexOf($taskPhases,$taskPhase)
                    Require ($taskOne.sample -eq $taskSample -and $taskOne.position -eq (($taskPhaseIndex-$taskSample+9)%9) -and $taskOne.iterations -eq 1024 -and $taskOne.warmups -eq 32) 'Phase rotation or workload differs'
                    foreach($taskFlag in @('allocator_active_during_io','all_syscalls_observed','runtime_changed','timing_claim','server','product')){Require ($taskOne.$taskFlag -is [bool] -and -not $taskOne.$taskFlag) 'Phase scope differs'}
                    Require ($taskOne.rust_requests_only -is [bool] -and $taskOne.rust_requests_only) 'Request scope absent'
                    Require ($taskOne.empty.syscr -eq 1 -and $taskOne.empty.syscw -eq 0 -and $taskOne.observed.syscw -eq 0 -and $taskOne.observed.wchar -eq 0) 'Unexpected I/O'
                    Require (Same $taskOne.counts $taskSamples[0].counts) 'Phase requests do not repeat'
                    $taskSites=@('path','dir-open','dir-next','dir-name','dir-insert','dir-expected','held-metadata','leaf-open','leaf-before','parent-metadata','buffer','read','after')
                    Require ($taskOne.sites.Count -eq 13 -and ($taskOne.sites.site -join ',') -ceq ($taskSites -join ',')) 'Expression site inventory differs'
                    Require (Same $taskOne.sites $taskSamples[0].sites) 'Expression samples do not repeat'
                    $taskCalls=@(0)*13
                    if($taskPhase -ceq 'traced-inventory'){$taskCalls=@(1024,1024,((2*$taskDepth+1)*1024),(2*$taskDepth*1024),(2*$taskDepth*1024),1024,0,0,0,0,0,0,0)}
                    if($taskPhase -in @('traced-records','traced-markers')){$taskCalls=@(0,0,0,0,0,0)+(@(1024*$taskDepth)*7)}
                    for($taskSiteIndex=0;$taskSiteIndex -lt 13;$taskSiteIndex++){
                        Require ($taskOne.sites[$taskSiteIndex].calls -eq $taskCalls[$taskSiteIndex]) 'Expression call count differs'
                    }
                    foreach($taskMetric in @('allocations','reallocations','requested_bytes')){
                        $taskTotal=0
                        foreach($taskSite in $taskOne.sites){$taskTotal+=$taskSite.counts.$taskMetric}
                        if($taskPhaseIndex -ge 6){Require ($taskTotal -eq $taskOne.counts.$taskMetric) 'Expression allocation sum differs from phase'}
                        else{Require ($taskTotal -eq 0) 'Actual phase must not pretend to have expression tags'}
                    }
                    Require (($taskOne.observed.syscr-$taskOne.empty.syscr) -eq ($taskSamples[0].observed.syscr-$taskSamples[0].empty.syscr)) 'Phase read count does not repeat'
                }
                $taskObserved+=[ordered]@{phase=$taskPhase;per_operation=[ordered]@{allocations=$taskSamples[0].counts.allocations/1024;reallocations=$taskSamples[0].counts.reallocations/1024;requested_bytes=$taskSamples[0].counts.requested_bytes/1024;read_requests=($taskSamples[0].observed.syscr-$taskSamples[0].empty.syscr)/1024};samples=$taskSamples}
            }
            $taskByPhase=@{}
            foreach($taskItem in $taskObserved){$taskByPhase[$taskItem.phase]=$taskItem.per_operation}
            foreach($taskMetric in @('allocations','reallocations','requested_bytes','read_requests')){
                $taskExpected=2*$taskByPhase.directory.$taskMetric+2*$taskByPhase.inventory.$taskMetric+$taskByPhase.records.$taskMetric+$taskByPhase.markers.$taskMetric+$taskByPhase.bind.$taskMetric
                Require ($taskByPhase.full.$taskMetric -eq $taskExpected) 'Full inspect does not conserve actual phases'
                foreach($taskPair in @(@('inventory','traced-inventory'),@('records','traced-records'),@('markers','traced-markers'))){
                    Require ($taskByPhase[$taskPair[0]].$taskMetric -eq $taskByPhase[$taskPair[1]].$taskMetric) 'Actual/traced phase does not conserve'
                }
            }
            $taskBaseline=if($taskDepth -eq 1){@(23,0,1662,4)}else{@(99,0,8444,28)}
            $taskMetrics=@('allocations','reallocations','requested_bytes','read_requests')
            for($taskIndex=0;$taskIndex -lt 4;$taskIndex++){Require ($taskByPhase.full.($taskMetrics[$taskIndex]) -eq $taskBaseline[$taskIndex]) 'Frozen full inspect baseline changed'}
            $taskDepths+=[ordered]@{generations=$taskDepth;identity=$taskIdentity[0];phases=$taskObserved}
        }
        foreach($taskCase in $taskCases){
            $taskA=@($taskFull|Where-Object case -CEQ $taskCase|Sort-Object sample)
            $taskI=@($taskIo|Where-Object case -CEQ $taskCase|Sort-Object sample)
            Require ($taskA.Count -eq 3 -and $taskI.Count -eq 3) 'Unchanged leaf control incomplete'
            for($taskSample=0;$taskSample -lt 3;$taskSample++){
                Require ($taskA[$taskSample].sample -eq $taskSample -and $taskI[$taskSample].sample -eq $taskSample -and $taskA[$taskSample].iterations -eq 1024 -and $taskI[$taskSample].iterations -eq 1024) 'Leaf workload differs'
                Require ($taskA[$taskSample].rust_requests_only -and -not $taskI[$taskSample].allocator_active -and -not $taskI[$taskSample].all_syscalls_observed -and -not $taskA[$taskSample].authority_fixed_ancestry -and -not $taskI[$taskSample].authority_fixed_ancestry -and -not $taskA[$taskSample].server -and -not $taskI[$taskSample].server -and -not $taskA[$taskSample].product -and -not $taskI[$taskSample].product) 'Leaf scope differs'
                Require ($taskI[$taskSample].empty.syscr -eq 1 -and $taskI[$taskSample].empty.syscw -eq 0 -and $taskI[$taskSample].observed.syscw -eq 0 -and $taskI[$taskSample].observed.wchar -eq 0) 'Leaf I/O differs'
                $taskExpected=[ordered]@{allocations=0;reallocations=0;requested_bytes=0}
                if($taskCase.StartsWith('authority-')){
                    $taskDepth=[int]$taskCase.Split('-')[1]
                    $taskBaseline=if($taskDepth -eq 1){@(23,0,1662,4)}else{@(99,0,8444,28)}
                    $taskExpected=[ordered]@{allocations=1024*$taskBaseline[0];reallocations=0;requested_bytes=1024*$taskBaseline[2]}
                    Require (($taskI[$taskSample].observed.syscr-$taskI[$taskSample].empty.syscr) -eq 1024*$taskBaseline[3]) 'Leaf authority reads differ'
                }
                Require (Same $taskA[$taskSample].counts $taskExpected) 'Unchanged leaf request control differs'
            }
        }
        $taskLanes+=[ordered]@{lane=$taskLane;source=$taskReceipt.source;image_sha256=$taskReceipt.image_sha256;origin=$taskOrigin[0];depths=$taskDepths}
    }
    Require ($taskLanes[0].source -ceq $taskLanes[1].source) 'Compiler sources differ'
    for($taskDepthIndex=0;$taskDepthIndex -lt 2;$taskDepthIndex++){
        Require (Same $taskLanes[0].depths[$taskDepthIndex].identity.entries $taskLanes[1].depths[$taskDepthIndex].identity.entries) 'Compiler payload/name inputs differ'
        for($taskPhaseIndex=0;$taskPhaseIndex -lt 9;$taskPhaseIndex++){
            Require (Same $taskLanes[0].depths[$taskDepthIndex].phases[$taskPhaseIndex].per_operation $taskLanes[1].depths[$taskDepthIndex].phases[$taskPhaseIndex].per_operation) 'Compiler phase costs differ'
            Require (Same $taskLanes[0].depths[$taskDepthIndex].phases[$taskPhaseIndex].samples[0].sites $taskLanes[1].depths[$taskDepthIndex].phases[$taskPhaseIndex].samples[0].sites) 'Compiler expression costs differ'
        }
    }
    foreach($taskCheckName in @('Capture-check-194-lifetime','Capture-check-188-lifetime','Capture-check-xtask-lifetime','Capture-check-contracts-lifetime','Capture-check-repository-lifetime')){
        $taskCheck=Json "$taskCheckName-exit.json"
        $taskCount=if($taskCheckName.Contains('xtask')){3}elseif($taskCheckName.Contains('contracts')){1}elseif($taskCheckName.Contains('repository')){4}else{11}
        Require (-not $taskCheck.failure -and $taskCheck.steps.Count -eq $taskCount -and @($taskCheck.steps.name|Select-Object -Unique).Count -eq $taskCount -and @($taskCheck.steps|Where-Object exit -ne 0).Count -eq 0) 'Checks failed or incomplete'
        $taskRecipe='check-lifetime.ps1'
        Require ($taskCheck.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskRecipe))) 'Check recipe differs'
        foreach($taskStep in $taskCheck.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'Check log changed'}
    }
    foreach($taskHistoryName in @('Capture-check-194','Capture-check-188','Capture-check-xtask-repair','Capture-check-contracts')){
        $taskHistory=Json "$taskHistoryName-exit.json"
        $taskHistoryCount=if($taskHistoryName.Contains('xtask')){3}elseif($taskHistoryName.Contains('contracts')){1}else{11}
        Require (-not $taskHistory.failure -and $taskHistory.steps.Count -eq $taskHistoryCount -and @($taskHistory.steps|Where-Object exit -ne 0).Count -eq 0) 'Pre-retirement audit checks not retained'
        $taskRecipe=if($taskHistoryName.EndsWith('-repair')){'check-repair.ps1'}else{'check.ps1'}
        Require ($taskHistory.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskRecipe))) 'History recipe changed'
        foreach($taskStep in $taskHistory.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'History log changed'}
    }
    [ordered]@{schema='authority-owners-local-control-074-v1';complete_request_cohort=$true;decomposition_conserved=$true;frozen_full_control_equal=$true;runtime_changed=$false;allocation_reduction_claimed=$false;latency_gain_claimed=$false;all_syscalls_observed=$false;full_active_authority_measured=$false;existing_live_refusals_retained=$true;native_gate_open=$false;server=$false;product=$false;lanes=$taskLanes}
}
function Verify-Inventory([string]$Directory,[string]$ExpectedHash){
    $taskFile=Join-Path $Directory 'inventory.json'
    if($ExpectedHash){Require ((Hash $taskFile) -ceq $ExpectedHash) 'Prior inventory changed'}
    $taskInventory=Get-Content -LiteralPath $taskFile -Raw|ConvertFrom-Json
    $taskNames=@($taskInventory.files|ForEach-Object path|Sort-Object)
    $taskActual=@(Get-ChildItem -LiteralPath $Directory -File -Recurse|Where-Object Name -ne 'inventory.json'|ForEach-Object {$_.FullName.Substring($Directory.Length+1).Replace('\','/')}|Sort-Object)
    Require (($taskNames -join "`n") -ceq ($taskActual -join "`n")) 'Inventory members differ'
    foreach($taskMember in $taskInventory.files){
        $taskPath=[IO.Path]::GetFullPath((Join-Path $Directory $taskMember.path))
        Require ($taskPath.StartsWith($Directory+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)) 'Member escapes packet'
        Require ((Get-Item -LiteralPath $taskPath).Length -eq $taskMember.bytes -and (Hash $taskPath) -ceq $taskMember.sha256) "Member changed: $($taskMember.path)"
    }
    $taskInventory.files.Count
}
$taskFacts=Facts
$taskSummary=Join-Path $PSScriptRoot 'summary.json'
if($Mode -ceq 'WriteSummary'){
    Require (-not (Test-Path $taskSummary)) 'Never overwrite summary'
    $taskFacts|ConvertTo-Json -Depth 20|Set-Content -Encoding utf8NoBOM $taskSummary
    exit
}
$taskStored=Json 'summary.json'
Require (Same $taskStored $taskFacts) 'Summary differs from observations'
$taskMutations=0
foreach($taskFlag in @('complete_request_cohort','decomposition_conserved','frozen_full_control_equal','runtime_changed','allocation_reduction_claimed','latency_gain_claimed','all_syscalls_observed','full_active_authority_measured','existing_live_refusals_retained','native_gate_open','server','product')){
    $taskChanged=($taskStored|ConvertTo-Json -Depth 20)|ConvertFrom-Json
    $taskChanged.$taskFlag=-not $taskChanged.$taskFlag
    Require (-not (Same $taskChanged $taskFacts)) 'Scope mutation accepted'
    $taskMutations++
}
$taskPrior=@(
    @('authority-fd-path-20261021','ff4d9e59bffefb40eb1712f253eaebce701a34b832a5417211ec88b144fa2440'),
    @('authority-breakdown-20261021','c758e549722903dd77d20e20da00347a3a950964ee8bb682ad43a10a6782003d'),
    @('authority-expected-name-20261021','4249e3cd970a5159951a22b5e40762b4aaa97eee003118d664bb67110376464f'),
    @('authority-name-capacity-20261021','2d94764f08a89ed50e4e1d8ce47906382fb6be6c796bd585e7d0fbdc5f628362'),
    @('authority-inventory-20261021','101b29bf56b767da04c86bcdd7d14fa9e488de3c319f11e56d718b614effdbef'),
    @('authority-leaf-name-20261020','384430c3e8a18f08e521ffb3d7ca4aba8428001f8011347265577a7a5c139a12'),
    @('authority-context-20261020','25796efbc05cbcca3a7c646d70624774979c84fb16c4beb84746a4c0cc70a01f'),
    @('authority-key-hex-20261020','6d7cadf2d084320d4d68c5fe73a44e6fe964003bb8e6e53d6b8c1af0905413f0'),
    @('validation-leaf-requests-20261020','9861517676b8ed06b49ebe9d3ebd3c9d8d4835c74fd81c4ff6703edccd9f9ad3'),
    @('validation-round-cost-20261020','b089573e0dc544df769c07b64be41562f107cdc1add64b1cd0c9512df2d882d6'),
    @('starting-effects-cost-20261020','69c7057e4994abea20e148a9179fe8229b6f6725ec7d1002e6d80484931a2d3c'),
    @('remaining-preparation-cost-20261020','2839ea78850f17f8070807b8853c0726133200311ccfd37f4f811b95ff796fbc'),
    @('custody-preparation-cost-20261020','3194244cb4e45c6f4f611b946afaa6a6a8b3aaf83179457c260d0a9e25f93864'),
    @('held-native-recheck-20261020','2a7be06fe6f93e3e8761b29afebaf89ac6266d7c405b457ef8ea26d60259bd63'),
    @('held-composition-recheck-20261020','0bfa7cef4d3ebd69c9e751fe6e4304a8bade3133122dcc48249c9d1396bfe1c0'),
    @('pid-path-cost-20261020','982d9598426830f6b3e44f9a701220b5778ceb3e6b568922d3b2ee405a0e6b05')
)
foreach($taskOld in $taskPrior){$null=Verify-Inventory (Join-Path (Split-Path $PSScriptRoot) $taskOld[0]) $taskOld[1]}
$taskFinal=Json 'final-check-exit.json'
Require (-not $taskFinal.failure -and $taskFinal.frozen_inputs_equal -and $taskFinal.steps.Count -eq 3 -and @($taskFinal.steps|Where-Object exit -ne 0).Count -eq 0) 'Final checks incomplete'
Require ($taskFinal.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'final-check.ps1'))) 'Final recipe changed'
foreach($taskStep in $taskFinal.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'Final check log changed'}
if($Mode -ceq 'Seal'){
    $taskInventory=Join-Path $PSScriptRoot 'inventory.json'
    Require (-not (Test-Path $taskInventory)) 'Never reseal'
    [ordered]@{schema='sha256-local-file-inventory-v1';files=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse|Sort-Object FullName|ForEach-Object {[ordered]@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM $taskInventory
}
$taskMembers=Verify-Inventory $PSScriptRoot ''
[ordered]@{integrity_verified=$true;members=$taskMembers;prior_packets=16;summary_mutations_rejected=$taskMutations;complete_request_cohort=$true;decomposition_conserved=$true;runtime_changed=$false;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
