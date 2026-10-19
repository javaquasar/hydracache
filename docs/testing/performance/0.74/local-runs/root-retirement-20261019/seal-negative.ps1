$ErrorActionPreference = 'Stop'
$taskInventory = Join-Path $PSScriptRoot 'inventory.json'
if (Test-Path -LiteralPath $taskInventory) { throw 'Never reseal closed evidence' }
$taskFiles = @(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force -File | Sort-Object FullName | ForEach-Object {
    if ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Reparse point refused' }
    [ordered]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');bytes=$_.Length;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
})
[ordered]@{schema_version=1;kind='local-root-retirement-negative-inventory';source_commit='6098ab02fc5b7ed23cc5fa80e1ce221c8edbbbce';baseline_commit='f48fd4bf1de242d4130cd61459a3daed8437f33e';normalization='none-raw-bytes';qualification_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc';retirement_frame_gate_passed=$false;creator_sigkill_tested=$true;server=$false;product=$false;original_send=$false;original_armed=$false;end_to_end_original=$false;completion_allowed=$false;files=$taskFiles} | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $taskInventory -Encoding utf8NoBOM
& (Join-Path $PSScriptRoot 'verify-negative.ps1') -SelfTest
if (-not $?) { throw 'Negative inventory verification failed; never reseal' }
