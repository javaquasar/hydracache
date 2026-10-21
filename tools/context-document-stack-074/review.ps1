#requires -Version 7.2
param([ValidateSet('Seal','Review')][string]$Mode='Review',[string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/context-document-stack-20261021'
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
    @('context-document-baseline-20261021','8db3e4df5579ec7a471886505191825609b5c839732670bece6702ae101afcb6')
)){
    $taskPrior=Join-Path (Split-Path $taskPacket) $taskPin[0]
    Need ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq $taskPin[1]) 'Prior raw seal drift'
    $taskPriorSeal=Json (Join-Path $taskPrior 'inventory.json')
    Members $taskPrior $taskPriorSeal
    $taskPriorCount+=$taskPriorSeal.files.Count
}
Need ($taskPriorCount -eq 249) 'Prior member count drift'
if($Mode -ceq 'Review'){$Source=(Json (Join-Path $taskPacket 'identity.json')).candidate_source}
Need ($Source -match '^[a-f0-9]{40}$') 'Exact candidate source required'
& git diff --quiet $Source -- tools/long-run-supervisor-074 tools/context-document-stack-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
Need ($LASTEXITCODE -eq 0) 'Candidate source drift'
$taskUntracked=@(& git ls-files --others --exclude-standard -- tools/long-run-supervisor-074 tools/context-document-stack-074 crates)
Need ($LASTEXITCODE -eq 0 -and $taskUntracked.Count -eq 0) 'Untracked candidate source'
Need ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification drift'
foreach($taskName in @('check.ps1','review.ps1','README.md')){
    Need ((Hash "tools/context-document-stack-074/$taskName") -ceq (Hash (Join-Path $taskPacket "method/$taskName"))) 'Recipe snapshot drift'
}
foreach($taskPath in @(
    'tools/long-run-supervisor-074/src/diagnostic_context_document_buffer.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_context.rs',
    'tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs',
    'tools/long-run-supervisor-074/tests/support/context_document_stack.rs',
    'tools/long-run-supervisor-074/tests/support/context_document_stack_adapter.rs',
    'tools/long-run-supervisor-074/tests/support/context_document_buffer_baseline.rs',
    'crates/xtask/tests/context_document_stack_074.rs',
    'crates/xtask/tests/context_document_baseline_074.rs',
    'crates/xtask/tests/projection_groups_074.rs',
    'crates/xtask/tests/pidfd_info_path_074.rs',
    'crates/xtask/tests/support/context_document_stack_guard.rs',
    'crates/xtask/tests/support/context_document_baseline_guard.rs'
)){
    Need ((Hash $taskPath) -ceq (Hash (Join-Path $taskPacket "source/$taskPath"))) 'Source snapshot drift'
}
$taskFixed=Text 'tools/long-run-supervisor-074/tests/support/context_document_stack.rs'
$taskInitial=$taskFixed.Replace('fault.is_multiple_of(2)','fault % 2 == 0').Replace('(seed >> 32).is_multiple_of(3)','(seed >> 32) % 3 == 0')
Need ($taskInitial -ceq (Text (Join-Path $taskPacket 'initial-source/context_document_stack.rs'))) 'Initial source differs beyond the two test-only lint fixes'
function Unit([string]$Path,[int]$Count){
    $taskText=Text (Join-Path $taskPacket $Path)
    Need ($taskText -match "test result: ok\. $Count passed; 0 failed; 0 ignored;" -and $taskText -notmatch 'test result: FAILED') "Unit refused: $Path"
}
foreach($taskCohort in @('first','corrected')){
    foreach($taskLane in @('host','194','188')){
        $taskResult=Json (Join-Path $taskPacket "$taskCohort/$taskLane-result.json")
        $taskExpected=if($taskLane -ceq 'host'){'canaries,check,clippy'}else{'context_document_stack_,context_document_buffer_baseline,fixed_leaf_alloc_counter,check,clippy'}
        $taskFailed=$taskCohort -ceq 'first' -and $taskLane -ceq '194'
        Need ($taskResult.cohort -ceq $taskCohort -and $taskResult.lane -ceq $taskLane -and ($taskResult.steps.name -join ',') -ceq $taskExpected -and -not $taskResult.ignored_tests_run -and $taskResult.original_owner_profiles -eq 0 -and -not $taskResult.server -and -not $taskResult.product) 'Scope/sequence drift'
        if($taskFailed){Need ($taskResult.failure -ceq 'Check refused: first/194/clippy (101)') 'First lint refusal erased'}
        else{Need ($null -eq $taskResult.failure) 'Corrected check refused'}
        foreach($taskStep in $taskResult.steps){
            $taskExit=if($taskFailed -and $taskStep.name -ceq 'clippy'){101}else{0}
            Need ($taskStep.log -ceq "$taskLane-$($taskStep.name).log" -and $taskStep.exit -eq $taskExit -and $taskStep.sha256 -ceq (Hash (Join-Path $taskPacket "$taskCohort/$($taskStep.log)"))) 'Check log drift'
        }
        if($taskLane -cne 'host'){
            Unit "$taskCohort/$taskLane-context_document_stack_.log" 4
            Unit "$taskCohort/$taskLane-context_document_buffer_baseline.log" 5
            Unit "$taskCohort/$taskLane-fixed_leaf_alloc_counter.log" 2
            Need ((Text (Join-Path $taskPacket "$taskCohort/$taskLane-context_document_stack_.log")) -match 'seed=0x074c5a19 samples=384 model_only=true') 'Seeded differential missing'
        }else{
            $taskCanaries=Text (Join-Path $taskPacket "$taskCohort/host-canaries.log");$taskTotal=0
            foreach($taskMatch in [regex]::Matches($taskCanaries,'test result: ok\. (\d+) passed; 0 failed; 0 ignored;')){$taskTotal += [int]$taskMatch.Groups[1].Value}
            Need ($taskTotal -eq 25 -and $taskCanaries -notmatch 'test result: FAILED') 'Source canaries refused'
        }
    }
}
$taskLint=Text (Join-Path $taskPacket 'first/194-clippy.log')
Need ([regex]::Matches($taskLint,'error: manual implementation of').Count -eq 2 -and $taskLint.Contains('manual_is_multiple_of')) 'First lint detail drift'
function Rows([string]$Path,[string]$Prefix){
    @(Get-Content -LiteralPath $Path|Where-Object {$_.Contains($Prefix)}|ForEach-Object {($_ -split [regex]::Escape($Prefix),2)[1]|ConvertFrom-Json})
}
$taskLanes=@();$taskLengths=@(0,1,4,32,33,36,37,38,63)
foreach($taskLane in @('194','188')){
    $taskReference=Rows (Join-Path (Split-Path $taskPacket) "context-document-baseline-20261021/corrected/$taskLane-context_document_buffer_baseline.log") 'HC074-CONTEXT-DOCUMENT-BASELINE '
    foreach($taskCohort in @('first','corrected')){
        $taskOld=Rows (Join-Path $taskPacket "$taskCohort/$taskLane-context_document_buffer_baseline.log") 'HC074-CONTEXT-DOCUMENT-BASELINE '
        $taskNew=Rows (Join-Path $taskPacket "$taskCohort/$taskLane-context_document_stack_.log") 'HC074-CONTEXT-DOCUMENT-CANDIDATE '
        Need ($taskOld.Count -eq 9 -and $taskNew.Count -eq 9 -and $taskReference.Count -eq 9) 'Missing allocation cases'
        $taskAllocations=0L;$taskBytes=0L;$taskProductionAllocations=0L;$taskProductionBytes=0L
        for($taskIndex=0;$taskIndex -lt 9;$taskIndex++){
            $taskRow=$taskNew[$taskIndex];$taskLength=$taskLengths[$taskIndex]
            Need (($taskOld[$taskIndex]|ConvertTo-Json -Depth 8 -Compress) -ceq ($taskReference[$taskIndex]|ConvertTo-Json -Depth 8 -Compress)) 'Frozen baseline counter drift'
            $taskExpectedAlloc=if($taskLength -lt 38){0}else{256}
            Need ($taskRow.length -eq $taskLength -and $taskRow.iterations -eq 256 -and $taskRow.counts.allocations -eq $taskExpectedAlloc -and $taskRow.counts.reallocations -eq 0 -and $taskRow.counts.requested_bytes -eq ($taskLength+1)*$taskExpectedAlloc -and $taskRow.production_length -is [bool] -and $taskRow.production_length -eq (@(33,37) -contains $taskLength) -and $taskRow.rust_requests_only -eq $true -and -not $taskRow.timing_claim -and -not $taskRow.profile -and -not $taskRow.server -and -not $taskRow.product) 'Candidate requests or scope drift'
            $taskAllocations+=$taskRow.counts.allocations;$taskBytes+=$taskRow.counts.requested_bytes
            if($taskRow.production_length){$taskProductionAllocations+=$taskRow.counts.allocations;$taskProductionBytes+=$taskRow.counts.requested_bytes}
        }
        if($taskCohort -ceq 'corrected'){
            $taskLanes+=[ordered]@{lane=$taskLane;constructions=2304;reference_allocations=2304;reference_requested_bytes=64768;candidate_allocations=$taskAllocations;candidate_requested_bytes=$taskBytes;reallocations=0;production_length_constructions=512;reference_production_allocations=512;reference_production_requested_bytes=18432;candidate_production_allocations=$taskProductionAllocations;candidate_production_requested_bytes=$taskProductionBytes;candidate_controls=4;unchanged_baseline_controls=5;allocator_counter_controls=2}
        }
    }
}
Need ((Get-Item (Join-Path $taskPacket 'final/format.log')).Length -eq 0) 'Format refused'
Need ((Text (Join-Path $taskPacket 'final/docs.log')) -match 'doc-check: OK') 'Docs refused'
Need ((Text (Join-Path $taskPacket 'final/registry.log')) -match 'OK \(186 discovered gates, all registered\)') 'Registry refused'
Need ((Text (Join-Path $taskPacket 'final/performance.log')) -match 'OK \(local, non-promotable\)') 'Performance contract refused'
$taskSummary=[ordered]@{candidate_source=$Source;baseline_source='71638d9db2817c1a1d24edddb467e2cbf7a4ec9e';lanes=$taskLanes;source_canaries=25;prior_members_verified=249;first_gnu_lint_refusal_retained=$true;runtime_candidate_implemented=$true;timing_improvement_proven=$false;original_windows_seconds=@(300,60,10);first_refusal_ages_ns=@(10042390247,10010835739);new_original_owner_profiles=0;server=$false;product=$false}
$taskInventory=Join-Path $taskPacket 'inventory.json'
if($Mode -ceq 'Seal'){
    Need (-not(Test-Path $taskInventory)) 'Never replace candidate seal'
    WriteNew (Join-Path $taskPacket 'identity.json') @{candidate_source=$Source}
    WriteNew (Join-Path $taskPacket 'summary.json') $taskSummary
    $taskFiles=@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File|Sort-Object FullName|ForEach-Object {@{path=[IO.Path]::GetRelativePath($taskPacket,$_.FullName).Replace([char]92,[char]47);bytes=$_.Length;sha256=(Hash $_.FullName)}})
    WriteNew $taskInventory @{candidate_source=$Source;files=$taskFiles}
}
Need (($taskSummary|ConvertTo-Json -Depth 20 -Compress) -ceq ((Json (Join-Path $taskPacket 'summary.json'))|ConvertTo-Json -Depth 20 -Compress)) 'Derived summary drift'
$taskSeal=Json $taskInventory
Need ($taskSeal.candidate_source -ceq $Source) 'Candidate identity drift'
Members $taskPacket $taskSeal
Write-Output "Context document stack verified: $($taskSeal.files.Count) members; inventory $(Hash $taskInventory); 249 prior members intact; no live replay or timing claim."
