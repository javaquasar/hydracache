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
    $taskStages=@()
    $taskCases=@('boot','namespace-user','namespace-mnt','namespace-pid','namespace-time','origin','authority-1','authority-7')
    $taskNames=@('0','9','10','99','100','12','-2147483648','2147483647')
    foreach($taskStage in @('Before','After')){
        $taskLanes=@()
        foreach($taskLane in @('194','188')){
            $taskReceipt=Json "$taskStage-$taskLane-exit.json"
            Require ($taskReceipt.stage -ceq $taskStage -and $taskReceipt.lane -ceq $taskLane -and $taskReceipt.source -cmatch '^[0-9a-f]{40}$') 'Profile identity differs'
            Require ($taskReceipt.image_sha256 -cmatch '^[0-9a-f]{64}$' -and $taskReceipt.image_sha256 -ceq $taskReceipt.image_after_sha256) 'Image changed'
            Require ($taskReceipt.steps.Count -eq 2 -and ($taskReceipt.steps.name -join ',') -ceq 'name,inspect' -and @($taskReceipt.steps|Where-Object exit -ne 0).Count -eq 0) 'Profile failed or incomplete'
            foreach($taskFlag in @('server','product','native_gate_open')){Require ($taskReceipt.$taskFlag -is [bool] -and -not $taskReceipt.$taskFlag) 'Scope flag differs or missing'}
            Require ($taskReceipt.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'profile.ps1'))) 'Profile recipe differs'
            Require ($taskReceipt.inputs.Count -eq 6 -and @($taskReceipt.inputs.path|Select-Object -Unique).Count -eq 6) 'Input set differs'
            foreach($taskInput in $taskReceipt.inputs){
                $taskBlob=(& git rev-parse "$($taskReceipt.source):$($taskInput.path)").Trim()
                Require ($LASTEXITCODE -eq 0 -and $taskBlob -ceq $taskInput.blob) 'Committed input differs'
                $taskText=((& git show "$($taskReceipt.source):$($taskInput.path)") -join "`n")+"`n"
                Require ($LASTEXITCODE -eq 0 -and (Text-Hash $taskText) -ceq $taskInput.normalized_sha256) 'Normalized input differs'
            }
            $taskNameTest='diagnostic_kernel_admission::origin::fixed_leaf_path_tests::allocations::authority_fd_path_alloc::authority_fd_path_allocation_attribution'
            $taskInspectTest='diagnostic_kernel_admission::origin::fixed_leaf_path_tests::allocations::validation_leaf_requests::validation_leaf_requests_attribution'
            $taskNameRows=@(Records "$taskStage-$taskLane-name.log" 'HC074-AUTHORITY-FD-PATH' $taskNameTest)
            $taskAlloc=@(Records "$taskStage-$taskLane-inspect.log" 'HC074-VALIDATION-LEAF-ALLOC' $taskInspectTest)
            $taskIo=@(Records "$taskStage-$taskLane-inspect.log" 'HC074-VALIDATION-LEAF-IO' $taskInspectTest)
            $taskOrigin=@(Records "$taskStage-$taskLane-inspect.log" 'HC074-VALIDATION-LEAF-ORIGIN' $taskInspectTest)
            Require ($taskNameRows.Count -eq 24 -and $taskAlloc.Count -eq 24 -and $taskIo.Count -eq 24 -and $taskOrigin.Count -eq 1) 'Incomplete observations'
            foreach($taskKind in @('name','inspect')){Require ((Get-Content (Join-Path $PSScriptRoot "$taskStage-$taskLane-$taskKind.log") -Raw).Contains('1 passed; 0 failed')) 'Native test incomplete'}
            Require ($taskOrigin[0].uid -gt 0 -and $taskOrigin[0].samples -eq 3 -and $taskOrigin[0].iterations -eq 1024 -and $taskOrigin[0].warmups -eq 32) 'Fixture origin differs'
            foreach($taskCase in $taskNames){
                $taskRows=@($taskNameRows|Where-Object case -CEQ $taskCase|Sort-Object sample)
                Require ($taskRows.Count -eq 3) 'Name case incomplete'
                $taskFd=[int]$taskCase
                for($taskSample=0;$taskSample -lt 3;$taskSample++){
                    $taskOne=$taskRows[$taskSample]
                    Require ($taskOne.fd -eq $taskFd -and $taskOne.sample -eq $taskSample -and $taskOne.iterations -eq 1024 -and $taskOne.warmups -eq 32) 'Comparison sample identity differs'
                    foreach($taskFlag in @('timing_claim','server','product')){Require ($taskOne.$taskFlag -is [bool] -and -not $taskOne.$taskFlag) 'Comparison scope differs'}
                    Require ($taskOne.rust_requests_only -is [bool] -and $taskOne.rust_requests_only) 'Comparison request scope absent'
                    Require (($taskOne.order -join ',') -ceq @('0,1','1,0','0,1')[$taskSample]) 'Arm alternation differs'
                    Require ((Same $taskOne.reference $taskRows[0].reference) -and (Same $taskOne.proposed $taskRows[0].proposed)) 'Comparison samples do not repeat'
                    Require ($taskOne.proposed.allocations -eq 0 -and $taskOne.proposed.reallocations -eq 0 -and $taskOne.proposed.requested_bytes -eq 0 -and $taskOne.reference.reallocations -eq 0) 'Unexpected isolated request cost'
                    Require ($taskOne.reference.allocations -gt 0 -and $taskOne.reference.requested_bytes -gt 0) 'No isolated path ownership saving'
                }
            }
            $taskInspect=@()
            foreach($taskCase in $taskCases){
                $taskA=@($taskAlloc|Where-Object case -CEQ $taskCase|Sort-Object sample)
                $taskI=@($taskIo|Where-Object case -CEQ $taskCase|Sort-Object sample)
                Require ($taskA.Count -eq 3 -and $taskI.Count -eq 3) 'Inspect case incomplete'
                for($taskSample=0;$taskSample -lt 3;$taskSample++){
                    $taskOne=$taskA[$taskSample];$taskTwo=$taskI[$taskSample]
                    Require ($taskOne.sample -eq $taskSample -and $taskTwo.sample -eq $taskSample -and $taskOne.iterations -eq 1024 -and $taskTwo.iterations -eq 1024) 'Inspect sample differs'
                    Require ($taskOne.rust_requests_only -and -not $taskTwo.allocator_active -and -not $taskTwo.all_syscalls_observed -and -not $taskOne.authority_fixed_ancestry -and -not $taskTwo.authority_fixed_ancestry -and -not $taskOne.server -and -not $taskTwo.server -and -not $taskOne.product -and -not $taskTwo.product) 'Inspect scope differs'
                    Require ($taskTwo.empty.syscr -eq 1 -and $taskTwo.empty.syscw -eq 0 -and $taskTwo.observed.syscw -eq 0 -and $taskTwo.observed.wchar -eq 0) 'Unexpected I/O'
                }
                $taskInspect+=[ordered]@{case=$taskCase;counts=@($taskA|ForEach-Object counts);read_requests=@($taskI|ForEach-Object {$_.observed.syscr-$_.empty.syscr});kernel_counts=@($taskI|ForEach-Object {,$_.kernel_calls})}
            }
            $taskLanes+=[ordered]@{lane=$taskLane;source=$taskReceipt.source;image_sha256=$taskReceipt.image_sha256;origin=$taskOrigin[0];names=$taskNameRows;inspect=$taskInspect}
        }
        Require ($taskLanes[0].source -ceq $taskLanes[1].source) 'Compiler sources differ'
        $taskStages+=[ordered]@{stage=$taskStage;lanes=$taskLanes}
    }
    $taskDeltas=@()
    for($taskIndex=0;$taskIndex -lt 2;$taskIndex++){
        $taskBefore=$taskStages[0].lanes[$taskIndex];$taskAfter=$taskStages[1].lanes[$taskIndex]
        $taskNameDeltas=[ordered]@{}
        foreach($taskCase in $taskNames){
            $taskB=@($taskBefore.names|Where-Object case -CEQ $taskCase|Sort-Object sample)
            $taskA=@($taskAfter.names|Where-Object case -CEQ $taskCase|Sort-Object sample)
            $taskDelta=[ordered]@{}
            for($taskSample=0;$taskSample -lt 3;$taskSample++){
                Require ((Same $taskB[$taskSample].reference $taskA[$taskSample].reference) -and (Same $taskB[$taskSample].proposed $taskA[$taskSample].proposed)) 'Frozen name reference/proposal changed'
                foreach($taskMetric in @('allocations','reallocations','requested_bytes')){
                    $taskValue=$taskB[$taskSample].reference.$taskMetric-$taskA[$taskSample].proposed.$taskMetric
                    Require ($taskValue -ge 0) 'Name metric regressed'
                    if($taskSample -eq 0){$taskDelta[$taskMetric]=$taskValue/1024}
                }
            }
            $taskNameDeltas[$taskCase]=$taskDelta
        }
        for($taskCaseIndex=0;$taskCaseIndex -lt 8;$taskCaseIndex++){
            $taskB=$taskBefore.inspect[$taskCaseIndex];$taskA=$taskAfter.inspect[$taskCaseIndex]
            Require ((Same $taskB.read_requests $taskA.read_requests) -and (Same $taskB.kernel_counts $taskA.kernel_counts)) 'Fresh read/kernel observations changed'
            for($taskSample=0;$taskSample -lt 3;$taskSample++){
                foreach($taskMetric in @('allocations','reallocations','requested_bytes')){
                    $taskExpected=0
                    if($taskCaseIndex -ge 6){
                        $taskExpected=1024 * 2 * $taskNameDeltas['12'].$taskMetric
                    }
                    Require (($taskB.counts[$taskSample].$taskMetric-$taskA.counts[$taskSample].$taskMetric) -eq $taskExpected) 'Complete inspect does not conserve two fd-path constructions'
                }
            }
        }
        foreach($taskFd in $taskNames){Require (Same $taskNameDeltas[$taskFd] $taskNameDeltas['12']) 'Descriptor formatting cost differs from conservation reference'}
        $taskDeltas+=[ordered]@{lane=$taskBefore.lane;isolated_per_operation=$taskNameDeltas}
    }
    foreach($taskStage in $taskStages){
        Require (Same $taskStage.lanes[0].names $taskStage.lanes[1].names) 'Compiler path controls differ'
        Require (Same $taskStage.lanes[0].inspect $taskStage.lanes[1].inspect) 'Compiler full controls differ'
    }
    foreach($taskCheckName in @('Before-check-194','Before-check-188','Before-check-xtask','Before-check-repository','After-check-194','After-check-188','After-check-xtask','After-check-contracts','After-check-repository')){
        $taskCheck=Json "$taskCheckName-exit.json"
        $taskCount=if($taskCheckName.Contains('-xtask')){3}elseif($taskCheckName.EndsWith('contracts')){1}elseif($taskCheckName.EndsWith('repository')){4}else{11}
        Require (-not $taskCheck.failure -and $taskCheck.steps.Count -eq $taskCount -and @($taskCheck.steps.name|Select-Object -Unique).Count -eq $taskCount -and @($taskCheck.steps|Where-Object exit -ne 0).Count -eq 0) 'Checks failed or incomplete'
        $taskCheckRecipe='check.ps1'
        Require ($taskCheck.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskCheckRecipe))) 'Check recipe differs'
        foreach($taskStep in $taskCheck.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'Check log changed'}
    }
    [ordered]@{schema='authority-fd-path-local-control-074-v1';complete_request_cohort=$true;allocation_reduction_conserved=$true;candidate_locally_accepted=$true;latency_gain_claimed=$false;all_syscalls_observed=$false;full_active_authority_measured=$false;existing_live_refusals_retained=$true;native_gate_open=$false;server=$false;product=$false;stages=$taskStages;deltas=$taskDeltas}
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
Require (Same $taskStored $taskFacts) 'Summary differs from raw observations'
$taskMutations=0
foreach($taskFlag in @('complete_request_cohort','allocation_reduction_conserved','candidate_locally_accepted','latency_gain_claimed','all_syscalls_observed','full_active_authority_measured','existing_live_refusals_retained','native_gate_open','server','product')){
    $taskChanged=($taskStored|ConvertTo-Json -Depth 20)|ConvertFrom-Json
    $taskChanged.$taskFlag=-not $taskChanged.$taskFlag
    Require (-not (Same $taskChanged $taskFacts)) 'Summary mutation accepted'
    $taskMutations++
}
$taskPrior=@(
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
Require (-not $taskFinal.failure -and $taskFinal.workload_and_qualification_equal -and $taskFinal.steps.Count -eq 3 -and @($taskFinal.steps|Where-Object exit -ne 0).Count -eq 0) 'Final checks incomplete'
Require ($taskFinal.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'final-check.ps1'))) 'Final recipe changed'
foreach($taskStep in $taskFinal.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'Final check log changed'}
if($Mode -ceq 'Seal'){
    $taskInventory=Join-Path $PSScriptRoot 'inventory.json'
    Require (-not (Test-Path $taskInventory)) 'Never reseal'
    [ordered]@{schema='sha256-local-file-inventory-v1';files=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse|Sort-Object FullName|ForEach-Object {[ordered]@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM $taskInventory
}
$taskMembers=Verify-Inventory $PSScriptRoot ''
[ordered]@{integrity_verified=$true;members=$taskMembers;prior_packets=15;summary_mutations_rejected=$taskMutations;complete_request_cohort=$true;allocation_reduction_conserved=$true;candidate_locally_accepted=$true;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
