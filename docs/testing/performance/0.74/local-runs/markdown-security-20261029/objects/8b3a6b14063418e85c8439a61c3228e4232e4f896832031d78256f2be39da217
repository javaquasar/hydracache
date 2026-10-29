#requires -Version 7.2
param([ValidateSet('Seal','Review')][string]$Mode='Review',[string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/reserved-leaf-audit-20261021'
function Need($Ok,[string]$Why){if(-not $Ok){throw $Why}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Path){Get-Content -LiteralPath $Path -Raw|ConvertFrom-Json}
function WriteNew([string]$Path,$Value){
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject $Value -Depth 20)+"`n")
    $taskStream=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
if($Mode -ceq 'Review'){$Source=(Json (Join-Path $taskPacket 'identity.json')).audit_source}
Need ($Source -match '^[a-f0-9]{40}$') 'Exact audit source required'
& git diff --quiet $Source -- tools/offline-reserved-leaf-audit-074
Need ($LASTEXITCODE -eq 0) 'Audit implementation drift'
& git diff --quiet e59190ffa431f36d9fc37998e60b368bcc2a1623 -- tools/long-run-supervisor-074 crates Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
Need ($LASTEXITCODE -eq 0) 'Runtime/protected input drift'
foreach($taskName in @('audit.py','test_audit.py','check.ps1','review.ps1','README.md')){
    Need ((Hash (Join-Path $PSScriptRoot $taskName)) -ceq (Hash (Join-Path $taskPacket "method/$taskName"))) 'Audit method snapshot drift'
}
# Pure recomputation only: no Rust, WSL, clock, profile, service or network entry.
& python -B tools/offline-reserved-leaf-audit-074/audit.py --output docs/testing/performance/0.74/local-runs/reserved-leaf-audit-20261021/facts.json --verify
Need ($LASTEXITCODE -eq 0) 'Offline audit facts refused'
$taskChecks=Json (Join-Path $taskPacket 'checks/result.json')
$taskExpected=@('audit-controls','round-controls','reader-controls','arithmetic-controls','derived-facts','source-canaries','check','clippy','docs','registry','performance','protected-source')
Need (($taskChecks.steps.name -join ',') -ceq ($taskExpected -join ',') -and $taskChecks.new_profile_invocations -eq 0 -and $taskChecks.runtime_changed -is [bool] -and -not $taskChecks.runtime_changed -and -not $taskChecks.observer_changed -and -not $taskChecks.server -and -not $taskChecks.product) 'Check sequence/scope drift'
foreach($taskStep in $taskChecks.steps){Need ($taskStep.exit -eq 0 -and $taskStep.sha256 -ceq (Hash (Join-Path $taskPacket "checks/$($taskStep.log)"))) 'Check evidence drift'}
foreach($taskPair in @(@('audit-controls',11),@('round-controls',15),@('reader-controls',9),@('arithmetic-controls',18))){
    $taskText=Get-Content -LiteralPath (Join-Path $taskPacket "checks/$($taskPair[0]).log") -Raw
    Need ($taskText -match "Ran $($taskPair[1]) tests" -and $taskText -match '(?m)^OK\s*$' -and $taskText -notmatch 'FAILED') 'Python controls refused'
}
$taskCanaries=Get-Content -LiteralPath (Join-Path $taskPacket 'checks/source-canaries.log') -Raw
$taskCount=0
foreach($taskMatch in [regex]::Matches($taskCanaries,'test result: ok\. (\d+) passed; 0 failed; 0 ignored;')){$taskCount += [int]$taskMatch.Groups[1].Value}
Need ($taskCount -eq 19 -and $taskCanaries -notmatch 'test result: FAILED') 'Source canaries refused'
Need ((Get-Item (Join-Path $taskPacket 'checks/protected-source.log')).Length -eq 0) 'Runtime comparison not empty'
Need ((Get-Content (Join-Path $taskPacket 'checks/docs-final.log') -Raw) -match 'doc-check: OK') 'Final docs refused'
Need ((Get-Content (Join-Path $taskPacket 'checks/registry.log') -Raw) -match 'OK \(186 discovered gates, all registered\)') 'Registry refused'
Need ((Get-Content (Join-Path $taskPacket 'checks/performance.log') -Raw) -match 'OK \(local, non-promotable\)') 'Performance contract refused'
$taskInventory=Join-Path $taskPacket 'inventory.json'
if($Mode -ceq 'Seal'){
    Need (-not(Test-Path $taskInventory)) 'Never replace seal'
    WriteNew (Join-Path $taskPacket 'identity.json') @{audit_source=$Source;audited_runtime_source='e59190ffa431f36d9fc37998e60b368bcc2a1623';new_profile_invocations=0}
    $taskFiles=@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File|Sort-Object FullName|ForEach-Object {@{path=[IO.Path]::GetRelativePath($taskPacket,$_.FullName).Replace([char]92,[char]47);bytes=$_.Length;sha256=(Hash $_.FullName)}})
    WriteNew $taskInventory @{audit_source=$Source;files=$taskFiles}
}
$taskSeal=Json $taskInventory
Need ($taskSeal.audit_source -ceq $Source) 'Audit identity drift'
$taskSeen=@{}
Need (@(Get-ChildItem -LiteralPath $taskPacket -Recurse -Force -Attributes ReparsePoint).Count -eq 0) 'Linked evidence'
foreach($taskRow in $taskSeal.files){
    Need (-not[IO.Path]::IsPathRooted($taskRow.path) -and -not $taskRow.path.Contains([char]92) -and -not($taskRow.path.Split([char]47) -contains '..') -and -not $taskSeen.ContainsKey($taskRow.path)) 'Unsafe member'
    $taskSeen[$taskRow.path]=$true
    $taskItem=Get-Item -LiteralPath (Join-Path $taskPacket $taskRow.path)
    Need ($taskItem.Length -eq $taskRow.bytes -and (Hash $taskItem.FullName) -ceq $taskRow.sha256) 'Evidence member drift'
}
Need (@(Get-ChildItem -LiteralPath $taskPacket -Recurse -File).Count -eq $taskSeen.Count+1) 'Membership drift'
Write-Output "Offline leaf audit verified: $($taskSeen.Count) members; inventory $(Hash $taskInventory); runtime unchanged."
