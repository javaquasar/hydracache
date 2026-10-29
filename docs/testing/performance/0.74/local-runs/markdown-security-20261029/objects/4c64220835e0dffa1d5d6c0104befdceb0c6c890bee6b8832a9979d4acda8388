#requires -Version 7.2
param([ValidateSet('Seal','Review')][string]$Mode='Review',[string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/collector-peer-capacity-20261022'
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
    @('trust-component-stack-20261021','e588fd8ae6b06a6e808446328b819240c4bb0e35955309e843ddb1078668fd85'),
    @('collector-peer-baseline-20261021','cc19972e68a264b7b84ac48400f43e1b981d5a0e762ad7a9aee1a67548bf2452')
)){
    $taskPrior=Join-Path (Split-Path $taskPacket) $taskPin[0]
    Need ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq $taskPin[1]) 'Prior raw seal drift'
    $taskPriorSeal=Json (Join-Path $taskPrior 'inventory.json');Members $taskPrior $taskPriorSeal
    $taskPriorCount+=$taskPriorSeal.files.Count
}
Need ($taskPriorCount -eq 442) 'Prior member count drift'
if($Mode -ceq 'Review'){$Source=(Json (Join-Path $taskPacket 'identity.json')).candidate_source}
Need ($Source -match '^[a-f0-9]{40}$') 'Exact candidate source required'
& git diff --quiet $Source -- tools/long-run-supervisor-074 tools/collector-peer-capacity-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
Need ($LASTEXITCODE -eq 0) 'Candidate source drift'
$taskUntracked=@(& git ls-files --others --exclude-standard -- tools/long-run-supervisor-074 tools/collector-peer-capacity-074 crates)
Need ($LASTEXITCODE -eq 0 -and $taskUntracked.Count -eq 0) 'Untracked candidate source'
Need ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification drift'
foreach($taskName in @('check.ps1','review.ps1','README.md')){Need ((Hash "tools/collector-peer-capacity-074/$taskName") -ceq (Hash (Join-Path $taskPacket "method/$taskName"))) 'Recipe snapshot drift'}
foreach($taskPath in @(
    'tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/tests/support/collector_peer_capacity.rs',
    'tools/long-run-supervisor-074/tests/support/collector_peer_capacity_adapter.rs',
    'tools/long-run-supervisor-074/tests/support/collector_peer_baseline.rs',
    'crates/xtask/tests/collector_peer_capacity_074.rs',
    'crates/xtask/tests/collector_peer_baseline_074.rs',
    'crates/xtask/tests/trust_component_stack_074.rs',
    'crates/xtask/tests/trust_component_baseline_074.rs',
    'crates/xtask/tests/context_document_stack_074.rs',
    'crates/xtask/tests/context_document_baseline_074.rs',
    'crates/xtask/tests/projection_groups_074.rs',
    'crates/xtask/tests/pidfd_info_path_074.rs',
    'crates/xtask/tests/support/collector_peer_capacity_guard.rs',
    'crates/xtask/tests/support/collector_peer_baseline_guard.rs'
)){Need ((Hash $taskPath) -ceq (Hash (Join-Path $taskPacket "source/$taskPath"))) 'Source snapshot drift'}
function Unit([string]$Path,[int]$Count){
    $taskText=Text (Join-Path $taskPacket $Path)
    Need ($taskText -match "test result: ok\. $Count passed; 0 failed; 0 ignored;" -and $taskText -notmatch 'test result: FAILED') "Unit refused: $Path"
}
$taskFixed=Text 'crates/xtask/tests/collector_peer_capacity_074.rs'
$taskExpected='//! Pure test-only forwarding to the actual private peer helper; no live packet.\nuse super::*;\npub(crate) fn candidate_peer(uid: u32, gid: u32, supplementary: &[u32]) -> DiagnosticPeer {\n    peer(uid, gid, supplementary)\n}\n'
$taskOriginal='use super::*; pub(crate) fn candidate_peer(uid:u32,gid:u32,supplementary:&[u32])->DiagnosticPeer {peer(uid,gid,supplementary)}'
Need ($taskFixed.Replace($taskExpected,$taskOriginal) -ceq (Text (Join-Path $taskPacket 'initial-source/collector_peer_capacity_074.rs'))) 'Initial canary differs beyond exact expected-text repair'
$taskInitial=Json (Join-Path $taskPacket 'first/host-result.json')
Need ($taskInitial.cohort -ceq 'first' -and $taskInitial.lane -ceq 'host' -and $taskInitial.failure -ceq 'Check refused: first/host/canaries (101)' -and $taskInitial.steps.Count -eq 1 -and $taskInitial.steps[0].name -ceq 'canaries' -and $taskInitial.steps[0].exit -eq 101 -and $taskInitial.steps[0].log -ceq 'host-canaries.log' -and $taskInitial.steps[0].sha256 -ceq (Hash (Join-Path $taskPacket 'first/host-canaries.log')) -and $taskInitial.runtime_candidate_implemented -eq $true -and -not $taskInitial.ignored_tests_run -and $taskInitial.original_owner_profiles -eq 0 -and -not $taskInitial.server -and -not $taskInitial.product) 'Initial host outcome/scope drift'
$taskInitialLog=Text (Join-Path $taskPacket 'first/host-canaries.log')
Need ($taskInitialLog.Contains('test result: FAILED. 2 passed; 1 failed;') -and $taskInitialLog.Contains('doc =') -and $taskInitialLog.Contains('collector_peer_capacity_freezes_baseline_corpus_reference_and_complete_wrapper')) 'First canary detail drift'
foreach($taskLane in @('host','194','188')){
    $taskCohort=if($taskLane -ceq 'host'){'canary-fixed'}else{'first'}
    $taskResult=Json (Join-Path $taskPacket "$taskCohort/$taskLane-result.json")
    $taskExpected=if($taskLane -ceq 'host'){'canaries,check,clippy'}else{'collector_peer_capacity_,collector_peer_baseline_,trust_component_stack_,context_document_stack_,fixed_leaf_alloc_counter,check,clippy'}
    Need ($taskResult.cohort -ceq $taskCohort -and $taskResult.lane -ceq $taskLane -and $null -eq $taskResult.failure -and ($taskResult.steps.name -join ',') -ceq $taskExpected -and $taskResult.runtime_candidate_implemented -eq $true -and -not $taskResult.ignored_tests_run -and $taskResult.original_owner_profiles -eq 0 -and -not $taskResult.server -and -not $taskResult.product) 'Scope/sequence/outcome drift'
    foreach($taskStep in $taskResult.steps){Need ($taskStep.log -ceq "$taskLane-$($taskStep.name).log" -and $taskStep.exit -eq 0 -and $taskStep.sha256 -ceq (Hash (Join-Path $taskPacket "$taskCohort/$($taskStep.log)"))) 'Check log drift'}
    if($taskLane -ceq 'host'){
        $taskCanaries=Text (Join-Path $taskPacket 'canary-fixed/host-canaries.log');$taskTotal=0
        foreach($taskMatch in [regex]::Matches($taskCanaries,'test result: ok\. (\d+) passed; 0 failed; 0 ignored;')){$taskTotal += [int]$taskMatch.Groups[1].Value}
        Need ($taskTotal -eq 37 -and $taskCanaries -notmatch 'test result: FAILED') 'Source canaries refused'
    }else{
        Unit "first/$taskLane-collector_peer_capacity_.log" 5
        Unit "first/$taskLane-collector_peer_baseline_.log" 5
        Unit "first/$taskLane-trust_component_stack_.log" 5
        Unit "first/$taskLane-context_document_stack_.log" 4
        Unit "first/$taskLane-fixed_leaf_alloc_counter.log" 2
        $taskPure=Text (Join-Path $taskPacket "first/$taskLane-collector_peer_capacity_.log")
        Need ($taskPure.Contains('seed=0x074c0ee1 samples=1024 model_only=true') -and $taskPure.Contains('cases=32 arms=2 model_only=true')) 'Pure comparison markers missing'
    }
}
function Rows([string]$Path,[string]$Prefix){@(Get-Content -LiteralPath $Path|Where-Object {$_.Contains($Prefix)}|ForEach-Object {($_ -split [regex]::Escape($Prefix),2)[1]|ConvertFrom-Json})}
$taskLanes=@()
foreach($taskLane in @('194','188')){
    $taskLog=Join-Path $taskPacket "first/$taskLane-collector_peer_capacity_.log"
    $taskOldLog=Join-Path $taskPacket "first/$taskLane-collector_peer_baseline_.log"
    $taskPriorLog=Join-Path (Split-Path $taskPacket) "collector-peer-baseline-20261021/corrected/$taskLane-collector_peer_baseline_.log"
    $taskRows=Rows $taskLog 'HC074-COLLECTOR-PEER-CAPACITY ';$taskStages=Rows $taskLog 'HC074-COLLECTOR-PEER-CAPACITY-STAGES '
    $taskOld=Rows $taskOldLog 'HC074-COLLECTOR-PEER-BASELINE ';$taskPrior=Rows $taskPriorLog 'HC074-COLLECTOR-PEER-BASELINE '
    $taskOldStages=Rows $taskOldLog 'HC074-COLLECTOR-PEER-STAGES ';$taskPriorStages=Rows $taskPriorLog 'HC074-COLLECTOR-PEER-STAGES '
    Need ($taskRows.Count -eq 31 -and $taskStages.Count -eq 31 -and $taskOld.Count -eq 31 -and $taskPrior.Count -eq 31 -and $taskOldStages.Count -eq 31 -and $taskPriorStages.Count -eq 31) 'Missing comparison cases'
    $taskReferenceAllocations=0L;$taskReferenceReallocations=0L;$taskReferenceBytes=0L;$taskAllocations=0L;$taskReallocations=0L;$taskBytes=0L
    for($taskIndex=0;$taskIndex -lt 31;$taskIndex++){
        $taskRow=$taskRows[$taskIndex];$taskStage=$taskStages[$taskIndex];$taskRef=$taskOld[$taskIndex]
        Need (($taskRef|ConvertTo-Json -Depth 8 -Compress) -ceq ($taskPrior[$taskIndex]|ConvertTo-Json -Depth 8 -Compress)) 'Frozen baseline main drift'
        Need (($taskOldStages[$taskIndex]|ConvertTo-Json -Depth 8 -Compress) -ceq ($taskPriorStages[$taskIndex]|ConvertTo-Json -Depth 8 -Compress)) 'Frozen baseline stages drift'
        Need ($taskRow.length -eq $taskRef.length -and $taskRow.shape -ceq $taskRef.shape -and $taskRow.primary_present -is [bool] -and $taskRow.primary_present -eq $taskRef.primary_present -and $taskRow.final_length -eq $taskRef.final_length -and $taskRow.final_capacity -eq $taskRef.length+1 -and $taskRow.iterations -eq 256 -and $taskRow.rust_requests_only -eq $true -and -not $taskRow.timing_claim -and -not $taskRow.profile -and -not $taskRow.server -and -not $taskRow.product) 'Candidate corpus/scope drift'
        Need ($taskRow.counts.allocations -eq 256 -and $taskRow.counts.reallocations -eq 0 -and $taskRow.counts.requested_bytes -eq ($taskRef.length+1)*4*256) 'Candidate request drift'
        Need ($taskStage.length -eq $taskRef.length -and $taskStage.shape -ceq $taskRef.shape -and $taskStage.constructions -eq 1 -and $taskStage.final_capacity -eq $taskRow.final_capacity -and $taskStage.model_only -eq $true -and -not $taskStage.timing_claim) 'Stage corpus/scope drift'
        Need ($taskStage.reserve.allocations -eq 1 -and $taskStage.reserve.reallocations -eq 0 -and $taskStage.reserve.requested_bytes -eq ($taskRef.length+1)*4) 'Reserve requests differ'
        foreach($taskKind in @('copy','push','canonical')){foreach($taskField in @('allocations','reallocations','requested_bytes')){Need ($taskStage.$taskKind.$taskField -eq 0) 'Unexpected post-reserve request'}}
        foreach($taskField in @('allocations','reallocations','requested_bytes')){Need ($taskStage.complete.$taskField -eq $taskStage.reserve.$taskField -and $taskRow.counts.$taskField -eq $taskStage.complete.$taskField*256) 'Stage/whole request conservation drift'}
        $taskReferenceAllocations+=$taskRef.counts.allocations;$taskReferenceReallocations+=$taskRef.counts.reallocations;$taskReferenceBytes+=$taskRef.counts.requested_bytes
        $taskAllocations+=$taskRow.counts.allocations;$taskReallocations+=$taskRow.counts.reallocations;$taskBytes+=$taskRow.counts.requested_bytes
    }
    $taskLanes+=[ordered]@{lane=$taskLane;cases=31;main_constructions=7936;reference_allocations=$taskReferenceAllocations;reference_reallocations=$taskReferenceReallocations;reference_requested_bytes=$taskReferenceBytes;candidate_allocations=$taskAllocations;candidate_reallocations=$taskReallocations;candidate_requested_bytes=$taskBytes;stage_constructions_per_arm_separate=31;union_seed='0x074c0ee1';union_cases=1024;synthetic_wrapper_cases=32;synthetic_wrapper_arms=2;pure_candidate_controls=5;unchanged_baseline_controls=5;prior_pure_controls=9;counter_controls=2}
}
Need ((Get-Item (Join-Path $taskPacket 'final/format.log')).Length -eq 0) 'Format refused'
Need ((Text (Join-Path $taskPacket 'final/docs.log')) -match 'doc-check: OK') 'Docs refused'
Need ((Text (Join-Path $taskPacket 'final/registry.log')) -match 'OK \(186 discovered gates, all registered\)') 'Registry refused'
Need ((Text (Join-Path $taskPacket 'final/performance.log')) -match 'OK \(local, non-promotable\)') 'Performance contract refused'
$taskSummary=[ordered]@{candidate_source=$Source;baseline_source='da44628a85779977d5d1cb2f32e324239165eeff';lanes=$taskLanes;source_canaries=37;prior_members_verified=442;first_refusals_retained=$true;runtime_candidate_implemented=$true;timing_improvement_proven=$false;original_windows_seconds=@(300,60,10);first_refusal_ages_ns=@(10042390247,10010835739);new_original_owner_profiles=0;server=$false;product=$false}
$taskInventory=Join-Path $taskPacket 'inventory.json'
if($Mode -ceq 'Seal'){
    Need (-not(Test-Path $taskInventory)) 'Never replace candidate seal'
    WriteNew (Join-Path $taskPacket 'identity.json') @{candidate_source=$Source}
    WriteNew (Join-Path $taskPacket 'summary.json') $taskSummary
    $taskFiles=@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File|Sort-Object FullName|ForEach-Object {@{path=[IO.Path]::GetRelativePath($taskPacket,$_.FullName).Replace([char]92,[char]47);bytes=$_.Length;sha256=(Hash $_.FullName)}})
    WriteNew $taskInventory @{candidate_source=$Source;files=$taskFiles}
}
Need (($taskSummary|ConvertTo-Json -Depth 20 -Compress) -ceq ((Json (Join-Path $taskPacket 'summary.json'))|ConvertTo-Json -Depth 20 -Compress)) 'Derived summary drift'
$taskSeal=Json $taskInventory;Need ($taskSeal.candidate_source -ceq $Source) 'Candidate identity drift'
Members $taskPacket $taskSeal
Write-Output "Collector peer capacity verified: $($taskSeal.files.Count) members; inventory $(Hash $taskInventory); 442 prior members intact; no live replay or timing claim."
