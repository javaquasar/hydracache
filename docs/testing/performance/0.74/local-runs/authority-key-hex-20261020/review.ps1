param([ValidateSet('WriteSummary','Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Text-Hash([string]$Text){[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){($A|ConvertTo-Json -Depth 20 -Compress) -ceq ($B|ConvertTo-Json -Depth 20 -Compress)}
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
    foreach($taskStage in @('Before','After')){
        $taskLanes=@()
        foreach($taskLane in @('194','188')){
            $taskReceipt=Json "$taskStage-$taskLane-exit.json"
            Require ($taskReceipt.stage -ceq $taskStage -and $taskReceipt.lane -ceq $taskLane -and $taskReceipt.source -cmatch '^[0-9a-f]{40}$') 'Profile identity differs'
            Require ($taskReceipt.image_sha256 -cmatch '^[0-9a-f]{64}$' -and $taskReceipt.image_sha256 -ceq $taskReceipt.image_after_sha256) 'Image changed'
            Require ($taskReceipt.steps.Count -eq 2 -and ($taskReceipt.steps.name -join ',') -ceq 'hex,inspect' -and @($taskReceipt.steps|Where-Object exit -ne 0).Count -eq 0) 'Profile failed or incomplete'
            foreach($taskFlag in @('server','product','native_gate_open')){Require ($taskReceipt.$taskFlag -is [bool] -and -not $taskReceipt.$taskFlag) 'Scope flag differs or missing'}
            Require ($taskReceipt.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'profile.ps1'))) 'Profile recipe differs'
            Require ($taskReceipt.inputs.Count -eq 6 -and @($taskReceipt.inputs.path|Select-Object -Unique).Count -eq 6) 'Input set differs'
            foreach($taskInput in $taskReceipt.inputs){
                $taskBlob=(& git rev-parse "$($taskReceipt.source):$($taskInput.path)").Trim()
                Require ($LASTEXITCODE -eq 0 -and $taskBlob -ceq $taskInput.blob) 'Committed input differs'
                $taskText=((& git show "$($taskReceipt.source):$($taskInput.path)") -join "`n")+"`n"
                Require ($LASTEXITCODE -eq 0 -and (Text-Hash $taskText) -ceq $taskInput.normalized_sha256) 'Normalized input differs'
            }
            $taskHexTest='diagnostic_kernel_admission::origin::fixed_leaf_path_tests::allocations::authority_hex_alloc::authority_hex_allocation_attribution'
            $taskInspectTest='diagnostic_kernel_admission::origin::fixed_leaf_path_tests::allocations::validation_leaf_requests::validation_leaf_requests_attribution'
            $taskHex=@(Records "$taskStage-$taskLane-hex.log" 'HC074-AUTHORITY-HEX' $taskHexTest|Sort-Object sample)
            $taskAlloc=@(Records "$taskStage-$taskLane-inspect.log" 'HC074-VALIDATION-LEAF-ALLOC' $taskInspectTest)
            $taskIo=@(Records "$taskStage-$taskLane-inspect.log" 'HC074-VALIDATION-LEAF-IO' $taskInspectTest)
            $taskOrigin=@(Records "$taskStage-$taskLane-inspect.log" 'HC074-VALIDATION-LEAF-ORIGIN' $taskInspectTest)
            Require ($taskHex.Count -eq 3 -and $taskAlloc.Count -eq 24 -and $taskIo.Count -eq 24 -and $taskOrigin.Count -eq 1) 'Incomplete observations'
            foreach($taskKind in @('hex','inspect')){Require ((Get-Content (Join-Path $PSScriptRoot "$taskStage-$taskLane-$taskKind.log") -Raw).Contains('1 passed; 0 failed')) 'Native test incomplete'}
            Require ($taskOrigin[0].uid -gt 0 -and $taskOrigin[0].samples -eq 3 -and $taskOrigin[0].iterations -eq 1024 -and $taskOrigin[0].warmups -eq 32) 'Fixture origin differs'
            for($taskSample=0;$taskSample -lt 3;$taskSample++){
                $taskOne=$taskHex[$taskSample]
                Require ($taskOne.sample -eq $taskSample -and $taskOne.iterations -eq 1024 -and $taskOne.warmups -eq 32 -and $taskOne.rust_requests_only -and -not $taskOne.timing_claim -and -not $taskOne.server -and -not $taskOne.product) 'Leaf sample differs'
                Require (Same $taskOne.actual $(if($taskStage -ceq 'Before'){$taskOne.reference}else{$taskOne.proposed})) 'Actual arm does not match stage'
            }
            $taskRows=@()
            foreach($taskCase in $taskCases){
                $taskA=@($taskAlloc|Where-Object case -CEQ $taskCase|Sort-Object sample)
                $taskI=@($taskIo|Where-Object case -CEQ $taskCase|Sort-Object sample)
                Require ($taskA.Count -eq 3 -and $taskI.Count -eq 3) 'Case incomplete'
                for($taskSample=0;$taskSample -lt 3;$taskSample++){
                    $taskOne=$taskA[$taskSample];$taskTwo=$taskI[$taskSample]
                    Require ($taskOne.sample -eq $taskSample -and $taskTwo.sample -eq $taskSample -and $taskOne.iterations -eq 1024 -and $taskTwo.iterations -eq 1024) 'Inspect sample differs'
                    Require ($taskOne.rust_requests_only -and -not $taskTwo.allocator_active -and -not $taskTwo.all_syscalls_observed -and -not $taskOne.authority_fixed_ancestry -and -not $taskTwo.authority_fixed_ancestry -and -not $taskOne.server -and -not $taskTwo.server -and -not $taskOne.product -and -not $taskTwo.product) 'Inspect scope differs'
                    Require ($taskTwo.empty.syscr -eq 1 -and $taskTwo.empty.syscw -eq 0 -and $taskTwo.observed.syscw -eq 0 -and $taskTwo.observed.wchar -eq 0) 'Unexpected I/O'
                }
                $taskRows+=[ordered]@{case=$taskCase;counts=@($taskA|ForEach-Object counts);read_requests=@($taskI|ForEach-Object {$_.observed.syscr-$_.empty.syscr});kernel_counts=@($taskI|ForEach-Object {,$_.kernel_calls})}
            }
            $taskLanes+=[ordered]@{lane=$taskLane;source=$taskReceipt.source;image_sha256=$taskReceipt.image_sha256;origin=$taskOrigin[0];hex=$taskHex;inspect=$taskRows}
        }
        Require ($taskLanes[0].source -ceq $taskLanes[1].source) 'Compiler sources differ'
        $taskStages+=[ordered]@{stage=$taskStage;lanes=$taskLanes}
    }
    $taskDeltas=@()
    for($taskLaneIndex=0;$taskLaneIndex -lt 2;$taskLaneIndex++){
        $taskBefore=$taskStages[0].lanes[$taskLaneIndex];$taskAfter=$taskStages[1].lanes[$taskLaneIndex]
        $taskDelta=[ordered]@{}
        for($taskSample=0;$taskSample -lt 3;$taskSample++){
            Require (Same $taskBefore.hex[$taskSample].reference $taskAfter.hex[$taskSample].reference) 'Frozen reference changed'
            Require (Same $taskBefore.hex[$taskSample].proposed $taskAfter.hex[$taskSample].proposed) 'Proposed leaf changed'
            foreach($taskMetric in @('allocations','reallocations','requested_bytes')){
                $taskReduction=$taskBefore.hex[$taskSample].reference.$taskMetric-$taskBefore.hex[$taskSample].proposed.$taskMetric
                Require ($taskReduction -gt 0 -and $taskReduction % 1024 -eq 0) 'No complete positive leaf reduction'
                if($taskSample -eq 0){$taskDelta[$taskMetric]=$taskReduction/1024}else{Require ($taskDelta[$taskMetric] -eq $taskReduction/1024) 'Leaf reduction varies'}
                for($taskCaseIndex=0;$taskCaseIndex -lt 8;$taskCaseIndex++){
                    $taskB=$taskBefore.inspect[$taskCaseIndex];$taskA=$taskAfter.inspect[$taskCaseIndex]
                    $taskExpected=if($taskCaseIndex -ge 6){$taskReduction}else{0}
                    Require (($taskB.counts[$taskSample].$taskMetric-$taskA.counts[$taskSample].$taskMetric) -eq $taskExpected) 'Complete inspect does not conserve isolated leaf reduction'
                    Require (Same $taskB.read_requests $taskA.read_requests) 'Read requests changed'
                    Require (Same $taskB.kernel_counts $taskA.kernel_counts) 'Fresh kernel observations changed'
                }
            }
        }
        $taskDeltas+=[ordered]@{lane=$taskBefore.lane;per_operation=$taskDelta}
    }
    foreach($taskCheckName in @('Before-check-194','Before-check-188','Before-check-xtask','After-check-194','After-check-188','After-check-xtask','After-check-contracts','After-check-repository')){
        $taskCheck=Json "$taskCheckName-exit.json"
        $taskCount=if($taskCheckName.EndsWith('xtask')){3}elseif($taskCheckName.EndsWith('contracts')){1}elseif($taskCheckName.EndsWith('repository')){4}else{14}
        Require (-not $taskCheck.failure -and $taskCheck.steps.Count -eq $taskCount -and @($taskCheck.steps.name|Select-Object -Unique).Count -eq $taskCount -and @($taskCheck.steps|Where-Object exit -ne 0).Count -eq 0) 'Checks failed or incomplete'
        Require ($taskCheck.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'check.ps1'))) 'Check recipe differs'
        foreach($taskStep in $taskCheck.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'Check log changed'}
    }
    [ordered]@{schema='authority-key-hex-local-control-074-v1';complete_request_cohort=$true;allocation_reduction_conserved=$true;candidate_locally_accepted=$true;latency_gain_claimed=$false;all_syscalls_observed=$false;full_active_authority_measured=$false;existing_live_refusals_retained=$true;native_gate_open=$false;server=$false;product=$false;stages=$taskStages;deltas=$taskDeltas}
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
if($Mode -ceq 'Seal'){
    $taskInventory=Join-Path $PSScriptRoot 'inventory.json'
    Require (-not (Test-Path $taskInventory)) 'Never reseal'
    [ordered]@{schema='sha256-local-file-inventory-v1';files=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse|Sort-Object FullName|ForEach-Object {[ordered]@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM $taskInventory
}
$taskMembers=Verify-Inventory $PSScriptRoot ''
[ordered]@{integrity_verified=$true;members=$taskMembers;prior_packets=8;summary_mutations_rejected=$taskMutations;complete_request_cohort=$true;allocation_reduction_conserved=$true;candidate_locally_accepted=$true;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
