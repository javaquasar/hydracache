param([ValidateSet('BaselineReview','WriteSummary','Seal','Review','Audit')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){
    [System.Text.Json.Nodes.JsonNode]::DeepEquals(
        [System.Text.Json.Nodes.JsonNode]::Parse(($A|ConvertTo-Json -Depth 40 -Compress)),
        [System.Text.Json.Nodes.JsonNode]::Parse(($B|ConvertTo-Json -Depth 40 -Compress)))
}
function Records([string]$File,[string]$Marker){
    foreach($taskLine in Get-Content -LiteralPath $File){
        $taskAt=$taskLine.IndexOf($Marker+' ',[StringComparison]::Ordinal)
        if($taskAt -ge 0){$taskLine.Substring($taskAt+$Marker.Length+1)|ConvertFrom-Json}
    }
}
function Inactive($Row,[string[]]$Flags){
    foreach($taskFlag in $Flags){Require ($Row.$taskFlag -is [bool] -and -not $Row.$taskFlag) "Unexpected scope: $taskFlag"}
}
function ExpectedCounts($Row,[long]$Alloc,[long]$Bytes,[long]$Realloc=0){
    Require ($Row.allocations -eq $Alloc -and $Row.requested_bytes -eq $Bytes -and $Row.reallocations -eq $Realloc) 'Rust requests differ'
}
function Distribution($Values){
    $taskSorted=@($Values|Sort-Object)
    Require ($taskSorted.Count -gt 0) 'Empty distribution'
    $taskMiddle=[int][Math]::Floor($taskSorted.Count/2)
    $taskMedian=if($taskSorted.Count%2){[double]$taskSorted[$taskMiddle]}else{([double]$taskSorted[$taskMiddle-1]+[double]$taskSorted[$taskMiddle])/2}
    @{minimum_ns=$taskSorted[0];median_ns=$taskMedian;maximum_ns=$taskSorted[-1]}
}
function Check([string]$Stage,[string]$Lane){
    $taskReceipt=Json "$Stage-check-$Lane-exit.json"
    $taskCount=if($Lane -ceq 'xtask'){3}elseif($Lane -ceq 'contracts'){1}elseif($Lane -ceq 'repository'){4}else{11}
    Require (-not $taskReceipt.failure -and $taskReceipt.steps.Count -eq $taskCount -and @($taskReceipt.steps|Where-Object exit -ne 0).Count -eq 0) 'Incomplete checks'
    Require (@($taskReceipt.steps.name|Select-Object -Unique).Count -eq $taskCount) 'Duplicate check step'
    Require ($taskReceipt.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'check-repair.ps1'))) 'Check recipe drift'
    foreach($taskStep in $taskReceipt.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Check log changed'}
    Inactive $taskReceipt @('server','product')
    $taskReceipt
}
function PreservedPreflight{
    foreach($taskLane in @('194','188','xtask')){
        $taskReceipt=Json "Baseline-check-$taskLane-exit.json"
        Require ($taskReceipt.failure -and @($taskReceipt.steps|Where-Object exit -ne 0).Count -eq 1) 'Original failed preflight missing'
        Require ($taskReceipt.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'check.ps1'))) 'Original recipe changed'
        foreach($taskStep in $taskReceipt.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Original failure log changed'}
    }
}
function Prior{
    $taskPrior=Join-Path (Split-Path $PSScriptRoot) 'authority-leaf-cpu-20261021'
    Require ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq '4a4c317f7e9cc878b57aa205324d865577cd17772bff3dea0d7e513c01b2d805') 'Prior anchor changed'
    $taskInventory=Get-Content (Join-Path $taskPrior 'inventory.json') -Raw|ConvertFrom-Json
    foreach($taskMember in $taskInventory.files){Require ((Hash (Join-Path $taskPrior $taskMember.path)) -ceq $taskMember.sha256) 'Prior CPU packet changed'}
    & (Join-Path $taskPrior 'review.ps1') -Mode Review|Out-Null
    Require ((Hash (Join-Path $taskRepo 'docs/testing/performance/0.74/qualification-manifest.toml')) -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification changed'
}
function StageFacts([string]$Stage,[string]$Mutation=''){
    $taskIsCandidate=$Stage -ceq 'Candidate'
    $taskCheckStage=if($taskIsCandidate){'Candidate'}else{'BaselineRepair'}
    $taskLanes=@()
    $taskSource=$null
    foreach($taskLane in @('194','188')){
        $taskCheck=Check $taskCheckStage $taskLane
        $taskReceipt=Json "$Stage-$taskLane-exit.json"
        Require ($taskReceipt.stage -ceq $Stage -and $taskReceipt.lane -ceq $taskLane -and $taskReceipt.source -cmatch '^[0-9a-f]{40}$') 'Profile identity differs'
        if($taskSource){Require ($taskReceipt.source -ceq $taskSource) 'Compiler sources differ'}else{$taskSource=$taskReceipt.source}
        $taskParent=(& git rev-parse "$($taskSource)^").Trim()
        Require ($LASTEXITCODE -eq 0 -and $taskParent -ceq $taskCheck.source) 'Precommit check parent differs'
        Require ($taskReceipt.image_sha256 -cmatch '^[0-9a-f]{64}$' -and $taskReceipt.image_sha256 -ceq $taskReceipt.image_after_sha256) 'Image changed'
        Require ($taskReceipt.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'profile.ps1'))) 'Profile recipe changed'
        Require ($taskReceipt.steps.Count -eq 3 -and ($taskReceipt.steps.name -join ',') -ceq 'cpu,owners,inspect' -and @($taskReceipt.steps|Where-Object exit -ne 0).Count -eq 0) 'Measured cohort failed/incomplete'
        Require ($taskReceipt.legacy_row_flags_are_not_comparative -eq $true) 'Legacy row scope not declared'
        Inactive $taskReceipt @('server','product','native_gate_open')
        Require ($taskReceipt.inputs.Count -eq 9 -and @($taskReceipt.inputs.path|Select-Object -Unique).Count -eq 9) 'Input set differs'
        foreach($taskInput in $taskReceipt.inputs){
            $taskBlob=(& git rev-parse "$($taskSource):$($taskInput.path)").Trim()
            Require ($LASTEXITCODE -eq 0 -and $taskBlob -ceq $taskInput.blob) 'Input blob differs'
            $taskText=((& git show "$($taskSource):$($taskInput.path)") -join "`n")+"`n"
            $taskHash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($taskText))).ToLowerInvariant()
            Require ($LASTEXITCODE -eq 0 -and $taskHash -ceq $taskInput.normalized_sha256) 'Input hash differs'
        }
        $taskLog=Join-Path $PSScriptRoot "$Stage-$taskLane-cpu.log"
        $taskOrigin=@(Records $taskLog 'HC074-OPEN-NAME-ORIGIN')
        Require ($taskOrigin.Count -eq 1) 'Origin count differs'
        $taskOrigin=$taskOrigin[0]
        Require ($taskOrigin.uid -gt 0 -and $taskOrigin.boot_id -cmatch '^[0-9a-f-]{36}$' -and $taskOrigin.clock -ceq 'CLOCK_THREAD_CPUTIME_ID' -and $taskOrigin.resolution_ns -gt 0 -and $taskOrigin.clock_pair_ns.Count -eq 64) 'Origin/clock differs'
        Require ($taskOrigin.iterations -eq 1024 -and $taskOrigin.samples -eq 20 -and $taskOrigin.allocation_samples -eq 3 -and $taskOrigin.warmups -eq 32) 'Workload changed'
        Inactive $taskOrigin @('server','product','native_gate_open','overhead_subtracted')
        $taskFixtures=@(Records $taskLog 'HC074-OPEN-NAME-FIXTURE')
        $taskAlloc=@(Records $taskLog 'HC074-OPEN-NAME-ALLOC')
        $taskCpu=@(Records $taskLog 'HC074-OPEN-NAME-CPU')
        $taskLeaf=@(Records $taskLog 'HC074-OPEN-NAME-LEAF')
        Require ($taskFixtures.Count -eq 6 -and $taskAlloc.Count -eq 18 -and $taskCpu.Count -eq 120 -and $taskLeaf.Count -eq 12) 'Incomplete helper cases'
        if($Mutation -ceq 'requests'){$taskAlloc[0].actual.allocations++}
        if($Mutation -ceq 'scope'){$taskCpu[0].server=$true}
        if($Mutation -ceq 'difference'){$taskCpu[0].difference_ns++}
        if($Mutation -ceq 'order'){$taskCpu[0].order=@(1,0)}
        $taskStats=@()
        foreach($taskCase in 0..5){
            $taskFixture=@($taskFixtures|Where-Object case -eq $taskCase)
            Require ($taskFixture.Count -eq 1) 'Duplicate fixture'
            $taskIdentity=$taskFixture[0].identity
            Require ($taskIdentity.name_bytes -eq @(21,23,24,23,255,3)[$taskCase] -and $taskIdentity.directory -eq ($taskCase -eq 5) -and $taskIdentity.temporary -eq $true) 'Helper shape differs'
            $taskNameHash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($taskIdentity.name))).ToLowerInvariant()
            Require ($taskNameHash -ceq $taskIdentity.name_sha256) 'Helper name differs'
            $taskSaved=($taskIsCandidate -and $taskCase -in @(0,1,3))
            for($taskSample=0;$taskSample -lt 3;$taskSample++){
                $taskRow=@($taskAlloc|Where-Object {$_.case -eq $taskCase -and $_.sample -eq $taskSample})
                Require ($taskRow.Count -eq 1) 'Allocation sample differs'
                $taskRow=$taskRow[0]
                Require ($taskRow.iterations -eq 1024 -and ($taskRow.order -join ',') -ceq $(if($taskSample%2){'1,0'}else{'0,1'})) 'Allocation parameters/order differ'
                ExpectedCounts $taskRow.reference 1024 (($taskIdentity.name_bytes+1)*1024)
                ExpectedCounts $taskRow.actual $(if($taskSaved){0}else{1024}) $(if($taskSaved){0}else{($taskIdentity.name_bytes+1)*1024})
                Inactive $taskRow @('server','product')
            }
            for($taskSample=0;$taskSample -lt 20;$taskSample++){
                $taskRow=@($taskCpu|Where-Object {$_.case -eq $taskCase -and $_.sample -eq $taskSample})
                Require ($taskRow.Count -eq 1) 'CPU sample differs'
                $taskRow=$taskRow[0]
                Require ($taskRow.iterations -eq 1024 -and ($taskRow.order -join ',') -ceq $(if($taskSample%2){'1,0'}else{'0,1'}) -and $taskRow.reference_ns -gt 0 -and $taskRow.actual_ns -gt 0) 'CPU parameters/order differ'
                Require ($taskRow.difference_ns -eq ([long]$taskRow.actual_ns-[long]$taskRow.reference_ns)) 'Signed pair difference differs'
                Inactive $taskRow @('allocator_active','overhead_subtracted','server','product')
            }
            $taskPairs=@($taskCpu|Where-Object case -eq $taskCase|Sort-Object sample)
            $taskStats+=@{case=$taskCase;identity=$taskIdentity;reference=(Distribution @($taskPairs|ForEach-Object {[double]$_.reference_ns/1024}));actual=(Distribution @($taskPairs|ForEach-Object {[double]$_.actual_ns/1024}));signed_pairs=(Distribution @($taskPairs|ForEach-Object {[double]$_.difference_ns/1024}));positive_pairs=@($taskPairs|Where-Object difference_ns -gt 0).Count;negative_pairs=@($taskPairs|Where-Object difference_ns -lt 0).Count}
        }
        foreach($taskCase in 0..1){
            foreach($taskCold in @($true,$false)){
                for($taskSample=0;$taskSample -lt 3;$taskSample++){
                    $taskRow=@($taskLeaf|Where-Object {$_.case -eq $taskCase -and $_.cold -eq $taskCold -and $_.sample -eq $taskSample})
                    Require ($taskRow.Count -eq 1) 'Cold/warm sample differs'
                    $taskRow=$taskRow[0]
                    $taskLength=@(21,23)[$taskCase]
                    $taskAllocations=if($taskCold){3}else{2}
                    $taskBytes=65+$taskLength+1+$(if($taskCold){$taskLength}else{0})
                    if($taskIsCandidate){$taskAllocations--;$taskBytes-=$taskLength+1}
                    Require ($taskRow.iterations -eq 1024 -and $taskRow.warmups -eq 32) 'Cold/warm parameters differ'
                    ExpectedCounts $taskRow.counts ($taskAllocations*1024) ($taskBytes*1024)
                    Inactive $taskRow @('whole_registry_initialization','server','product')
                }
            }
        }
        $taskOwnersLog=Join-Path $PSScriptRoot "$Stage-$taskLane-owners.log"
        $taskOwners=@(Records $taskOwnersLog 'HC074-AUTHORITY-OWNERS')
        $taskOwnersFixtures=@(Records $taskOwnersLog 'HC074-AUTHORITY-OWNERS-FIXTURE')
        $taskPriorRoot=Join-Path (Split-Path $PSScriptRoot) 'authority-leaf-cpu-20261021'
        $taskOldOwners=@(Records (Join-Path $taskPriorRoot "Capture-$taskLane-owners.log") 'HC074-AUTHORITY-OWNERS')
        $taskOldFixtures=@(Records (Join-Path $taskPriorRoot "Capture-$taskLane-owners.log") 'HC074-AUTHORITY-OWNERS-FIXTURE')
        Require ($taskOwners.Count -eq 54 -and $taskOwnersFixtures.Count -eq 2) 'Owners cohort incomplete'
        foreach($taskFixture in $taskOwnersFixtures){
            $taskOld=@($taskOldFixtures|Where-Object generations -eq $taskFixture.generations)
            Require ($taskOld.Count -eq 1) 'Owners depth differs'
            foreach($taskField in @('generations','revision','epoch','root_name_bytes','entries')){Require (Same $taskFixture.$taskField $taskOld[0].$taskField) 'Owners identity changed'}
        }
        foreach($taskRow in $taskOwners){
            $taskOld=@($taskOldOwners|Where-Object {$_.generations -eq $taskRow.generations -and $_.phase -ceq $taskRow.phase -and $_.sample -eq $taskRow.sample})
            Require ($taskOld.Count -eq 1 -and $taskRow.iterations -eq 1024 -and $taskRow.warmups -eq 32 -and $taskRow.position -eq $taskOld[0].position) 'Owners parameters differ'
            $taskOld=$taskOld[0]
            $taskCalls=0;$taskBytes=0
            if($taskIsCandidate){
                if($taskRow.phase -ceq 'full'){$taskCalls=2*$taskRow.generations;$taskBytes=46*$taskRow.generations}
                if($taskRow.phase -in @('records','traced-records')){$taskCalls=$taskRow.generations;$taskBytes=22*$taskRow.generations}
                if($taskRow.phase -in @('markers','traced-markers')){$taskCalls=$taskRow.generations;$taskBytes=24*$taskRow.generations}
            }
            ExpectedCounts $taskRow.counts ($taskOld.counts.allocations-$taskCalls*1024) ($taskOld.counts.requested_bytes-$taskBytes*1024) $taskOld.counts.reallocations
            Require ($taskRow.observed.syscr-$taskRow.empty.syscr -eq $taskOld.observed.syscr-$taskOld.empty.syscr -and $taskRow.empty.syscr -eq 1 -and $taskRow.observed.syscw -eq 0 -and $taskRow.observed.wchar -eq 0) 'Owners reads/writes changed'
            Require ($taskRow.sites.Count -eq 13) 'Owners expression count differs'
            for($taskSite=0;$taskSite -lt 13;$taskSite++){
                $taskNewSite=$taskRow.sites[$taskSite];$taskOldSite=$taskOld.sites[$taskSite]
                Require ($taskNewSite.site -ceq $taskOldSite.site -and $taskNewSite.calls -eq $taskOldSite.calls) 'Fresh expression call count changed'
                $taskSiteCalls=if($taskSite -eq 7 -and $taskRow.phase.StartsWith('traced-')){$taskCalls}else{0}
                $taskSiteBytes=if($taskSite -eq 7 -and $taskRow.phase.StartsWith('traced-')){$taskBytes}else{0}
                ExpectedCounts $taskNewSite.counts ($taskOldSite.counts.allocations-$taskSiteCalls*1024) ($taskOldSite.counts.requested_bytes-$taskSiteBytes*1024) $taskOldSite.counts.reallocations
            }
            Inactive $taskRow @('allocator_active_during_io','all_syscalls_observed','timing_claim','server','product')
        }
        $taskInspect=@(Records (Join-Path $PSScriptRoot "$Stage-$taskLane-inspect.log") 'HC074-VALIDATION-LEAF-ALLOC')
        $taskOldInspect=@(Records (Join-Path $taskPriorRoot "Capture-$taskLane-inspect.log") 'HC074-VALIDATION-LEAF-ALLOC')
        $taskIo=@(Records (Join-Path $PSScriptRoot "$Stage-$taskLane-inspect.log") 'HC074-VALIDATION-LEAF-IO')
        $taskOldIo=@(Records (Join-Path $taskPriorRoot "Capture-$taskLane-inspect.log") 'HC074-VALIDATION-LEAF-IO')
        Require ($taskInspect.Count -eq 24 -and $taskIo.Count -eq 24) 'Full control incomplete'
        foreach($taskRow in $taskInspect){
            $taskOld=@($taskOldInspect|Where-Object {$_.case -ceq $taskRow.case -and $_.sample -eq $taskRow.sample})
            Require ($taskOld.Count -eq 1 -and $taskRow.iterations -eq 1024) 'Full sample differs'
            $taskDepth=if($taskRow.case -ceq 'authority-1'){1}elseif($taskRow.case -ceq 'authority-7'){7}else{0}
            $taskCalls=if($taskIsCandidate){2*$taskDepth*1024}else{0}
            $taskBytes=if($taskIsCandidate){46*$taskDepth*1024}else{0}
            ExpectedCounts $taskRow.counts ($taskOld[0].counts.allocations-$taskCalls) ($taskOld[0].counts.requested_bytes-$taskBytes) $taskOld[0].counts.reallocations
        }
        foreach($taskRow in $taskIo){
            $taskOld=@($taskOldIo|Where-Object {$_.case -ceq $taskRow.case -and $_.sample -eq $taskRow.sample})
            Require ($taskOld.Count -eq 1 -and $taskRow.iterations -eq 1024 -and (Same $taskRow.kernel_calls $taskOld[0].kernel_calls)) 'Kernel control calls differ'
            Require ($taskRow.observed.syscr-$taskRow.empty.syscr -eq $taskOld[0].observed.syscr-$taskOld[0].empty.syscr -and $taskRow.empty.syscr -eq 1 -and $taskRow.observed.syscw -eq 0 -and $taskRow.observed.wchar -eq 0) 'Full reads/writes changed'
            Inactive $taskRow @('allocator_active','all_syscalls_observed','authority_fixed_ancestry','server','product')
        }
        $taskLanes+=@{lane=$taskLane;source=$taskSource;image_sha256=$taskReceipt.image_sha256;origin=$taskOrigin;helpers=$taskStats;cold_warm_rows=$taskLeaf;owners_fixtures=$taskOwnersFixtures;owners_rows=$taskOwners;full_rows=$taskInspect}
    }
    @{stage=$Stage;source=$taskSource;lanes=$taskLanes;helper_rows_per_lane=120;owners_rows_per_lane=54;full_rows_per_lane=24}
}
function Facts{
    PreservedPreflight
    Prior
    $taskBaseline=StageFacts 'Baseline'
    $taskCandidate=StageFacts 'Candidate'
    foreach($taskLane in @('xtask','contracts','repository')){Check 'Candidate' $taskLane|Out-Null}
    $taskBaselineInputs=(Json 'Baseline-194-exit.json').inputs
    $taskCandidateInputs=(Json 'Candidate-194-exit.json').inputs
    foreach($taskInput in $taskBaselineInputs|Where-Object path -ne 'tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs'){
        $taskOther=@($taskCandidateInputs|Where-Object path -ceq $taskInput.path)
        Require ($taskOther.Count -eq 1 -and (Same $taskInput $taskOther[0])) 'Instrumentation or preregistration changed between captures'
    }
    foreach($taskIndex in 0..1){
        foreach($taskCase in 0..5){Require (Same $taskBaseline.lanes[$taskIndex].helpers[$taskCase].identity $taskCandidate.lanes[$taskIndex].helpers[$taskCase].identity) 'Helper payload/name changed'}
    }
    @{schema='authority-open-name-local-comparison-074-v1';baseline=$taskBaseline;candidate=$taskCandidate;rust_request_effect_confirmed=$true;directory_and_long_fallback_equal=$true;fresh_expression_counts_equal=$true;cpu_nonregression_established=$false;product_gain_claimed=$false;full_active_authority_measured=$false;existing_live_refusals_retained=$true;native_gate_open=$false;server=$false;product=$false}
}
function Inventory{
    @{schema='sha256-local-file-inventory-v1';files=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object{
        @{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}
    })}
}
if($Mode -ceq 'BaselineReview'){
    PreservedPreflight
    Prior
    $taskFacts=StageFacts 'Baseline'
    [ordered]@{baseline_valid=$true;source=$taskFacts.source;lanes=2;runtime_candidate_not_required=$true}|ConvertTo-Json
    exit
}
if($Mode -ceq 'Audit'){
    $taskRefusals=@()
    foreach($taskMutation in @('requests','scope','difference','order')){
        $taskRefused=$false
        try{StageFacts 'Baseline' $taskMutation|Out-Null}catch{$taskRefused=$true}
        Require $taskRefused "Mutation accepted: $taskMutation"
        $taskRefusals+=@{mutation=$taskMutation;refused=$true}
    }
    @{mutations=$taskRefusals;server=$false;product=$false}|ConvertTo-Json -Depth 6
    exit
}
$taskFacts=Facts
if($Mode -ceq 'WriteSummary'){
    Require (-not(Test-Path (Join-Path $PSScriptRoot 'summary.json')) -and -not(Test-Path (Join-Path $PSScriptRoot 'inventory.json'))) 'Never overwrite summary'
    $taskFacts|ConvertTo-Json -Depth 40|Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'summary.json')
}else{
    Require (Same $taskFacts (Json 'summary.json')) 'Summary differs from raw facts'
}
if($Mode -ceq 'Seal'){
    Require (-not(Test-Path (Join-Path $PSScriptRoot 'inventory.json'))) 'Never reseal'
    Inventory|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'inventory.json')
}
if($Mode -ceq 'Review'){
    Require (Same (Inventory) (Json 'inventory.json')) 'Inventory changed'
}
@{mode=$Mode;reviewed=$true;inventory_sha256=if(Test-Path (Join-Path $PSScriptRoot 'inventory.json')){Hash (Join-Path $PSScriptRoot 'inventory.json')}else{$null};server=$false;product=$false}|ConvertTo-Json
