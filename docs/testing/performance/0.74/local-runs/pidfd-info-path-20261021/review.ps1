#requires -Version 7.2
param([ValidateSet('Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskBaseline='1c7e00f327e45be8ec543fcd9b8da21faf481874'
$taskCandidate='0cb604afdc14adb544ba613fd6d46189401254b4'
function Need($Ok,[string]$Reason){if(-not $Ok){throw $Reason}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function WriteNew([string]$Path,$Value){
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject $Value -Depth 20)+"`n")
    $taskStream=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
function Members([string]$Directory,$Seal){
    $taskSeen=@{}
    Need (@(Get-ChildItem -LiteralPath $Directory -Recurse -Force -Attributes ReparsePoint).Count -eq 0) 'Reparse point in evidence'
    foreach($taskRow in $Seal.files){
        Need (-not[IO.Path]::IsPathRooted($taskRow.path) -and -not $taskRow.path.Contains([char]92) -and -not($taskRow.path.Split([char]47) -contains '..') -and -not $taskSeen.ContainsKey($taskRow.path)) 'Unsafe/duplicate member'
        $taskSeen[$taskRow.path]=$true
        $taskItem=Get-Item -LiteralPath (Join-Path $Directory $taskRow.path)
        Need ($taskItem.Length -eq $taskRow.bytes -and (Hash $taskItem.FullName) -ceq $taskRow.sha256) 'Evidence member drift'
    }
    Need (@(Get-ChildItem -LiteralPath $Directory -Recurse -File).Count -eq $taskSeen.Count+1) 'Evidence membership drift'
}
foreach($taskPin in @(@('held-material-live-20261021','0c385fbfe209fad1903aa2c0e620817c458f413eec8bd36fa32be5c01fbb4109'),@('held-material-round-attribution-20261021','84c2d0a10e1d2d66b4dc76e1c041b9920d90d85621461bc3e83102569a6e1a2f'))){
    $taskPrior=Join-Path (Split-Path $PSScriptRoot) $taskPin[0]
    Need ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq $taskPin[1]) 'Prior seal drift'
    Members $taskPrior (Get-Content (Join-Path $taskPrior 'inventory.json') -Raw|ConvertFrom-Json)
}
& git diff --quiet $taskCandidate -- tools/long-run-supervisor-074 tools/local-profile-session-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
Need ($LASTEXITCODE -eq 0) 'Candidate source drift'
$taskOrigin='tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs'
$taskOld=(& git show "${taskBaseline}:$taskOrigin") -join "`n"
Need ($LASTEXITCODE -eq 0) 'Baseline unavailable'
Need ($taskOld.TrimEnd() -ceq (Get-Content (Join-Path $PSScriptRoot 'baseline/source/diagnostic_kernel_origin.rs') -Raw).Replace("`r",'').TrimEnd()) 'Baseline origin snapshot drift'
foreach($taskFile in @('diagnostic_kernel_origin.rs','diagnostic_pidfd_info_path.rs')){Need ((Hash "tools/long-run-supervisor-074/src/$taskFile") -ceq (Hash (Join-Path $PSScriptRoot "candidate/source/$taskFile"))) 'Candidate source snapshot drift'}
foreach($taskFile in @('pidfd_info_path.rs','pidfd_info_path_alloc.rs')){Need ((Hash "tools/long-run-supervisor-074/tests/support/$taskFile") -ceq (Hash (Join-Path $PSScriptRoot "candidate/source/$taskFile"))) 'Candidate test snapshot drift'}
Need ((Hash 'tools/local-profile-session-074/check_pidfd_info_path.ps1') -ceq (Hash (Join-Path $PSScriptRoot 'method/check_pidfd_info_path.ps1'))) 'Recipe drift'
Need ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Original qualification changed'
function Unit([string]$Name,[int]$Count){
    $taskText=Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw
    Need ($taskText -match "test result: ok\. $Count passed; 0 failed; 0 ignored;" -and $taskText -notmatch 'test result: FAILED') "Unit evidence refused: $Name"
}
function Rows([string]$Name,[string]$Marker){
    @(Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name)|Where-Object {$_.Contains($Marker)}|ForEach-Object {($_ -split [regex]::Escape($Marker),2)[1]|ConvertFrom-Json})
}
$taskLanes=@();$taskDescriptors=@(-2147483648,-1,0,1,9,10,99,100,2147483647)
foreach($taskStage in @('baseline','candidate')){
    foreach($taskLane in @('host','194','188')){
        $taskCheck=Json "$taskStage/checks/$taskLane-result.json"
        Need ($taskCheck.failure -eq $null -and $taskCheck.stage -ceq $taskStage -and $taskCheck.lane -ceq $taskLane -and $taskCheck.original_owner_profiles -eq 0 -and $taskCheck.ignored_tests_run -is [bool] -and $taskCheck.ignored_tests_run -eq $false -and $taskCheck.server -is [bool] -and $taskCheck.server -eq $false -and $taskCheck.product -eq $false) 'Check scope/outcome drift'
        $taskCount=if($taskLane -ceq 'host'){3}elseif($taskStage -ceq 'baseline'){2}else{7}
        Need ($taskCheck.steps.Count -eq $taskCount) 'Incomplete check steps'
        foreach($taskStep in $taskCheck.steps){Need ($taskStep.exit -eq 0 -and $taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot "$taskStage/checks/$($taskStep.log)"))) 'Check log drift'}
    }
}
foreach($taskLane in @('194','188')){
    Unit "baseline/pidfd-info-baseline-$taskLane.log" 1
    foreach($taskCase in @(@('pidfd_info_path',6),@('document_scratch',4),@('boot_comparison',4),@('consuming_join',10),@('allocations',8))){Unit "candidate/checks/$taskLane-$($taskCase[0]).log" $taskCase[1]}
    $taskBefore=Rows "baseline/pidfd-info-baseline-$taskLane.log" 'HC074-PIDFD-PATH-REFERENCE '
    $taskReference=Rows "candidate/checks/$taskLane-allocations.log" 'HC074-PIDFD-PATH-REFERENCE '
    $taskAfter=Rows "candidate/checks/$taskLane-allocations.log" 'HC074-PIDFD-PATH-CANDIDATE '
    Need ($taskBefore.Count -eq 9 -and $taskReference.Count -eq 9 -and $taskAfter.Count -eq 9) 'Allocation case count drift'
    for($taskIndex=0;$taskIndex -lt 9;$taskIndex++){
        foreach($taskRow in @($taskBefore[$taskIndex],$taskReference[$taskIndex],$taskAfter[$taskIndex])){
            Need ($taskRow.fd -eq $taskDescriptors[$taskIndex] -and $taskRow.iterations -eq 256 -and $taskRow.rust_requests_only -is [bool] -and $taskRow.rust_requests_only -eq $true -and $taskRow.profile -is [bool] -and $taskRow.profile -eq $false -and $taskRow.server -eq $false -and $taskRow.product -eq $false) 'Allocation input/scope drift'
        }
        foreach($taskRow in @($taskBefore[$taskIndex],$taskReference[$taskIndex])){Need ($taskRow.reference.allocations -eq 256 -and $taskRow.reference.reallocations -eq 0 -and $taskRow.reference.requested_bytes -eq 12800) 'Reference requests drift'}
        $taskRow=$taskAfter[$taskIndex]
        Need ($taskRow.candidate.allocations -eq 0 -and $taskRow.candidate.reallocations -eq 0 -and $taskRow.candidate.requested_bytes -eq 0) 'Candidate request result differs'
    }
    $taskLanes+=[ordered]@{lane=$taskLane;cases=9;iterations_per_case=256;reference_allocations=2304;reference_requested_bytes=115200;candidate_allocations=0;candidate_reallocations=0;candidate_requested_bytes=0;selected_unit_executions=32}
}
Need ((Get-Item (Join-Path $PSScriptRoot 'final/format-final.log')).Length -eq 0) 'Final format not clean'
Need ((Get-Content (Join-Path $PSScriptRoot 'final/docs.log') -Raw) -match 'doc-check: OK') 'Documentation refused'
Need ((Get-Content (Join-Path $PSScriptRoot 'final/registry.log') -Raw) -match 'OK \(186 discovered gates, all registered\)') 'Registry refused'
Need ((Get-Content (Join-Path $PSScriptRoot 'final/performance.log') -Raw) -match 'OK \(local, non-promotable\)') 'Local contract refused'
$taskSummary=[ordered]@{baseline_source=$taskBaseline;candidate_source=$taskCandidate;lanes=$taskLanes;allocation_only_candidate=$true;timing_improvement_proven=$false;native_api_improvement_proven=$false;original_material_gate_passed=$false;original_owner_profiles=0;original_windows_seconds=@(300,60,10);first_refusal_ages_ns=@(10042390247,10010835739);server=$false;product=$false}
$taskInventory=Join-Path $PSScriptRoot 'inventory.json'
if($Mode -ceq 'Seal'){
    Need (-not(Test-Path $taskInventory)) 'Never replace candidate seal'
    WriteNew (Join-Path $PSScriptRoot 'summary.json') $taskSummary
    $taskFiles=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File|Sort-Object FullName|ForEach-Object {@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace([char]92,[char]47);bytes=$_.Length;sha256=(Hash $_.FullName)}})
    WriteNew $taskInventory @{baseline_source=$taskBaseline;candidate_source=$taskCandidate;files=$taskFiles}
}
Need ((ConvertTo-Json -InputObject (Json 'summary.json') -Depth 20 -Compress) -ceq (ConvertTo-Json -InputObject $taskSummary -Depth 20 -Compress)) 'Derived allocation summary differs'
$taskSeal=Json 'inventory.json'
Need ($taskSeal.baseline_source -ceq $taskBaseline -and $taskSeal.candidate_source -ceq $taskCandidate) 'Seal identity drift'
Members $PSScriptRoot $taskSeal
Write-Output "Pidfd pathname candidate verified: $($taskSeal.files.Count) members; inventory $(Hash $taskInventory); no live replay or timing claim."
