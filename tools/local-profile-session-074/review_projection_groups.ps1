#requires -Version 7.2
param([ValidateSet('Seal','Review')][string]$Mode='Review',[string]$Baseline,[string]$Candidate)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/projection-groups-20261021'
function Need($Ok,[string]$Why){if(-not $Ok){throw $Why}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Load([string]$Path){Get-Content -LiteralPath $Path -Raw|ConvertFrom-Json}
function WriteNew([string]$Path,$Value){
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject $Value -Depth 20)+"`n")
    $taskStream=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
function Members([string]$Directory,$Seal){
    $taskSeen=@{}
    Need (@(Get-ChildItem -LiteralPath $Directory -Recurse -Force -Attributes ReparsePoint).Count -eq 0) 'Reparse point'
    foreach($taskRow in $Seal.files){
        Need (-not[IO.Path]::IsPathRooted($taskRow.path) -and -not $taskRow.path.Contains([char]92) -and -not($taskRow.path.Split([char]47) -contains '..') -and -not $taskSeen.ContainsKey($taskRow.path)) 'Unsafe member'
        $taskSeen[$taskRow.path]=$true
        $taskItem=Get-Item -LiteralPath (Join-Path $Directory $taskRow.path)
        Need ($taskItem.Length -eq $taskRow.bytes -and (Hash $taskItem.FullName) -ceq $taskRow.sha256) 'Member drift'
    }
    Need (@(Get-ChildItem -LiteralPath $Directory -Recurse -File).Count -eq $taskSeen.Count+1) 'Membership drift'
}
foreach($taskPin in @(@('held-material-live-20261021','0c385fbfe209fad1903aa2c0e620817c458f413eec8bd36fa32be5c01fbb4109'),@('held-material-round-attribution-20261021','84c2d0a10e1d2d66b4dc76e1c041b9920d90d85621461bc3e83102569a6e1a2f'),@('pidfd-info-path-20261021','dc7af6cd78dfc1b69c7443078a8c6e2edc4504e294f74d30df93e085f8471daa'))){
    $taskPrior=Join-Path (Split-Path $taskPacket) $taskPin[0]
    Need ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq $taskPin[1]) 'Prior seal drift'
    Members $taskPrior (Load (Join-Path $taskPrior 'inventory.json'))
}
if($Mode -ceq 'Review'){
    $taskIdentity=Load (Join-Path $taskPacket 'identity.json')
    $Baseline=$taskIdentity.baseline_source;$Candidate=$taskIdentity.candidate_source
}else{
    Need ($Baseline -match '^[a-f0-9]{40}$' -and $Candidate -match '^[a-f0-9]{40}$') 'Exact source commits required'
}
& git diff --quiet $Candidate -- tools/long-run-supervisor-074 tools/local-profile-session-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
Need ($LASTEXITCODE -eq 0) 'Candidate source drift'
Need ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification drift'
foreach($taskMethod in @('check_projection_groups.ps1','review_projection_groups.ps1')){
    Need ((Hash "tools/local-profile-session-074/$taskMethod") -ceq (Hash (Join-Path $taskPacket "method/$taskMethod"))) 'Method snapshot drift'
}
foreach($taskStage in @('baseline','candidate')){
    foreach($taskLane in @('host','194','188')){
        $taskResult=Load (Join-Path $taskPacket "$taskStage/$taskLane-result.json")
        Need ($taskResult.stage -ceq $taskStage -and $taskResult.lane -ceq $taskLane -and $null -eq $taskResult.failure -and $taskResult.original_owner_profiles -eq 0 -and $taskResult.ignored_tests_run -is [bool] -and -not $taskResult.ignored_tests_run -and $taskResult.server -is [bool] -and -not $taskResult.server -and -not $taskResult.product) 'Check scope/outcome drift'
        $taskExpected=if($taskLane -ceq 'host' -or $taskStage -ceq 'baseline'){3}else{6}
        Need ($taskResult.steps.Count -eq $taskExpected) 'Check step count'
        foreach($taskStep in $taskResult.steps){Need ($taskStep.exit -eq 0 -and $taskStep.sha256 -ceq (Hash (Join-Path $taskPacket "$taskStage/$($taskStep.log)"))) 'Check log drift'}
    }
}
function Rows([string]$Stage,[string]$Lane,[string]$Arm){
    @(Get-Content -LiteralPath (Join-Path $taskPacket "$Stage/$Lane-allocations.log")|Where-Object {$_.Contains("HC074-PROJECTION-GROUPS-$Arm ")}|ForEach-Object {($_ -split "HC074-PROJECTION-GROUPS-$Arm ",2)[1]|ConvertFrom-Json})
}
$taskLanes=@()
foreach($taskLane in @('194','188')){
    $taskBefore=Rows 'baseline' $taskLane 'REFERENCE'
    $taskReference=Rows 'candidate' $taskLane 'REFERENCE'
    $taskAfter=Rows 'candidate' $taskLane 'CANDIDATE'
    Need ($taskBefore.Count -eq 7 -and $taskReference.Count -eq 7 -and $taskAfter.Count -eq 7) 'Missing allocation cases'
    $taskLengths=@(0,1,4,5,16,32,33);$taskOldBytes=0L;$taskNewBytes=0L;$taskOldRealloc=0L
    for($taskIndex=0;$taskIndex -lt 7;$taskIndex++){
        $taskOld=$taskBefore[$taskIndex];$taskRef=$taskReference[$taskIndex];$taskNew=$taskAfter[$taskIndex];$taskLength=$taskLengths[$taskIndex]
        foreach($taskRow in @($taskOld,$taskRef,$taskNew)){Need ($taskRow.length -eq $taskLength -and $taskRow.iterations -eq 256 -and $taskRow.profile -is [bool] -and -not $taskRow.profile -and -not $taskRow.server -and -not $taskRow.product) 'Allocation case scope drift'}
        Need (($taskOld.counts|ConvertTo-Json -Compress) -ceq ($taskRef.counts|ConvertTo-Json -Compress)) 'Reference changed'
        $taskAlloc=if($taskLength -eq 0){0}else{256}
        Need ($taskNew.counts.allocations -eq $taskAlloc -and $taskNew.counts.reallocations -eq 0 -and $taskNew.counts.requested_bytes -eq 4*$taskLength*256) 'Exact allocation hypothesis refused'
        Need ($taskNew.counts.requested_bytes -le $taskOld.counts.requested_bytes) 'Byte request regression'
        $taskOldBytes+=$taskOld.counts.requested_bytes;$taskNewBytes+=$taskNew.counts.requested_bytes;$taskOldRealloc+=$taskOld.counts.reallocations
    }
    Need ($taskOldBytes -gt $taskNewBytes -and $taskOldRealloc -gt 0) 'No measured reduction'
    $taskLanes+=@{lane=$taskLane;invocations=1792;reference_requested_bytes=$taskOldBytes;candidate_requested_bytes=$taskNewBytes;reference_reallocations=$taskOldRealloc;candidate_reallocations=0}
}
$taskSummary=[ordered]@{baseline_source=$Baseline;candidate_source=$Candidate;lanes=$taskLanes;allocation_only=$true;timing_improvement_proven=$false;original_material_gate_passed=$false;original_windows_seconds=@(300,60,10);first_refusal_ages_ns=@(10042390247,10010835739);original_owner_profiles=0;server=$false;product=$false}
if($Mode -ceq 'Seal'){
    Need (-not(Test-Path (Join-Path $taskPacket 'inventory.json'))) 'Never replace seal'
    WriteNew (Join-Path $taskPacket 'identity.json') @{baseline_source=$Baseline;candidate_source=$Candidate}
    WriteNew (Join-Path $taskPacket 'summary.json') $taskSummary
    $taskFiles=@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File|Sort-Object FullName|ForEach-Object {@{path=[IO.Path]::GetRelativePath($taskPacket,$_.FullName).Replace([char]92,[char]47);bytes=$_.Length;sha256=(Hash $_.FullName)}})
    WriteNew (Join-Path $taskPacket 'inventory.json') @{files=$taskFiles}
}
Need (($taskSummary|ConvertTo-Json -Depth 20 -Compress) -ceq ((Load (Join-Path $taskPacket 'summary.json'))|ConvertTo-Json -Depth 20 -Compress)) 'Summary drift'
$taskSeal=Load (Join-Path $taskPacket 'inventory.json')
Members $taskPacket $taskSeal
Write-Output "Projection groups verified: $($taskSeal.files.Count) members; inventory $(Hash (Join-Path $taskPacket 'inventory.json')); allocation only, no live replay."
