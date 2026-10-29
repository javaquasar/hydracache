#requires -Version 7.2
param([ValidateSet('Seal','Review')][string]$Mode='Review',[string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/collector-peer-baseline-20261021'
function Need($Ok,[string]$Why){if(-not $Ok){throw $Why}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Path){Get-Content -LiteralPath $Path -Raw|ConvertFrom-Json}
function Text([string]$Path){(Get-Content -LiteralPath $Path -Raw).Replace("`r",'')}
function WriteNew([string]$Path,$Value){
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject $Value -Depth 20)+"`n")
    $taskStream=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
function Members([string]$Directory,$Seal){
    $taskSeen=@{}
    Need (@(Get-ChildItem -LiteralPath $Directory -Recurse -Force -Attributes ReparsePoint).Count -eq 0) 'Linked evidence'
    foreach($taskRow in $Seal.files){
        Need (-not[IO.Path]::IsPathRooted($taskRow.path) -and -not $taskRow.path.Contains([char]92) -and -not($taskRow.path.Split([char]47) -contains '..') -and -not $taskSeen.ContainsKey($taskRow.path)) 'Unsafe member'
        $taskSeen[$taskRow.path]=$true
        $taskItem=Get-Item -LiteralPath (Join-Path $Directory $taskRow.path)
        Need ($taskItem.Length -eq $taskRow.bytes -and (Hash $taskItem.FullName) -ceq $taskRow.sha256) 'Evidence member drift'
    }
    Need (@(Get-ChildItem -LiteralPath $Directory -Recurse -File).Count -eq $taskSeen.Count+1) 'Membership drift'
}
$taskPriorCount=0
foreach($taskPin in @(
    @('held-material-live-20261021','0c385fbfe209fad1903aa2c0e620817c458f413eec8bd36fa32be5c01fbb4109'),
    @('held-material-round-attribution-20261021','84c2d0a10e1d2d66b4dc76e1c041b9920d90d85621461bc3e83102569a6e1a2f'),
    @('pidfd-info-path-20261021','dc7af6cd78dfc1b69c7443078a8c6e2edc4504e294f74d30df93e085f8471daa'),
    @('projection-groups-20261021','ae7db79c3e579b7ad9f1939d2571aab27452b62f81c89b46b98340381f4b91c6'),
    @('reserved-leaf-audit-20261021','d66631eec23dc969f372d73c2bb83519058214383023564ec269ec542f854c4e'),
    @('context-document-baseline-20261021','8db3e4df5579ec7a471886505191825609b5c839732670bece6702ae101afcb6'),
    @('context-document-stack-20261021','7a65d50c19622754bda58fd389ef03d3f07254465d96f1e16b586cdec94f71f0'),
    @('trust-component-baseline-20261021','7eacd69041a722fc3d6153ef8be43eff87bf945a8fb6ccdd456acadff1bfcc9d'),
    @('trust-component-stack-20261021','e588fd8ae6b06a6e808446328b819240c4bb0e35955309e843ddb1078668fd85')
)){
    $taskPrior=Join-Path (Split-Path $taskPacket) $taskPin[0]
    Need ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq $taskPin[1]) 'Prior raw seal drift'
    $taskPriorSeal=Json (Join-Path $taskPrior 'inventory.json');Members $taskPrior $taskPriorSeal
    $taskPriorCount+=$taskPriorSeal.files.Count
}
Need ($taskPriorCount -eq 401) 'Prior member count drift'
if($Mode -ceq 'Review'){$Source=(Json (Join-Path $taskPacket 'identity.json')).baseline_source}
Need ($Source -match '^[a-f0-9]{40}$') 'Exact baseline source required'
& git diff --quiet $Source -- tools/long-run-supervisor-074 tools/collector-peer-baseline-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
Need ($LASTEXITCODE -eq 0) 'Baseline source drift'
$taskUntracked=@(& git ls-files --others --exclude-standard -- tools/long-run-supervisor-074 tools/collector-peer-baseline-074 crates)
Need ($LASTEXITCODE -eq 0 -and $taskUntracked.Count -eq 0) 'Untracked baseline source'
Need ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification drift'
foreach($taskName in @('check.ps1','review.ps1','README.md')){Need ((Hash "tools/collector-peer-baseline-074/$taskName") -ceq (Hash (Join-Path $taskPacket "method/$taskName"))) 'Recipe snapshot drift'}
foreach($taskPath in @(
    'tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/tests/support/collector_peer_baseline.rs',
    'crates/xtask/tests/collector_peer_baseline_074.rs',
    'crates/xtask/tests/support/collector_peer_baseline_guard.rs',
    'crates/xtask/tests/support/trust_component_stack_guard.rs'
)){Need ((Hash $taskPath) -ceq (Hash (Join-Path $taskPacket "source/$taskPath"))) 'Source snapshot drift'}
$taskFixed=Text 'tools/long-run-supervisor-074/tests/support/collector_peer_baseline.rs'
$taskUnfixed=$taskFixed.Replace('Vec<(usize, String, u32, Vec<u32>)>','Vec<(usize, &''static str, u32, Vec<u32>)>').Replace('[0_usize, 1, 2, 3, 4, 8, 16, 31, 32, 33, 64]','[0, 1, 2, 3, 4, 8, 16, 31, 32, 33, 64]')
foreach($taskLabel in @('empty','absent','present','duplicates')){$taskUnfixed=$taskUnfixed.Replace(('"'+$taskLabel+'".into()'),('"'+$taskLabel+'"'))}
Need ($taskUnfixed -ceq (Text (Join-Path $taskPacket 'initial-source/collector_peer_baseline.rs'))) 'Initial source differs beyond label/type repairs'
function Unit([string]$Path,[int]$Count){
    $taskText=Text (Join-Path $taskPacket $Path)
    Need ($taskText -match "test result: ok\. $Count passed; 0 failed; 0 ignored;" -and $taskText -notmatch 'test result: FAILED') "Unit refused: $Path"
}
foreach($taskLane in @('host','194','188')){
    foreach($taskCohort in @('first','corrected')){
        $taskResult=Json (Join-Path $taskPacket "$taskCohort/$taskLane-result.json")
        $taskFailed=$taskCohort -ceq 'first'
        $taskExpected=if($taskFailed){if($taskLane -ceq 'host'){'canaries'}else{'collector_peer_baseline_'}}elseif($taskLane -ceq 'host'){'canaries,check,clippy'}else{'collector_peer_baseline_,trust_component_stack_,context_document_stack_,fixed_leaf_alloc_counter,check,clippy'}
        Need ($taskResult.cohort -ceq $taskCohort -and $taskResult.lane -ceq $taskLane -and ($taskResult.steps.name -join ',') -ceq $taskExpected -and -not $taskResult.runtime_candidate_implemented -and -not $taskResult.ignored_tests_run -and $taskResult.original_owner_profiles -eq 0 -and -not $taskResult.server -and -not $taskResult.product) 'Scope/sequence drift'
        if($taskFailed){
            $taskName=if($taskLane -ceq 'host'){'canaries'}else{'collector_peer_baseline_'}
            Need ($taskResult.failure -ceq "Check refused: first/$taskLane/$taskName (101)") 'First refusal erased'
            $taskFailure=Text (Join-Path $taskPacket "first/$taskLane-$taskName.log")
            if($taskLane -ceq 'host'){Need ($taskFailure.Contains('unexpected capability static') -and $taskFailure.Contains('test result: FAILED. 2 passed; 1 failed;')) 'First canary detail drift'}
            else{Need ($taskFailure.Contains('error[E0689]') -and -not $taskFailure.Contains('test result: ok.')) 'First compile detail drift'}
        }else{Need ($null -eq $taskResult.failure) 'Corrected checks refused'}
        foreach($taskStep in $taskResult.steps){
            $taskExit=if($taskFailed){101}else{0}
            Need ($taskStep.log -ceq "$taskLane-$($taskStep.name).log" -and $taskStep.exit -eq $taskExit -and $taskStep.sha256 -ceq (Hash (Join-Path $taskPacket "$taskCohort/$($taskStep.log)"))) 'Check log drift'
        }
        if(-not $taskFailed){
            if($taskLane -ceq 'host'){
                $taskCanaries=Text (Join-Path $taskPacket 'corrected/host-canaries.log');$taskTotal=0
                foreach($taskMatch in [regex]::Matches($taskCanaries,'test result: ok\. (\d+) passed; 0 failed; 0 ignored;')){$taskTotal += [int]$taskMatch.Groups[1].Value}
                Need ($taskTotal -eq 34 -and $taskCanaries -notmatch 'test result: FAILED') 'Source canaries refused'
            }else{
                Unit "corrected/$taskLane-collector_peer_baseline_.log" 5
                Unit "corrected/$taskLane-trust_component_stack_.log" 5
                Unit "corrected/$taskLane-context_document_stack_.log" 4
                Unit "corrected/$taskLane-fixed_leaf_alloc_counter.log" 2
                $taskPure=Text (Join-Path $taskPacket "corrected/$taskLane-collector_peer_baseline_.log")
                Need ($taskPure.Contains('seed=0x074c0ee1 samples=1024 model_only=true') -and $taskPure.Contains('cases=32 model_only=true')) 'Pure model markers missing'
            }
        }
    }
}
function Rows([string]$Path,[string]$Prefix){@(Get-Content -LiteralPath $Path|Where-Object {$_.Contains($Prefix)}|ForEach-Object {($_ -split [regex]::Escape($Prefix),2)[1]|ConvertFrom-Json})}
$taskCases=@(@{length=0;shape='empty'})
foreach($taskLength in @(1,2,3,4,8,16,31,32,33,64)){foreach($taskShape in @('absent','present','duplicates')){$taskCases+=@{length=$taskLength;shape=$taskShape}}}
$taskLanes=@()
foreach($taskLane in @('194','188')){
    $taskLog=Join-Path $taskPacket "corrected/$taskLane-collector_peer_baseline_.log"
    $taskRows=Rows $taskLog 'HC074-COLLECTOR-PEER-BASELINE ';$taskStages=Rows $taskLog 'HC074-COLLECTOR-PEER-STAGES '
    Need ($taskRows.Count -eq 31 -and $taskStages.Count -eq 31) 'Missing baseline/stage cases'
    $taskAllocations=0L;$taskReallocations=0L;$taskBytes=0L
    for($taskIndex=0;$taskIndex -lt 31;$taskIndex++){
        $taskRow=$taskRows[$taskIndex];$taskStage=$taskStages[$taskIndex];$taskCase=$taskCases[$taskIndex]
        $taskLength=$taskCase.length;$taskShape=$taskCase.shape
        $taskPresent=$taskShape -ceq 'present' -or $taskShape -ceq 'duplicates'
        $taskFinal=if($taskShape -ceq 'duplicates'){1}elseif($taskShape -ceq 'present'){$taskLength}else{$taskLength+1}
        $taskRealloc=if($taskLength -eq 0){0}else{1}
        Need ($taskRow.length -eq $taskLength -and $taskRow.shape -ceq $taskShape -and $taskRow.primary_present -is [bool] -and $taskRow.primary_present -eq $taskPresent -and $taskRow.final_length -eq $taskFinal -and $taskRow.final_capacity -ge $taskLength+1 -and $taskRow.iterations -eq 256 -and $taskRow.rust_requests_only -eq $true -and -not $taskRow.timing_claim -and -not $taskRow.profile -and -not $taskRow.server -and -not $taskRow.product) 'Main corpus/scope drift'
        Need ($taskStage.length -eq $taskLength -and $taskStage.shape -ceq $taskShape -and $taskStage.constructions -eq 1 -and $taskStage.final_capacity -eq $taskRow.final_capacity -and $taskStage.model_only -eq $true -and -not $taskStage.timing_claim) 'Stage corpus/scope drift'
        $taskClone=if($taskLength -eq 0){0}else{1};$taskPush=1-$taskClone
        Need ($taskStage.clone.allocations -eq $taskClone -and $taskStage.clone.reallocations -eq 0 -and $taskStage.clone.requested_bytes -eq $taskLength*4 -and $taskStage.push.allocations -eq $taskPush -and $taskStage.push.reallocations -eq $taskRealloc -and $taskStage.push.requested_bytes -eq $taskRow.final_capacity*4 -and $taskStage.canonical.allocations -eq 0 -and $taskStage.canonical.reallocations -eq 0 -and $taskStage.canonical.requested_bytes -eq 0) 'Stage request attribution drift'
        foreach($taskField in @('allocations','reallocations','requested_bytes')){
            Need ($taskStage.complete.$taskField -eq $taskStage.clone.$taskField+$taskStage.push.$taskField+$taskStage.canonical.$taskField -and $taskRow.counts.$taskField -eq $taskStage.complete.$taskField*256) 'Stage/whole request conservation drift'
        }
        Need ($taskRow.counts.allocations -eq 256 -and $taskRow.counts.reallocations -eq $taskRealloc*256) 'Baseline request count drift'
        $taskAllocations+=$taskRow.counts.allocations;$taskReallocations+=$taskRow.counts.reallocations;$taskBytes+=$taskRow.counts.requested_bytes
    }
    $taskLanes+=[ordered]@{lane=$taskLane;cases=31;main_constructions=7936;allocations=$taskAllocations;reallocations=$taskReallocations;requested_bytes=$taskBytes;stage_constructions_separate=31;stage_sort_dedup_requests=0;union_seed='0x074c0ee1';union_cases=1024;synthetic_wrapper_cases=32;pure_baseline_controls=5;prior_pure_controls=9;counter_controls=2}
}
Need ((Get-Item (Join-Path $taskPacket 'final/format.log')).Length -eq 0) 'Format refused'
Need ((Text (Join-Path $taskPacket 'final/docs.log')) -match 'doc-check: OK') 'Docs refused'
Need ((Text (Join-Path $taskPacket 'final/registry.log')) -match 'OK \(186 discovered gates, all registered\)') 'Registry refused'
Need ((Text (Join-Path $taskPacket 'final/performance.log')) -match 'OK \(local, non-promotable\)') 'Performance contract refused'
$taskSummary=[ordered]@{baseline_source=$Source;lanes=$taskLanes;source_canaries=34;prior_members_verified=401;first_refusals_retained=$true;runtime_candidate_implemented=$false;timing_improvement_proven=$false;original_windows_seconds=@(300,60,10);first_refusal_ages_ns=@(10042390247,10010835739);new_original_owner_profiles=0;server=$false;product=$false}
$taskInventory=Join-Path $taskPacket 'inventory.json'
if($Mode -ceq 'Seal'){
    Need (-not(Test-Path $taskInventory)) 'Never replace baseline seal'
    WriteNew (Join-Path $taskPacket 'identity.json') @{baseline_source=$Source}
    WriteNew (Join-Path $taskPacket 'summary.json') $taskSummary
    $taskFiles=@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File|Sort-Object FullName|ForEach-Object {@{path=[IO.Path]::GetRelativePath($taskPacket,$_.FullName).Replace([char]92,[char]47);bytes=$_.Length;sha256=(Hash $_.FullName)}})
    WriteNew $taskInventory @{baseline_source=$Source;files=$taskFiles}
}
Need (($taskSummary|ConvertTo-Json -Depth 20 -Compress) -ceq ((Json (Join-Path $taskPacket 'summary.json'))|ConvertTo-Json -Depth 20 -Compress)) 'Derived summary drift'
$taskSeal=Json $taskInventory;Need ($taskSeal.baseline_source -ceq $Source) 'Baseline identity drift'
Members $taskPacket $taskSeal
Write-Output "Collector peer baseline verified: $($taskSeal.files.Count) members; inventory $(Hash $taskInventory); 401 prior members intact; no candidate or live replay."
