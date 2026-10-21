param([ValidateSet('WriteSummary','Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){
    [System.Text.Json.Nodes.JsonNode]::DeepEquals(
        [System.Text.Json.Nodes.JsonNode]::Parse(($A|ConvertTo-Json -Depth 30 -Compress)),
        [System.Text.Json.Nodes.JsonNode]::Parse(($B|ConvertTo-Json -Depth 30 -Compress)))
}
function Records([string]$Path,[string]$Marker){
    foreach($taskLine in Get-Content -LiteralPath $Path){
        $taskAt=$taskLine.IndexOf($Marker+' ',[StringComparison]::Ordinal)
        if($taskAt -ge 0){$taskLine.Substring($taskAt+$Marker.Length+1)|ConvertFrom-Json}
    }
}
function Inactive($Row,[string[]]$Flags){
    foreach($taskFlag in $Flags){Require ($Row.$taskFlag -is [bool] -and -not $Row.$taskFlag) "Unexpected scope: $taskFlag"}
}
function Verify-Inventory([string]$Directory,[string]$ExpectedHash){
    $taskFile=Join-Path $Directory 'inventory.json'
    if($ExpectedHash){Require ((Hash $taskFile) -ceq $ExpectedHash) 'Prior inventory changed'}
    $taskInventory=Get-Content -LiteralPath $taskFile -Raw|ConvertFrom-Json
    Require ($taskInventory.schema -ceq 'sha256-local-file-inventory-v1') 'Inventory schema changed'
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
function Facts{
    $taskOriginal=Json 'Capture-check-194-exit.json'
    Require ($taskOriginal.failure -ceq 'Failed lint (101)' -and $taskOriginal.steps.Count -eq 10 -and $taskOriginal.steps[-1].exit -eq 101) 'Original GNU lint refusal missing'
    Require ($taskOriginal.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'check.ps1'))) 'Original check recipe changed'
    foreach($taskStep in $taskOriginal.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'Original check log changed'}
    foreach($taskLane in @('194','188','xtask','contracts','repository')){
        $taskSuffix=if($taskLane -in @('194','188')){'-repair'}else{''}
        $taskRecipe=if($taskSuffix){'check-repair.ps1'}else{'check.ps1'}
        $taskCheck=Json "Capture-check-$taskLane$taskSuffix-exit.json"
        $taskCount=if($taskLane -ceq 'xtask'){3}elseif($taskLane -ceq 'contracts'){1}elseif($taskLane -ceq 'repository'){4}else{11}
        Require (-not $taskCheck.failure -and $taskCheck.steps.Count -eq $taskCount -and @($taskCheck.steps.name|Select-Object -Unique).Count -eq $taskCount -and @($taskCheck.steps|Where-Object exit -ne 0).Count -eq 0) 'Incomplete check lane'
        Require ($taskCheck.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskRecipe))) 'Check recipe changed'
        foreach($taskStep in $taskCheck.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'Check log changed'}
        Inactive $taskCheck @('server','product')
    }
    $taskPhases=@('full','directory','inventory','records','markers','bind','empty')
    $taskLanes=@()
    $taskSource=$null
    foreach($taskLane in @('194','188')){
        $taskReceipt=Json "Capture-$taskLane-exit.json"
        Require ($taskReceipt.stage -ceq 'Capture' -and $taskReceipt.lane -ceq $taskLane -and $taskReceipt.source -cmatch '^[0-9a-f]{40}$') 'Capture identity differs'
        if($taskSource){Require ($taskReceipt.source -ceq $taskSource) 'Compiler sources differ'}else{$taskSource=$taskReceipt.source}
        Require ($taskReceipt.image_sha256 -cmatch '^[0-9a-f]{64}$' -and $taskReceipt.image_sha256 -ceq $taskReceipt.image_after_sha256) 'Image changed'
        Require ($taskReceipt.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'profile-repair.ps1'))) 'Profile recipe changed'
        Require ($taskReceipt.steps.Count -eq 2 -and ($taskReceipt.steps.name -join ',') -ceq 'cpu,inspect' -and @($taskReceipt.steps|Where-Object exit -ne 0).Count -eq 0) 'Measured invocation failed or incomplete'
        Inactive $taskReceipt @('server','product','native_gate_open')
        Require ($taskReceipt.inputs.Count -eq 6 -and @($taskReceipt.inputs.path|Select-Object -Unique).Count -eq 6) 'Input set differs'
        foreach($taskInput in $taskReceipt.inputs){
            $taskBlob=(& git rev-parse "$($taskSource):$($taskInput.path)").Trim()
            Require ($LASTEXITCODE -eq 0 -and $taskBlob -ceq $taskInput.blob) 'Committed input changed'
            $taskText=((& git show "$($taskSource):$($taskInput.path)") -join "`n")+"`n"
            $taskHash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($taskText))).ToLowerInvariant()
            Require ($LASTEXITCODE -eq 0 -and $taskHash -ceq $taskInput.normalized_sha256) 'Input hash differs'
        }
        $taskLog=Join-Path $PSScriptRoot "Capture-$taskLane-cpu.log"
        $taskRows=@(Records $taskLog 'HC074-AUTHORITY-CPU')
        $taskOrigin=@(Records $taskLog 'HC074-AUTHORITY-CPU-ORIGIN')
        $taskFixtures=@(Records $taskLog 'HC074-AUTHORITY-CPU-FIXTURE')
        $taskResiduals=@(Records $taskLog 'HC074-AUTHORITY-CPU-RESIDUAL')
        Require ($taskRows.Count -eq 196 -and $taskOrigin.Count -eq 1 -and $taskFixtures.Count -eq 2 -and $taskResiduals.Count -eq 28) 'Incomplete CPU observations'
        $taskOrigin=$taskOrigin[0]
        Require ($taskOrigin.uid -gt 0 -and $taskOrigin.boot_id -cmatch '^[0-9a-f-]{36}$' -and $taskOrigin.clock -ceq 'CLOCK_THREAD_CPUTIME_ID' -and $taskOrigin.resolution_ns -gt 0 -and $taskOrigin.clock_pair_ns.Count -eq 64) 'CPU clock identity differs'
        Require ($taskOrigin.iterations -eq 2048 -and $taskOrigin.samples -eq 14 -and $taskOrigin.warmups -eq 32 -and @($taskOrigin.clock_pair_ns|Where-Object {$_ -lt 0}).Count -eq 0) 'Clock controls differ'
        Inactive $taskOrigin @('runtime_changed','server','product','native_gate_open')
        $taskOldRoot=Join-Path (Split-Path $PSScriptRoot) 'authority-owners-20261021'
        $taskOldFixtures=@(Records (Join-Path $taskOldRoot "Capture-$taskLane-breakdown.log") 'HC074-AUTHORITY-OWNERS-FIXTURE')
        $taskDepths=@()
        foreach($taskDepth in @(1,7)){
            $taskIdentity=@($taskFixtures|Where-Object generations -eq $taskDepth)
            $taskOldIdentity=@($taskOldFixtures|Where-Object generations -eq $taskDepth)
            Require ($taskIdentity.Count -eq 1 -and $taskOldIdentity.Count -eq 1) 'Missing fixture'
            $taskIdentity=$taskIdentity[0];$taskOldIdentity=$taskOldIdentity[0]
            foreach($taskField in @('generations','revision','epoch','root_name_bytes','entries')){
                Require (Same $taskIdentity.$taskField $taskOldIdentity.$taskField) 'Fixture payload or policy differs'
            }
            $taskStats=@()
            for($taskPhaseIndex=0;$taskPhaseIndex -lt 7;$taskPhaseIndex++){
                $taskPhase=$taskPhases[$taskPhaseIndex]
                $taskSamples=@($taskRows|Where-Object {$_.generations -eq $taskDepth -and $_.phase -ceq $taskPhase}|Sort-Object sample)
                Require ($taskSamples.Count -eq 14) 'Phase sample count differs'
                for($taskSample=0;$taskSample -lt 14;$taskSample++){
                    $taskRow=$taskSamples[$taskSample]
                    $taskPosition=if($taskSample -lt 7){($taskPhaseIndex-$taskSample+7)%7}else{($taskSample-7-$taskPhaseIndex+7)%7}
                    Require ($taskRow.sample -eq $taskSample -and $taskRow.position -eq $taskPosition -and $taskRow.iterations -eq 2048 -and $taskRow.cpu_ns -is [long] -and $taskRow.cpu_ns -gt 0) 'CPU workload or order differs'
                    Inactive $taskRow @('allocator_active','traced_methods','runtime_changed','server','product')
                }
                $taskValues=@($taskSamples|ForEach-Object {[double]$_.cpu_ns/2048}|Sort-Object)
                $taskStats+=@{phase=$taskPhase;min_ns_per_call=$taskValues[0];median_ns_per_call=($taskValues[6]+$taskValues[7])/2;max_ns_per_call=$taskValues[13];raw_batch_ns=@($taskSamples.cpu_ns)}
            }
            $taskRemainders=@()
            for($taskSample=0;$taskSample -lt 14;$taskSample++){
                $taskTotals=@(foreach($taskPhase in $taskPhases){@($taskRows|Where-Object {$_.generations -eq $taskDepth -and $_.sample -eq $taskSample -and $_.phase -ceq $taskPhase})[0].cpu_ns})
                $taskPart=2*[long]$taskTotals[1]+2*[long]$taskTotals[2]+[long]$taskTotals[3]+[long]$taskTotals[4]+[long]$taskTotals[5]
                $taskRest=@($taskResiduals|Where-Object {$_.generations -eq $taskDepth -and $_.sample -eq $taskSample})
                Require ($taskRest.Count -eq 1 -and $taskRest[0].full_ns -eq $taskTotals[0] -and $taskRest[0].components_ns -eq $taskPart -and $taskRest[0].signed_residual_ns -eq ([long]$taskTotals[0]-$taskPart) -and $taskRest[0].empty_ns -eq $taskTotals[6] -and $taskRest[0].iterations -eq 2048) 'Signed residual differs'
                $taskRemainders+=@{sample=$taskSample;signed_ns_per_call=[double]$taskRest[0].signed_residual_ns/2048}
            }
            $taskDepths+=@{generations=$taskDepth;identity=$taskIdentity;phases=$taskStats;residuals=$taskRemainders}
        }
        foreach($taskKind in @('cpu','inspect')){
            Require ((Get-Content (Join-Path $PSScriptRoot "Capture-$taskLane-$taskKind.log") -Raw).Contains('1 passed; 0 failed')) 'Manual test incomplete'
        }
        foreach($taskMarker in @('HC074-VALIDATION-LEAF-ALLOC','HC074-VALIDATION-LEAF-IO')){
            $taskNow=@(Records (Join-Path $PSScriptRoot "Capture-$taskLane-inspect.log") $taskMarker)
            $taskOld=@(Records (Join-Path $taskOldRoot "Capture-$taskLane-inspect.log") $taskMarker)
            Require ($taskNow.Count -eq 24 -and $taskOld.Count -eq 24) 'Frozen full control incomplete'
            for($taskIndex=0;$taskIndex -lt 24;$taskIndex++){
                $taskA=$taskNow[$taskIndex];$taskB=$taskOld[$taskIndex]
                foreach($taskField in @('case','sample','iterations','server','product','authority_fixed_ancestry')){Require (Same $taskA.$taskField $taskB.$taskField) 'Full control scope differs'}
                if($taskMarker.EndsWith('-ALLOC')){
                    Require (Same $taskA.counts $taskB.counts) 'Full allocation control changed'
                    Require (Same $taskA.rust_requests_only $taskB.rust_requests_only) 'Full request scope changed'
                }else{
                    Require (Same $taskA.kernel_calls $taskB.kernel_calls) 'Tracked kernel control changed'
                    foreach($taskField in @('kernel_iterations','allocator_active','all_syscalls_observed')){Require (Same $taskA.$taskField $taskB.$taskField) 'Full I/O scope changed'}
                    # The proc I/O snapshot text has variable-width counters; retain its
                    # raw rchar, but do not equate its own byte length across processes.
                    foreach($taskField in @('wchar','syscr','syscw')){
                        Require (($taskA.observed.$taskField-$taskA.empty.$taskField) -eq ($taskB.observed.$taskField-$taskB.empty.$taskField)) 'Full I/O deltas changed'
                    }
                }
            }
        }
        $taskLanes+=@{lane=$taskLane;origin=$taskOrigin;depths=$taskDepths;image_sha256=$taskReceipt.image_sha256}
    }
    Require ((Hash (Join-Path $taskRepo 'docs/testing/performance/0.74/qualification-manifest.toml')) -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Frozen qualification differs'
    [ordered]@{schema='authority-thread-cpu-attribution-v1';source=$taskSource;lanes=$taskLanes;full_control_equal=$true;runtime_changed=$false;improvement_claimed=$false;cpu_only=$true;overhead_subtracted=$false;existing_live_refusals_retained=$true;native_gate_open=$false;server=$false;product=$false}
}
$taskFacts=Facts
$taskSummary=Join-Path $PSScriptRoot 'summary.json'
if($Mode -ceq 'WriteSummary'){
    Require (-not (Test-Path $taskSummary)) 'Never overwrite summary'
    $taskFacts|ConvertTo-Json -Depth 30|Set-Content -Encoding utf8NoBOM $taskSummary
    exit
}
$taskStored=Json 'summary.json'
Require (Same $taskStored $taskFacts) 'Summary differs from raw facts'
$taskMutations=0
foreach($taskFlag in @('full_control_equal','runtime_changed','improvement_claimed','cpu_only','overhead_subtracted','existing_live_refusals_retained','native_gate_open','server','product')){
    $taskChanged=($taskStored|ConvertTo-Json -Depth 30)|ConvertFrom-Json
    $taskChanged.$taskFlag=-not $taskChanged.$taskFlag
    Require (-not (Same $taskChanged $taskFacts)) 'Scope mutation accepted'
    $taskMutations++
}
$taskReference=[ordered]@{a=1;b=@(2,3)}
Require (Same $taskReference ([ordered]@{b=@(2,3);a=1})) 'JSON object order differs'
foreach($taskChanged in @(@{a=2;b=@(2,3)},@{a='1';b=@(2,3)},@{a=1;b=@(3,2)},@{a=1},@{a=1;b=$null},@{a=1;b=@(2,3);c=0},@{A=1;b=@(2,3)},@{a=$true;b=@(2,3)})){
    Require (-not (Same $taskReference $taskChanged)) 'Structural JSON mutation accepted'
}
$taskPrior=@(
    @('authority-owners-20261021','9c561339ebcbb08f379121996306bf0e11a7264d5c5ac5dff7abc678d289fe17'),
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
if($Mode -ceq 'Seal'){
    $taskInventory=Join-Path $PSScriptRoot 'inventory.json'
    Require (-not (Test-Path $taskInventory)) 'Never reseal'
    [ordered]@{schema='sha256-local-file-inventory-v1';files=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse|Sort-Object FullName|ForEach-Object {[ordered]@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM $taskInventory
}
$taskMembers=Verify-Inventory $PSScriptRoot ''
[ordered]@{integrity_verified=$true;members=$taskMembers;prior_packets=17;scope_mutations_rejected=$taskMutations;structural_mutations_rejected=8;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
