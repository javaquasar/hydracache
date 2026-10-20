$ErrorActionPreference='Stop'
if((Test-Path "$PSScriptRoot/summary.json") -or (Test-Path "$PSScriptRoot/inventory.json")){throw 'Never replace sealed evidence'}
$taskSummary=(& "$PSScriptRoot/verify.ps1" -Review|Out-String)|ConvertFrom-Json
$taskSummary|ConvertTo-Json -Depth 60|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/summary.json"
$taskRoot=(Resolve-Path $PSScriptRoot).Path
$taskNames=[string[]]@(Get-ChildItem $taskRoot -File -Recurse -Force|ForEach-Object{$_.FullName.Substring($taskRoot.Length+1).Replace('\','/')})
[Array]::Sort($taskNames,[StringComparer]::Ordinal)
$taskFiles=@($taskNames|ForEach-Object{$taskItem=Get-Item -LiteralPath (Join-Path $taskRoot $_);[ordered]@{path=$_;bytes=$taskItem.Length;sha256=(Get-FileHash -LiteralPath $taskItem.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}})
[ordered]@{schema='sha256-local-file-inventory-v1';files=$taskFiles}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM "$taskRoot/inventory.json"
& "$PSScriptRoot/verify.ps1"
