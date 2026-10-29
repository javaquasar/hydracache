#requires -Version 7.2
param([ValidateSet('Seal','Review')][string]$Mode='Review',[string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/context-document-baseline-20261021'
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
foreach($taskPin in @(@('held-material-live-20261021','0c385fbfe209fad1903aa2c0e620817c458f413eec8bd36fa32be5c01fbb4109'),@('held-material-round-attribution-20261021','84c2d0a10e1d2d66b4dc76e1c041b9920d90d85621461bc3e83102569a6e1a2f'),@('pidfd-info-path-20261021','dc7af6cd78dfc1b69c7443078a8c6e2edc4504e294f74d30df93e085f8471daa'),@('projection-groups-20261021','ae7db79c3e579b7ad9f1939d2571aab27452b62f81c89b46b98340381f4b91c6'),@('reserved-leaf-audit-20261021','d66631eec23dc969f372d73c2bb83519058214383023564ec269ec542f854c4e'))){
    $taskPrior=Join-Path (Split-Path $taskPacket) $taskPin[0]
    Need ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq $taskPin[1]) 'Prior raw seal drift'
    Members $taskPrior (Json (Join-Path $taskPrior 'inventory.json'))
}
if($Mode -ceq 'Review'){$Source=(Json (Join-Path $taskPacket 'identity.json')).baseline_source}
Need ($Source -match '^[a-f0-9]{40}$') 'Exact baseline source required'
& git diff --quiet $Source -- tools/long-run-supervisor-074 tools/context-document-baseline-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
Need ($LASTEXITCODE -eq 0) 'Baseline source drift'
Need ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification drift'
foreach($taskName in @('check.ps1','review.ps1','README.md')){
    Need ((Hash "tools/context-document-baseline-074/$taskName") -ceq (Hash (Join-Path $taskPacket "method/$taskName"))) 'Recipe snapshot drift'
}
Need ((Hash 'tools/long-run-supervisor-074/tests/support/context_document_buffer_baseline.rs') -ceq (Hash (Join-Path $taskPacket 'source/context_document_buffer_baseline.rs'))) 'Control source snapshot drift'
Need ((Hash 'tools/long-run-supervisor-074/src/diagnostic_worker_context.rs') -ceq (Hash (Join-Path $taskPacket 'source/diagnostic_worker_context.rs'))) 'Runtime snapshot drift'
$taskCorrected=Text 'tools/long-run-supervisor-074/tests/support/context_document_buffer_baseline.rs'
$taskUnfixed=$taskCorrected.Replace('"production_length":([33,37].contains(&length))','"production_length":[33,37].contains(&length)').Replace('for length in [0_usize, 1, 4, 33, 37, 63]','for length in [0, 1, 4, 33, 37, 63]')
Need ($taskUnfixed -ceq (Text (Join-Path $taskPacket 'initial/source/context_document_buffer_baseline.rs'))) 'First control source differs beyond the two compile fixes'
function Unit([string]$Path,[int]$Count){
    $taskText=Text (Join-Path $taskPacket $Path)
    Need ($taskText -match "test result: ok\. $Count passed; 0 failed; 0 ignored;" -and $taskText -notmatch 'test result: FAILED') "Unit refused: $Path"
}
foreach($taskLane in @('host','194','188')){
    $taskResult=Json (Join-Path $taskPacket "corrected/$taskLane-result.json")
    $taskExpected=if($taskLane -ceq 'host'){'canaries,check,clippy'}else{'context_document_buffer_baseline,fixed_leaf_alloc_counter,check,clippy'}
    Need ($taskResult.lane -ceq $taskLane -and $null -eq $taskResult.failure -and ($taskResult.steps.name -join ',') -ceq $taskExpected -and $taskResult.runtime_candidate_implemented -is [bool] -and -not $taskResult.runtime_candidate_implemented -and -not $taskResult.ignored_tests_run -and $taskResult.original_owner_profiles -eq 0 -and -not $taskResult.server -and -not $taskResult.product) 'Corrected check scope/sequence/outcome drift'
    foreach($taskStep in $taskResult.steps){Need ($taskStep.exit -eq 0 -and $taskStep.sha256 -ceq (Hash (Join-Path $taskPacket "corrected/$($taskStep.log)"))) 'Corrected check log drift'}
    $taskInitial=Json (Join-Path $taskPacket "initial/$taskLane-result.json")
    if($taskLane -ceq 'host'){Need ($null -eq $taskInitial.failure -and $taskInitial.steps.Count -eq 3) 'Initial host outcome erased'}
    else{Need ($taskInitial.failure -is [string] -and $taskInitial.steps.Count -eq 1 -and $taskInitial.steps[0].exit -eq 101) 'First compile refusal erased'}
    foreach($taskStep in $taskInitial.steps){Need ($taskStep.sha256 -ceq (Hash (Join-Path $taskPacket "initial/$($taskStep.log)"))) 'Initial log drift'}
}
$taskLanes=@();$taskLengths=@(0,1,4,32,33,36,37,38,63)
foreach($taskLane in @('194','188')){
    Unit "corrected/$taskLane-context_document_buffer_baseline.log" 5
    Unit "corrected/$taskLane-fixed_leaf_alloc_counter.log" 2
    $taskRows=@(Get-Content (Join-Path $taskPacket "corrected/$taskLane-context_document_buffer_baseline.log")|Where-Object {$_.Contains('HC074-CONTEXT-DOCUMENT-BASELINE ')}|ForEach-Object {($_ -split 'HC074-CONTEXT-DOCUMENT-BASELINE ',2)[1]|ConvertFrom-Json})
    Need ($taskRows.Count -eq 9) 'Missing baseline cases'
    $taskAllocations=0L;$taskBytes=0L;$taskProductionAllocations=0L;$taskProductionBytes=0L
    for($taskIndex=0;$taskIndex -lt 9;$taskIndex++){
        $taskRow=$taskRows[$taskIndex];$taskLength=$taskLengths[$taskIndex]
        Need ($taskRow.length -eq $taskLength -and $taskRow.iterations -eq 256 -and $taskRow.counts.allocations -eq 256 -and $taskRow.counts.reallocations -eq 0 -and $taskRow.counts.requested_bytes -eq ($taskLength+1)*256 -and $taskRow.production_length -is [bool] -and $taskRow.production_length -eq (@(33,37) -contains $taskLength) -and -not $taskRow.timing_claim -and -not $taskRow.profile -and -not $taskRow.server -and -not $taskRow.product) 'Baseline requests or scope differ'
        $taskAllocations+=$taskRow.counts.allocations;$taskBytes+=$taskRow.counts.requested_bytes
        if($taskRow.production_length){$taskProductionAllocations+=$taskRow.counts.allocations;$taskProductionBytes+=$taskRow.counts.requested_bytes}
    }
    $taskLanes+=[ordered]@{lane=$taskLane;constructions=2304;allocations=$taskAllocations;reallocations=0;requested_bytes=$taskBytes;production_length_constructions=512;production_length_allocations=$taskProductionAllocations;production_length_requested_bytes=$taskProductionBytes;pure_unit_controls=5;allocator_counter_controls=2}
}
$taskCanaries=Text (Join-Path $taskPacket 'corrected/host-canaries.log');$taskTotal=0
foreach($taskMatch in [regex]::Matches($taskCanaries,'test result: ok\. (\d+) passed; 0 failed; 0 ignored;')){$taskTotal += [int]$taskMatch.Groups[1].Value}
Need ($taskTotal -eq 22 -and $taskCanaries -notmatch 'test result: FAILED') 'Source canaries refused'
Need ((Get-Item (Join-Path $taskPacket 'final/format.log')).Length -eq 0) 'Format refused'
Need ((Text (Join-Path $taskPacket 'final/docs.log')) -match 'doc-check: OK') 'Docs refused'
Need ((Text (Join-Path $taskPacket 'final/registry.log')) -match 'OK \(186 discovered gates, all registered\)') 'Registry refused'
Need ((Text (Join-Path $taskPacket 'final/performance.log')) -match 'OK \(local, non-promotable\)') 'Performance contract refused'
$taskSummary=[ordered]@{baseline_source=$Source;runtime_before='8717ea4f482a995226a799df1d39b77d4665a60a';lanes=$taskLanes;baseline_only=$true;runtime_candidate_implemented=$false;timing_improvement_proven=$false;original_windows_seconds=@(300,60,10);first_refusal_ages_ns=@(10042390247,10010835739);new_original_owner_profiles=0;server=$false;product=$false}
$taskInventory=Join-Path $taskPacket 'inventory.json'
if($Mode -ceq 'Seal'){
    Need (-not(Test-Path $taskInventory)) 'Never replace baseline seal'
    WriteNew (Join-Path $taskPacket 'identity.json') @{baseline_source=$Source}
    WriteNew (Join-Path $taskPacket 'summary.json') $taskSummary
    $taskFiles=@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File|Sort-Object FullName|ForEach-Object {@{path=[IO.Path]::GetRelativePath($taskPacket,$_.FullName).Replace([char]92,[char]47);bytes=$_.Length;sha256=(Hash $_.FullName)}})
    WriteNew $taskInventory @{baseline_source=$Source;files=$taskFiles}
}
Need (($taskSummary|ConvertTo-Json -Depth 20 -Compress) -ceq ((Json (Join-Path $taskPacket 'summary.json'))|ConvertTo-Json -Depth 20 -Compress)) 'Derived baseline summary drift'
$taskSeal=Json $taskInventory
Need ($taskSeal.baseline_source -ceq $Source) 'Baseline identity drift'
Members $taskPacket $taskSeal
Write-Output "Context document baseline verified: $($taskSeal.files.Count) members; inventory $(Hash $taskInventory); no runtime candidate or live replay."
