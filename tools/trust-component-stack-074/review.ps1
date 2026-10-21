#requires -Version 7.2
param([ValidateSet('Seal','Review')][string]$Mode='Review',[string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/trust-component-stack-20261021'
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
    @('trust-component-baseline-20261021','7eacd69041a722fc3d6153ef8be43eff87bf945a8fb6ccdd456acadff1bfcc9d')
)){
    $taskPrior=Join-Path (Split-Path $taskPacket) $taskPin[0]
    Need ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq $taskPin[1]) 'Prior raw seal drift'
    $taskPriorSeal=Json (Join-Path $taskPrior 'inventory.json');Members $taskPrior $taskPriorSeal
    $taskPriorCount+=$taskPriorSeal.files.Count
}
Need ($taskPriorCount -eq 348) 'Prior member count drift'
if($Mode -ceq 'Review'){$Source=(Json (Join-Path $taskPacket 'identity.json')).candidate_source}
Need ($Source -match '^[a-f0-9]{40}$') 'Exact candidate source required'
& git diff --quiet $Source -- tools/long-run-supervisor-074 tools/trust-component-stack-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
Need ($LASTEXITCODE -eq 0) 'Candidate source drift'
$taskUntracked=@(& git ls-files --others --exclude-standard -- tools/long-run-supervisor-074 tools/trust-component-stack-074 crates)
Need ($LASTEXITCODE -eq 0 -and $taskUntracked.Count -eq 0) 'Untracked candidate source'
Need ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification drift'
foreach($taskName in @('check.ps1','review.ps1','README.md')){Need ((Hash "tools/trust-component-stack-074/$taskName") -ceq (Hash (Join-Path $taskPacket "method/$taskName"))) 'Recipe snapshot drift'}
foreach($taskPath in @(
    'tools/long-run-supervisor-074/src/diagnostic_trust_component_name.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs',
    'tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs',
    'tools/long-run-supervisor-074/tests/support/trust_component_stack.rs',
    'tools/long-run-supervisor-074/tests/support/trust_component_stack_adapter.rs',
    'tools/long-run-supervisor-074/tests/support/trust_component_baseline.rs',
    'crates/xtask/tests/trust_component_stack_074.rs',
    'crates/xtask/tests/trust_component_baseline_074.rs',
    'crates/xtask/tests/context_document_stack_074.rs',
    'crates/xtask/tests/context_document_baseline_074.rs',
    'crates/xtask/tests/projection_groups_074.rs',
    'crates/xtask/tests/pidfd_info_path_074.rs',
    'crates/xtask/tests/support/trust_component_stack_guard.rs',
    'crates/xtask/tests/support/trust_component_baseline_guard.rs'
)){Need ((Hash $taskPath) -ceq (Hash (Join-Path $taskPacket "source/$taskPath"))) 'Source snapshot drift'}
$taskFixed=Text 'tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs'
Need ($taskFixed.Replace("#[cfg(test)]`nuse std::ffi::CString;`n",'') -ceq (Text (Join-Path $taskPacket 'initial-source/diagnostic_worker_trust_input.rs'))) 'Initial source differs beyond test-only import restoration'
Need ((Hash (Join-Path $taskPacket 'initial-source/trust_component_stack_guard.rs')) -ceq 'cef8b4f853f4d7074b01134d65ba8e33c8ebea24060cade834d7d8461a03ac70') 'Initial guard source drift'
function Unit([string]$Path,[int]$Count){
    $taskText=Text (Join-Path $taskPacket $Path)
    Need ($taskText -match "test result: ok\. $Count passed; 0 failed; 0 ignored;" -and $taskText -notmatch 'test result: FAILED') "Unit refused: $Path"
}
foreach($taskLane in @('host','194','188')){
    foreach($taskCohort in @('first','corrected')){
        $taskResult=Json (Join-Path $taskPacket "$taskCohort/$taskLane-result.json")
        $taskFailed=$taskCohort -ceq 'first' -and $taskLane -cne 'host'
        $taskExpected=if($taskLane -ceq 'host'){'canaries,check,clippy'}elseif($taskFailed){'trust_component_stack_'}else{'trust_component_stack_,trust_component_baseline_,context_document_stack_,context_document_buffer_baseline,fixed_leaf_alloc_counter,check,clippy'}
        Need ($taskResult.cohort -ceq $taskCohort -and $taskResult.lane -ceq $taskLane -and ($taskResult.steps.name -join ',') -ceq $taskExpected -and $taskResult.runtime_candidate_implemented -eq $true -and -not $taskResult.ignored_tests_run -and $taskResult.original_owner_profiles -eq 0 -and -not $taskResult.server -and -not $taskResult.product) 'Scope/sequence drift'
        if($taskFailed){
            Need ($taskResult.failure -ceq "Check refused: first/$taskLane/trust_component_stack_ (101)") 'First compile refusal erased'
            $taskFailure=Text (Join-Path $taskPacket "first/$taskLane-trust_component_stack_.log")
            Need ([regex]::Matches($taskFailure,'error\[E0433\]').Count -eq 2 -and $taskFailure.Contains('CString') -and -not $taskFailure.Contains('test result: ok.')) 'First compile detail drift'
        }else{Need ($null -eq $taskResult.failure) 'Checks refused'}
        foreach($taskStep in $taskResult.steps){
            $taskExit=if($taskFailed){101}else{0}
            Need ($taskStep.log -ceq "$taskLane-$($taskStep.name).log" -and $taskStep.exit -eq $taskExit -and $taskStep.sha256 -ceq (Hash (Join-Path $taskPacket "$taskCohort/$($taskStep.log)"))) 'Check log drift'
        }
        if($taskLane -ceq 'host'){
            $taskCanaries=Text (Join-Path $taskPacket "$taskCohort/host-canaries.log");$taskTotal=0
            foreach($taskMatch in [regex]::Matches($taskCanaries,'test result: ok\. (\d+) passed; 0 failed; 0 ignored;')){$taskTotal += [int]$taskMatch.Groups[1].Value}
            Need ($taskTotal -eq 31 -and $taskCanaries -notmatch 'test result: FAILED') 'Source canaries refused'
        }elseif(-not $taskFailed){
            Unit "corrected/$taskLane-trust_component_stack_.log" 5
            Unit "corrected/$taskLane-trust_component_baseline_.log" 5
            Unit "corrected/$taskLane-context_document_stack_.log" 4
            Unit "corrected/$taskLane-context_document_buffer_baseline.log" 5
            Unit "corrected/$taskLane-fixed_leaf_alloc_counter.log" 2
            Need ((Text (Join-Path $taskPacket "corrected/$taskLane-trust_component_stack_.log")) -match 'seed=0x074c57a1 samples=512 model_only=true') 'Seeded differential missing'
        }
    }
}
function Rows([string]$Path,[string]$Prefix){@(Get-Content -LiteralPath $Path|Where-Object {$_.Contains($Prefix)}|ForEach-Object {($_ -split [regex]::Escape($Prefix),2)[1]|ConvertFrom-Json})}
$taskLanes=@()
foreach($taskLane in @('194','188')){
    $taskOldLog=Join-Path $taskPacket "corrected/$taskLane-trust_component_baseline_.log"
    $taskNewLog=Join-Path $taskPacket "corrected/$taskLane-trust_component_stack_.log"
    $taskPriorLog=Join-Path (Split-Path $taskPacket) "trust-component-baseline-20261021/corrected/$taskLane-trust_component_baseline_.log"
    $taskTotals=[ordered]@{valid_constructions=0;reference_valid_allocations=0L;reference_valid_bytes=0L;candidate_valid_allocations=0L;candidate_valid_bytes=0L;production_constructions=0;reference_production_allocations=0L;reference_production_bytes=0L;candidate_production_allocations=0L;candidate_production_bytes=0L;nul_constructions=0;reference_nul_allocations=0L;reference_nul_bytes=0L;candidate_nul_allocations=0L;candidate_nul_bytes=0L}
    foreach($taskKind in @('VALID','NUL')){
        $taskPrefix=if($taskKind -ceq 'VALID'){'HC074-TRUST-COMPONENT-'}else{'HC074-TRUST-COMPONENT-NUL-'}
        $taskOld=Rows $taskOldLog ($taskPrefix+'BASELINE ');$taskNew=Rows $taskNewLog ($taskPrefix+'CANDIDATE ');$taskPrior=Rows $taskPriorLog ($taskPrefix+'BASELINE ')
        $taskCount=if($taskKind -ceq 'VALID'){21}else{4}
        Need ($taskOld.Count -eq $taskCount -and $taskNew.Count -eq $taskCount -and $taskPrior.Count -eq $taskCount) 'Missing corpus cases'
        for($taskIndex=0;$taskIndex -lt $taskCount;$taskIndex++){
            $taskRef=$taskOld[$taskIndex];$taskRow=$taskNew[$taskIndex]
            Need (($taskRef|ConvertTo-Json -Depth 8 -Compress) -ceq ($taskPrior[$taskIndex]|ConvertTo-Json -Depth 8 -Compress)) 'Frozen baseline counter drift'
            $taskAlloc=if($taskRef.length -lt 32){0}else{256}
            Need ($taskRow.label -ceq $taskRef.label -and $taskRow.length -eq $taskRef.length -and $taskRow.iterations -eq 256 -and $taskRow.counts.allocations -eq $taskAlloc -and $taskRow.counts.reallocations -eq 0 -and $taskRow.counts.requested_bytes -eq ($taskRef.length+1)*$taskAlloc -and $taskRow.rust_requests_only -eq $true -and -not $taskRow.timing_claim -and -not $taskRow.profile -and -not $taskRow.server -and -not $taskRow.product) 'Candidate requests or scope drift'
            $taskBucket=if($taskKind -ceq 'VALID'){'valid'}else{'nul'}
            $taskTotals["${taskBucket}_constructions"]+=256
            $taskTotals["reference_${taskBucket}_allocations"]+=$taskRef.counts.allocations;$taskTotals["reference_${taskBucket}_bytes"]+=$taskRef.counts.requested_bytes
            $taskTotals["candidate_${taskBucket}_allocations"]+=$taskRow.counts.allocations;$taskTotals["candidate_${taskBucket}_bytes"]+=$taskRow.counts.requested_bytes
            if($taskKind -ceq 'VALID'){
                Need ($taskRow.production -is [bool] -and $taskRow.production -eq $taskRef.production) 'Production corpus drift'
                if($taskRef.production){
                    $taskTotals.production_constructions+=256
                    $taskTotals.reference_production_allocations+=$taskRef.counts.allocations;$taskTotals.reference_production_bytes+=$taskRef.counts.requested_bytes
                    $taskTotals.candidate_production_allocations+=$taskRow.counts.allocations;$taskTotals.candidate_production_bytes+=$taskRow.counts.requested_bytes
                }
            }else{Need ($taskRow.position -eq $taskRef.position) 'NUL position drift'}
        }
    }
    $taskLanes+=[ordered]@{lane=$taskLane;totals=$taskTotals;candidate_controls=5;unchanged_baseline_controls=5;prior_pure_controls=9;counter_controls=2}
}
Need ((Get-Item (Join-Path $taskPacket 'final/format.log')).Length -eq 0) 'Format refused'
Need ((Text (Join-Path $taskPacket 'final/docs.log')) -match 'doc-check: OK') 'Docs refused'
Need ((Text (Join-Path $taskPacket 'final/registry.log')) -match 'OK \(186 discovered gates, all registered\)') 'Registry refused'
Need ((Text (Join-Path $taskPacket 'final/performance.log')) -match 'OK \(local, non-promotable\)') 'Performance contract refused'
$taskSummary=[ordered]@{candidate_source=$Source;baseline_source='a72457aade616e67d2fc5693f6105c8063d6e9e1';lanes=$taskLanes;source_canaries=31;prior_members_verified=348;first_linux_compile_refusals_retained=$true;runtime_candidate_implemented=$true;timing_improvement_proven=$false;original_windows_seconds=@(300,60,10);first_refusal_ages_ns=@(10042390247,10010835739);new_original_owner_profiles=0;server=$false;product=$false}
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
Write-Output "Trust component stack verified: $($taskSeal.files.Count) members; inventory $(Hash $taskInventory); 348 prior members intact; no live replay or timing claim."
