$ErrorActionPreference='Stop'
if(Test-Path "$PSScriptRoot/inventory.json"){throw 'Never replace an inventory'}
$taskFacts=((& "$PSScriptRoot/verify.ps1" -Review)|Out-String)|ConvertFrom-Json
$taskSummary=Get-Content -Raw "$PSScriptRoot/summary.json"|ConvertFrom-Json
if((ConvertTo-Json -Depth 30 -Compress -InputObject $taskFacts) -cne (ConvertTo-Json -Depth 30 -Compress -InputObject $taskSummary)){throw 'Review summary mismatch'}
if(Get-ChildItem $PSScriptRoot -Recurse -Force|Where-Object{($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0}){throw 'No reparse member'}
$taskNames=[string[]]@(Get-ChildItem $PSScriptRoot -Recurse -File -Force|ForEach-Object{$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/')})
[Array]::Sort($taskNames,[StringComparer]::Ordinal)
$taskRows=@($taskNames|ForEach-Object{[ordered]@{path=$_;bytes=(Get-Item -LiteralPath "$PSScriptRoot/$_").Length;sha256=(Get-FileHash -LiteralPath "$PSScriptRoot/$_" -Algorithm SHA256).Hash.ToLowerInvariant()}})
[ordered]@{schema='sha256-local-file-inventory-v1';files=$taskRows}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/inventory.json"
& "$PSScriptRoot/verify.ps1"
Get-FileHash "$PSScriptRoot/inventory.json" -Algorithm SHA256
