$ErrorActionPreference = 'Stop'
$taskInventory = Join-Path $PSScriptRoot 'inventory.json'
if (Test-Path -LiteralPath $taskInventory) { throw 'Closed evidence must never be resealed' }
$taskFiles = @(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force -File | Sort-Object FullName | ForEach-Object {
    if ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Reparse point refused' }
    [ordered]@{ path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/'); bytes=$_.Length;
        sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
})
[ordered]@{ schema_version=1; kind='local-root-guardian-inventory'; source_commit='72f538063aa03d460843ed813a2c249e36223b24';
    baseline_commit='037802aa0efc0e87ad847f1f285a0d043148535b'; normalization='none-raw-bytes';
    qualification_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc';
    server=$false; product=$false; original_send=$false; original_armed=$false; creator_sigkill_tested=$false;
    completion_allowed=$false; files=$taskFiles } | ConvertTo-Json -Depth 12 |
    Set-Content -LiteralPath $taskInventory -Encoding utf8NoBOM
& (Join-Path $PSScriptRoot 'verify.ps1') -SelfTest
if (-not $?) { throw 'Verification failed; preserve inventory, never reseal' }
