$ErrorActionPreference='Stop'
$taskInventory=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'inventory.json') | ConvertFrom-Json
if($taskInventory.schema -cne 'sha256-local-file-inventory-v1'){throw 'Inventory schema'}
$taskPaths=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($taskEntry in $taskInventory.files){
    if(-not $taskEntry.path -or $taskEntry.path -match '[:\\]' -or $taskEntry.path.StartsWith('/') -or $taskEntry.path.Split('/') -contains '..' -or $taskEntry.path.Split('/') -contains '.' -or $taskEntry.path -ceq 'inventory.json' -or -not $taskPaths.Add($taskEntry.path)){throw 'Invalid member'}
    $taskFile=Get-Item -LiteralPath (Join-Path $PSScriptRoot $taskEntry.path)
    if($taskFile.PSIsContainer -or $taskFile.Attributes -band [IO.FileAttributes]::ReparsePoint -or $taskFile.Length -ne $taskEntry.bytes -or (Get-FileHash -LiteralPath $taskFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskEntry.sha256){throw "Changed member: $($taskEntry.path)"}
}
[string[]]$taskExpected=@($taskInventory.files.path)
[string[]]$taskActual=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object Name -CNE 'inventory.json' | ForEach-Object {$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/')})
[Array]::Sort($taskActual,[StringComparer]::Ordinal)
if(($taskActual -join "`n") -cne ($taskExpected -join "`n")){throw 'Unlisted, missing or unsorted member'}
$taskReceipt=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'receipt.json') | ConvertFrom-Json
if($taskReceipt.source -cne '148bb3fe25cb94d329e6a35468ecd6838a88fed7' -or $taskReceipt.windows_artifact_tests -ne 17 -or $taskReceipt.artifact_tests_per_linux_compiler -ne 31 -or $taskReceipt.protocol_tests_per_retained_linux_image -ne 11 -or $taskReceipt.existing_regressions_passed -cne $true -or $taskReceipt.recipe -cne (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'audit.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()){throw 'Regression binding'}
foreach($taskFlag in @('runtime_changed','held_artifact_admission','topology_approved','original_send','server','supervisor','product','new_live_or_fault_matrix','clock_changed')){
    if($taskReceipt.$taskFlag -isnot [bool] -or $taskReceipt.$taskFlag -cne $false){throw 'Audit scope upgrade'}
}
[ordered]@{verified=$true;files=$taskPaths.Count;inventory_sha256=(Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'inventory.json') -Algorithm SHA256).Hash.ToLowerInvariant();topology_approved=$false;held_artifact_admission=$false;server=$false;product=$false} | ConvertTo-Json
