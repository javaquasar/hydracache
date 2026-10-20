$ErrorActionPreference='Stop'
if(Test-Path "$PSScriptRoot/inventory.json"){throw 'Never replace a sealed inventory'}
& "$PSScriptRoot/verify.ps1" -UnsealedReview
[string[]]$taskNames=@(Get-ChildItem $PSScriptRoot -Recurse -Force -File|ForEach-Object{$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/')})
[Array]::Sort($taskNames,[StringComparer]::Ordinal)
$taskMembers=@(foreach($taskName in $taskNames){
    $taskFile=Get-Item -LiteralPath "$PSScriptRoot/$taskName"
    [ordered]@{path=$taskName;bytes=$taskFile.Length;sha256=(Get-FileHash -LiteralPath $taskFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
})
[ordered]@{schema='sha256-local-file-inventory-v1';files=$taskMembers}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/inventory.json"
& "$PSScriptRoot/verify.ps1"
