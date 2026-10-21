#requires -Version 7.2
param([ValidateSet('Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Need($Ok,[string]$Reason){if(-not $Ok){throw $Reason}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
$taskAnalyzer='5536087e7ad1acfa1af66f796631bdfb4338d2e0'
$taskMeasured='1d6d179bd7150ef564560821a897794e20b24b36'
& git diff --quiet $taskMeasured -- tools/long-run-supervisor-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
Need ($LASTEXITCODE -eq 0) 'Runtime/live checks/origins changed'
& git diff --quiet $taskAnalyzer -- tools/local-profile-session-074
Need ($LASTEXITCODE -eq 0) 'Frozen offline reader/method changed'
foreach($taskName in @('attribute_material_rounds.py','test_attribute_material_rounds.py')){
    Need ((Hash "tools/local-profile-session-074/$taskName") -ceq (Hash (Join-Path $PSScriptRoot "method/$taskName"))) 'Retained method drift'
}
& python -B tools/local-profile-session-074/attribute_material_rounds.py --output "$PSScriptRoot/facts.json" --verify
Need ($LASTEXITCODE -eq 0) 'Offline input/facts reconstruction refused'
$taskFacts=Json 'facts.json'
Need ($taskFacts.measured_source -ceq $taskMeasured -and $taskFacts.input_members_verified -eq 65 -and ($taskFacts.new_profile_invocations -is [long] -or $taskFacts.new_profile_invocations -is [int]) -and $taskFacts.new_profile_invocations -eq 0) 'Provenance/scope changed'
foreach($taskFlag in @('runtime_changed','observer_changed','original_material_gate_passed','full_sigkill_matrix','native_gate_open','original_send','server','product')){
    Need ($taskFacts.$taskFlag -is [bool] -and $taskFacts.$taskFlag -ceq $false) "Scope flag drift: $taskFlag"
}
Need ($taskFacts.analyzer_sha256 -ceq (Hash 'tools/local-profile-session-074/attribute_material_rounds.py')) 'Analyzer digest drift'
foreach($taskLog in @(@('tests-frozen.log',15),@('prior-material-reader.log',9),@('prior-arithmetic-reader.log',18))){
    $taskText=Get-Content -LiteralPath (Join-Path $PSScriptRoot "checks/$($taskLog[0])") -Raw
    Need ($taskText -match "Ran $($taskLog[1]) tests" -and $taskText -match '(?m)^OK\s*$' -and $taskText -notmatch 'FAILED \(') 'Offline tests incomplete'
}
Need ((Get-Content (Join-Path $PSScriptRoot 'checks/doc-check.log') -Raw) -match 'doc-check: OK') 'Docs refused'
$taskCanaries=Get-Content (Join-Path $PSScriptRoot 'checks/source-canaries.log') -Raw
$taskMatches=[regex]::Matches($taskCanaries,'test result: ok\. (\d+) passed; 0 failed; 0 ignored;')
Need ($taskMatches.Count -eq 5 -and ($taskMatches|ForEach-Object {[int]$_.Groups[1].Value}|Measure-Object -Sum).Sum -eq 14) 'Source canaries incomplete'
foreach($taskLog in @('xtask-check.log','xtask-clippy.log')){Need ((Get-Content (Join-Path $PSScriptRoot "checks/$taskLog") -Raw) -match 'Finished `dev`') 'Static check incomplete'}
Need ((Get-Content (Join-Path $PSScriptRoot 'checks/registry.log') -Raw) -match 'OK \(186 discovered gates, all registered\)') 'Registry refused'
Need ((Get-Content (Join-Path $PSScriptRoot 'checks/performance-contract.log') -Raw) -match 'OK \(local, non-promotable\)') 'Local performance contract refused'
$taskInventory=Join-Path $PSScriptRoot 'inventory.json'
if($Mode -ceq 'Seal'){
    Need (-not(Test-Path $taskInventory)) 'Never overwrite attribution seal'
    $taskFiles=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File|Sort-Object FullName|ForEach-Object {@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace([char]92,[char]47);bytes=$_.Length;sha256=(Hash $_.FullName)}})
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject @{analyzer_source=$taskAnalyzer;measured_source=$taskMeasured;files=$taskFiles} -Depth 8)+"`n")
    $taskStream=[IO.File]::Open($taskInventory,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
$taskSeal=Json 'inventory.json';$taskSeen=@{}
Need ($taskSeal.analyzer_source -ceq $taskAnalyzer -and $taskSeal.measured_source -ceq $taskMeasured) 'Inventory sources drift'
Need (@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force -Attributes ReparsePoint).Count -eq 0) 'Reparse point in attribution'
foreach($taskRow in $taskSeal.files){
    Need (-not[IO.Path]::IsPathRooted($taskRow.path) -and -not $taskRow.path.Contains([char]92) -and -not($taskRow.path.Split([char]47) -contains '..') -and -not $taskSeen.ContainsKey($taskRow.path)) 'Unsafe/duplicate member'
    $taskSeen[$taskRow.path]=$true
    $taskItem=Get-Item -LiteralPath (Join-Path $PSScriptRoot $taskRow.path)
    Need ($taskItem.Length -eq $taskRow.bytes -and (Hash $taskItem.FullName) -ceq $taskRow.sha256) 'Attribution member drift'
}
Need (@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File).Count -eq $taskSeen.Count+1) 'Attribution membership drift'
Write-Output "Offline attribution verified: $($taskSeen.Count) members; inventory $(Hash $taskInventory); first refusals unchanged; live invocations=0."
