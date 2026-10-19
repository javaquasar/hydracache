$ErrorActionPreference = 'Stop'
$taskInventory = Join-Path $PSScriptRoot 'inventory.json'
if (Test-Path -LiteralPath $taskInventory) { throw 'Closed packet; never reseal' }
foreach ($taskName in @('README.md','metadata.json','verify.ps1','gnu194-confirmed/receipt.json','gnu188-confirmed/receipt.json')) {
    if (-not (Test-Path -LiteralPath (Join-Path $PSScriptRoot $taskName))) { throw "Missing $taskName" }
}
& (Join-Path $PSScriptRoot 'verify.ps1') -PreSeal -SelfTest
$taskEntries = @(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | ForEach-Object {
    [ordered]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
} | Sort-Object { $_.path })
$taskEntries | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $taskInventory -Encoding utf8NoBOM
& (Join-Path $PSScriptRoot 'verify.ps1') -SelfTest
