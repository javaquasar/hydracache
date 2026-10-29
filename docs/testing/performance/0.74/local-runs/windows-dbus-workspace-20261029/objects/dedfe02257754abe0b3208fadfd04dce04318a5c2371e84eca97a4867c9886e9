#requires -Version 7.2
param([ValidateSet('Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/held-material-live-20261021'
$taskSource='1d6d179bd7150ef564560821a897794e20b24b36'
function Need($Ok,[string]$Reason){if(-not $Ok){throw $Reason}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $taskPacket $Name) -Raw|ConvertFrom-Json}
& git diff --quiet $taskSource HEAD -- tools/long-run-supervisor-074 tools/local-profile-session-074 ':!tools/local-profile-session-074/seal_material_live.ps1' crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
Need ($LASTEXITCODE -eq 0) 'Frozen runtime/driver drift'
$taskMetadata=Json 'metadata.json';$taskResult=Json 'result.json';$taskSummary=Json 'summary.json'
$taskLaunch=Json 'held-material-live-20261021-launch-exit.json'
Need ($taskMetadata.source -ceq $taskSource -and $taskResult.source -ceq $taskSource -and $taskLaunch.source -ceq $taskSource) 'Source binding'
Need ($taskResult.failure -eq $null -and $taskResult.schedule_completed -ceq $false -and $taskResult.profiles.Count -eq 2) 'First refused outcome changed'
Need ($taskLaunch.exit -eq 1 -and $taskLaunch.single_profile_session -ceq $true) 'Launch outcome changed'
Need ($taskLaunch.session_log_sha256 -ceq (Hash (Join-Path $taskPacket 'held-material-live-20261021-session.log')) -and $taskLaunch.recipe_sha256 -ceq (Hash (Join-Path $taskPacket 'method/launch_material_live.ps1'))) 'Launch log/recipe binding'
Need ($taskMetadata.pins_sha256 -ceq (Hash (Join-Path $taskPacket 'held-material-live-20261021-pins.json')) -and $taskMetadata.preparation_sha256 -ceq (Hash (Join-Path $taskPacket 'held-material-live-20261021-preparation.json'))) 'Pins/preparation binding'
Need ($taskMetadata.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification drift'
Need ($taskSummary.original_material_gate_passed -ceq $false -and $taskSummary.full_sigkill_matrix -ceq $false -and $taskSummary.original_delivery_proven -ceq $false -and $taskSummary.all_production_writers_proven -ceq $false) 'Scope claim drift'
foreach($taskObject in @($taskResult,$taskSummary)){
    foreach($taskFlag in @('server','product','original_send','native_gate_open')){Need ($taskObject.$taskFlag -is [bool] -and $taskObject.$taskFlag -ceq $false) "Scope flag $taskFlag"}
}
$taskFresh=Json 'held-material-live-20261021-preparation.json';$taskPins=Json 'held-material-live-20261021-pins.json'
foreach($taskLane in @('194','188')){
    $taskRow=$taskFresh.$taskLane
    Need ($taskRow.source -ceq $taskSource -and $taskRow.exit -eq 0 -and $taskRow.sha256 -ceq $taskMetadata.images.$taskLane -and $taskRow.sha256 -ceq $taskPins.images.$taskLane) 'Fresh image binding'
    Need ($taskRow.build_log_sha256 -ceq (Hash (Join-Path $taskPacket $taskRow.build_log))) 'Build log binding'
    $taskChecks=Json "checks/pwsh/$taskLane-result.json"
    Need ($taskChecks.failure -eq $null -and $taskChecks.steps.Count -eq 7) 'Linux checks incomplete'
    foreach($taskCheck in $taskChecks.steps){Need ($taskCheck.exit -eq 0 -and $taskCheck.sha256 -ceq (Hash (Join-Path $taskPacket "checks/pwsh/$($taskCheck.log)"))) 'Linux check evidence drift'}
}
$taskChecks=Json 'checks/pwsh/host-result.json'
Need ($taskChecks.failure -eq $null -and $taskChecks.steps.Count -eq 8) 'Host checks incomplete'
foreach($taskCheck in $taskChecks.steps){Need ($taskCheck.exit -eq 0 -and $taskCheck.sha256 -ceq (Hash (Join-Path $taskPacket "checks/pwsh/$($taskCheck.log)"))) 'Host check evidence drift'}
$taskMethod=$taskMetadata.material_method
foreach($taskName in $taskMethod.PSObject.Properties.Name){Need ($taskMethod.$taskName -ceq (Hash (Join-Path $taskPacket "method/$taskName")) -and $taskMethod.$taskName -ceq (Hash (Join-Path $PSScriptRoot $taskName))) 'Method bytes changed'}
& python -B tools/local-profile-session-074/review_material_live.py --source $taskSource --output "$taskPacket/summary.json" --verify
Need ($LASTEXITCODE -eq 0) 'Offline original reconstruction failed'
$taskInventory=Join-Path $taskPacket 'inventory.json'
if($Mode -ceq 'Seal'){
    Need (-not(Test-Path $taskInventory)) 'Never overwrite an inventory'
    $taskMembers=@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File|Sort-Object FullName|ForEach-Object {@{path=[IO.Path]::GetRelativePath($taskPacket,$_.FullName).Replace([char]92,[char]47);bytes=$_.Length;sha256=(Hash $_.FullName)}})
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject @{source=$taskSource;files=$taskMembers} -Depth 8)+"`n")
    $taskStream=[IO.File]::Open($taskInventory,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
$taskSealed=Json 'inventory.json';$taskSeen=@{}
Need ($taskSealed.source -ceq $taskSource) 'Seal source drift'
Need (@(Get-ChildItem -LiteralPath $taskPacket -Recurse -Force -Attributes ReparsePoint).Count -eq 0) 'Reparse point in packet'
foreach($taskMember in $taskSealed.files){
    Need (-not[IO.Path]::IsPathRooted($taskMember.path) -and -not $taskMember.path.Contains([char]92) -and -not($taskMember.path.Split([char]47) -contains '..') -and -not $taskSeen.ContainsKey($taskMember.path)) 'Unsafe/duplicate inventory path'
    $taskSeen[$taskMember.path]=$true
    $taskPath=Join-Path $taskPacket $taskMember.path
    $taskItem=Get-Item -LiteralPath $taskPath
    Need (-not($taskItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -and $taskItem.Length -eq $taskMember.bytes -and (Hash $taskPath) -ceq $taskMember.sha256) 'Inventory member drift'
}
Need (@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File).Count -eq $taskSeen.Count+1) 'Inventory membership drift'
Write-Output "Original material first-refusal packet verified: $($taskSeen.Count) members; inventory $(Hash $taskInventory); no retry/server admission."
