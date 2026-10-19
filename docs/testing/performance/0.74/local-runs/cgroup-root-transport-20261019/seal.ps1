$ErrorActionPreference = 'Stop'
$taskInventory = Join-Path $PSScriptRoot 'inventory.json'
if (Test-Path -LiteralPath $taskInventory) { throw 'Closed evidence must never be resealed' }
$taskFiles = @(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force -File | Sort-Object FullName | ForEach-Object {
    if ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Reparse point refused' }
    [ordered]@{ path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/'); bytes=$_.Length;
        sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
})
[ordered]@{ schema_version=1; kind='local-cgroup-root-transport-inventory';
    source_commit='65b02f3bac81ef0a29046584d44fc25db95d91b7';
    baseline_commit='f183134cb1e37b7a94c8f60a22f6f868bb98945a'; normalization='none-raw-bytes';
    qualification_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc';
    server=$false; product=$false; original_send=$false; armed_capability=$false; ack_emitted=$false;
    completion_allowed=$false; files=$taskFiles } | ConvertTo-Json -Depth 12 |
    Set-Content -LiteralPath $taskInventory -Encoding utf8NoBOM
& (Join-Path $PSScriptRoot 'verify.ps1') -SelfTest
if (-not $?) { throw 'Verification failed; preserve the failed inventory, do not reseal' }
