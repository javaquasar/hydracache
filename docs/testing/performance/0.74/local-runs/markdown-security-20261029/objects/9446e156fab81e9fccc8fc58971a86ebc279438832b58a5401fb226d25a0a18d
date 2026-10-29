#requires -Version 7.2
param([ValidateSet('Seal','Review')][string]$Mode='Review',[string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/trust-component-baseline-20261021'
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
    @('context-document-stack-20261021','7a65d50c19622754bda58fd389ef03d3f07254465d96f1e16b586cdec94f71f0')
)){
    $taskPrior=Join-Path (Split-Path $taskPacket) $taskPin[0]
    Need ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq $taskPin[1]) 'Prior raw seal drift'
    $taskPriorSeal=Json (Join-Path $taskPrior 'inventory.json')
    Members $taskPrior $taskPriorSeal
    $taskPriorCount+=$taskPriorSeal.files.Count
}
Need ($taskPriorCount -eq 303) 'Prior member count drift'
if($Mode -ceq 'Review'){$Source=(Json (Join-Path $taskPacket 'identity.json')).baseline_source}
Need ($Source -match '^[a-f0-9]{40}$') 'Exact baseline source required'
& git diff --quiet $Source -- tools/long-run-supervisor-074 tools/trust-component-baseline-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
Need ($LASTEXITCODE -eq 0) 'Baseline source drift'
$taskUntracked=@(& git ls-files --others --exclude-standard -- tools/long-run-supervisor-074 tools/trust-component-baseline-074 crates)
Need ($LASTEXITCODE -eq 0 -and $taskUntracked.Count -eq 0) 'Untracked baseline source'
Need ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification drift'
foreach($taskName in @('check.ps1','review.ps1','README.md')){
    Need ((Hash "tools/trust-component-baseline-074/$taskName") -ceq (Hash (Join-Path $taskPacket "method/$taskName"))) 'Recipe snapshot drift'
}
foreach($taskPath in @(
    'tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_trust_input.rs',
    'tools/long-run-supervisor-074/src/diagnostic_held_runtime_trust.rs',
    'tools/long-run-supervisor-074/src/diagnostic_oidc_input.rs',
    'tools/long-run-supervisor-074/tests/support/trust_component_baseline.rs',
    'crates/xtask/tests/trust_component_baseline_074.rs',
    'crates/xtask/tests/support/trust_component_baseline_guard.rs',
    'crates/xtask/tests/support/context_document_stack_guard.rs'
)){
    Need ((Hash $taskPath) -ceq (Hash (Join-Path $taskPacket "source/$taskPath"))) 'Source snapshot drift'
}
$taskFixed=Text 'tools/long-run-supervisor-074/tests/support/trust_component_baseline.rs'
Need ($taskFixed.Replace('error.into_vec().as_slice()', 'value.as_bytes()') -ceq (Text (Join-Path $taskPacket 'initial-source/trust_component_baseline.rs'))) 'Initial source differs beyond the payload inspection repair'
function Unit([string]$Path,[int]$Count){
    $taskText=Text (Join-Path $taskPacket $Path)
    Need ($taskText -match "test result: ok\. $Count passed; 0 failed; 0 ignored;" -and $taskText -notmatch 'test result: FAILED') "Unit refused: $Path"
}
foreach($taskLane in @('host','194','188')){
    foreach($taskCohort in @('first','corrected')){
        $taskResult=Json (Join-Path $taskPacket "$taskCohort/$taskLane-result.json")
        $taskFailed=$taskCohort -ceq 'first' -and $taskLane -cne 'host'
        $taskExpected=if($taskLane -ceq 'host'){'canaries,check,clippy'}elseif($taskFailed){'trust_component_baseline_'}else{'trust_component_baseline_,context_document_stack_,context_document_buffer_baseline,fixed_leaf_alloc_counter,check,clippy'}
        Need ($taskResult.cohort -ceq $taskCohort -and $taskResult.lane -ceq $taskLane -and ($taskResult.steps.name -join ',') -ceq $taskExpected -and -not $taskResult.runtime_candidate_implemented -and -not $taskResult.ignored_tests_run -and $taskResult.original_owner_profiles -eq 0 -and -not $taskResult.server -and -not $taskResult.product) 'Scope/sequence drift'
        if($taskFailed){
            Need ($taskResult.failure -ceq "Check refused: first/$taskLane/trust_component_baseline_ (101)") 'First compile refusal erased'
            $taskFailure=Text (Join-Path $taskPacket "first/$taskLane-trust_component_baseline_.log")
            Need ($taskFailure.Contains('error[E0599]') -and $taskFailure.Contains('as_bytes') -and -not $taskFailure.Contains('test result: ok.')) 'First compile detail drift'
        }else{Need ($null -eq $taskResult.failure) 'Checks refused'}
        foreach($taskStep in $taskResult.steps){
            $taskExit=if($taskFailed){101}else{0}
            Need ($taskStep.log -ceq "$taskLane-$($taskStep.name).log" -and $taskStep.exit -eq $taskExit -and $taskStep.sha256 -ceq (Hash (Join-Path $taskPacket "$taskCohort/$($taskStep.log)"))) 'Check log drift'
        }
        if($taskLane -ceq 'host'){
            $taskCanaries=Text (Join-Path $taskPacket "$taskCohort/host-canaries.log");$taskTotal=0
            foreach($taskMatch in [regex]::Matches($taskCanaries,'test result: ok\. (\d+) passed; 0 failed; 0 ignored;')){$taskTotal += [int]$taskMatch.Groups[1].Value}
            Need ($taskTotal -eq 28 -and $taskCanaries -notmatch 'test result: FAILED') 'Source canaries refused'
        }elseif(-not $taskFailed){
            Unit "corrected/$taskLane-trust_component_baseline_.log" 5
            Unit "corrected/$taskLane-context_document_stack_.log" 4
            Unit "corrected/$taskLane-context_document_buffer_baseline.log" 5
            Unit "corrected/$taskLane-fixed_leaf_alloc_counter.log" 2
            Need ((Text (Join-Path $taskPacket "corrected/$taskLane-trust_component_baseline_.log")) -match 'seed=0x074c57a1 samples=512 model_only=true') 'Seeded model missing'
        }
    }
}
function Rows([string]$Path,[string]$Prefix){
    @(Get-Content -LiteralPath $Path|Where-Object {$_.Contains($Prefix)}|ForEach-Object {($_ -split [regex]::Escape($Prefix),2)[1]|ConvertFrom-Json})
}
$taskCases=@()
foreach($taskName in @('etc','hydracache-perf','worker-trust-074.json','controller-trust-074.json','held-runtime-trust-074.json','oidc-trust-074.json')){$taskCases+=@{label=$taskName;length=[Text.Encoding]::UTF8.GetByteCount($taskName);production=$true}}
foreach($taskLength in @(0,1,30,31,32,63,64,255,256,4096)){$taskCases+=@{label="length-$taskLength";length=$taskLength;production=$false}}
foreach($taskPair in @(@('unicode-cyrillic','ключ'),@('unicode-path','é/名'),@('parent-component','../x'),@('dot','.'),@('absolute-name','/x'))){$taskCases+=@{label=$taskPair[0];length=[Text.Encoding]::UTF8.GetByteCount($taskPair[1]);production=$false}}
$taskNuls=@(@('leading-nul',4,0),@('interior-nul',5,2),@('trailing-nul',4,3),@('unicode-nul',6,2))
$taskLanes=@()
foreach($taskLane in @('194','188')){
    $taskLog=Join-Path $taskPacket "corrected/$taskLane-trust_component_baseline_.log"
    $taskRows=Rows $taskLog 'HC074-TRUST-COMPONENT-BASELINE '
    $taskNulRows=Rows $taskLog 'HC074-TRUST-COMPONENT-NUL-BASELINE '
    Need ($taskRows.Count -eq 21 -and $taskNulRows.Count -eq 4) 'Missing baseline cases'
    $taskAllocations=0L;$taskBytes=0L;$taskProductionAllocations=0L;$taskProductionBytes=0L;$taskNulAllocations=0L;$taskNulBytes=0L
    for($taskIndex=0;$taskIndex -lt 25;$taskIndex++){
        $taskValid=$taskIndex -lt 21
        if($taskValid){$taskRow=$taskRows[$taskIndex];$taskCase=$taskCases[$taskIndex];$taskLength=$taskCase.length;$taskLabel=$taskCase.label}
        else{$taskNulIndex=$taskIndex-21;$taskRow=$taskNulRows[$taskNulIndex];$taskCase=$taskNuls[$taskNulIndex];$taskLength=$taskCase[1];$taskLabel=$taskCase[0];Need ($taskRow.position -eq $taskCase[2]) 'NUL position drift'}
        Need ($taskRow.label -ceq $taskLabel -and $taskRow.length -eq $taskLength -and $taskRow.iterations -eq 256 -and $taskRow.counts.allocations -eq 256 -and $taskRow.counts.reallocations -eq 0 -and $taskRow.counts.requested_bytes -eq ($taskLength+1)*256 -and $taskRow.rust_requests_only -eq $true -and -not $taskRow.timing_claim -and -not $taskRow.profile -and -not $taskRow.server -and -not $taskRow.product) 'Baseline requests or scope drift'
        if($taskValid){
            Need ($taskRow.production -is [bool] -and $taskRow.production -eq $taskCase.production) 'Production corpus drift'
            $taskAllocations+=$taskRow.counts.allocations;$taskBytes+=$taskRow.counts.requested_bytes
            if($taskRow.production){$taskProductionAllocations+=$taskRow.counts.allocations;$taskProductionBytes+=$taskRow.counts.requested_bytes}
        }else{$taskNulAllocations+=$taskRow.counts.allocations;$taskNulBytes+=$taskRow.counts.requested_bytes}
    }
    $taskLanes+=[ordered]@{lane=$taskLane;valid_constructions=5376;valid_allocations=$taskAllocations;valid_requested_bytes=$taskBytes;production_constructions=1536;production_allocations=$taskProductionAllocations;production_requested_bytes=$taskProductionBytes;nul_constructions=1024;nul_allocations=$taskNulAllocations;nul_requested_bytes=$taskNulBytes;reallocations=0;pure_baseline_controls=5;prior_pure_controls=9;counter_controls=2}
}
Need ((Get-Item (Join-Path $taskPacket 'final/format.log')).Length -eq 0) 'Format refused'
Need ((Text (Join-Path $taskPacket 'final/docs.log')) -match 'doc-check: OK') 'Docs refused'
Need ((Text (Join-Path $taskPacket 'final/registry.log')) -match 'OK \(186 discovered gates, all registered\)') 'Registry refused'
Need ((Text (Join-Path $taskPacket 'final/performance.log')) -match 'OK \(local, non-promotable\)') 'Performance contract refused'
$taskSummary=[ordered]@{baseline_source=$Source;lanes=$taskLanes;source_canaries=28;prior_members_verified=303;first_linux_compile_refusals_retained=$true;runtime_candidate_implemented=$false;timing_improvement_proven=$false;original_windows_seconds=@(300,60,10);first_refusal_ages_ns=@(10042390247,10010835739);new_original_owner_profiles=0;server=$false;product=$false}
$taskInventory=Join-Path $taskPacket 'inventory.json'
if($Mode -ceq 'Seal'){
    Need (-not(Test-Path $taskInventory)) 'Never replace baseline seal'
    WriteNew (Join-Path $taskPacket 'identity.json') @{baseline_source=$Source}
    WriteNew (Join-Path $taskPacket 'summary.json') $taskSummary
    $taskFiles=@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File|Sort-Object FullName|ForEach-Object {@{path=[IO.Path]::GetRelativePath($taskPacket,$_.FullName).Replace([char]92,[char]47);bytes=$_.Length;sha256=(Hash $_.FullName)}})
    WriteNew $taskInventory @{baseline_source=$Source;files=$taskFiles}
}
Need (($taskSummary|ConvertTo-Json -Depth 20 -Compress) -ceq ((Json (Join-Path $taskPacket 'summary.json'))|ConvertTo-Json -Depth 20 -Compress)) 'Derived summary drift'
$taskSeal=Json $taskInventory
Need ($taskSeal.baseline_source -ceq $Source) 'Baseline identity drift'
Members $taskPacket $taskSeal
Write-Output "Trust component baseline verified: $($taskSeal.files.Count) members; inventory $(Hash $taskInventory); 303 prior members intact; no runtime candidate or live replay."
