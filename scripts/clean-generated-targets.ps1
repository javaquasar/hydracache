[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [string] $TargetDir = (Join-Path $PSScriptRoot "..\target")
)

$ErrorActionPreference = "Stop"

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$expectedTarget = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target")).TrimEnd('\', '/')
$targetPath = [System.IO.Path]::GetFullPath($TargetDir).TrimEnd('\', '/')

if ($targetPath -ne $expectedTarget) {
    throw "Refusing to clean '$targetPath'. This script only cleans the repository target directory '$expectedTarget'."
}

if (-not (Test-Path -LiteralPath $targetPath)) {
    Write-Host "Target directory does not exist: $targetPath"
    return
}

$patterns = @(
    "consumer-check-*",
    "llvm-cov-target",
    "msrv-*",
    "release-*",
    "release-gate*",
    "semver-checks",
    "verify-sandbox-*"
)

$items = foreach ($pattern in $patterns) {
    Get-ChildItem -LiteralPath $targetPath -Force -Directory -Filter $pattern -ErrorAction SilentlyContinue
}

$uniqueItems = $items | Sort-Object FullName -Unique

if (-not $uniqueItems) {
    Write-Host "No generated target directories matched cleanup patterns."
    return
}

foreach ($item in $uniqueItems) {
    $resolvedItem = [System.IO.Path]::GetFullPath($item.FullName)
    if (-not $resolvedItem.StartsWith($targetPath, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove path outside target directory: $resolvedItem"
    }

    if ($PSCmdlet.ShouldProcess($resolvedItem, "Remove generated target directory")) {
        Remove-Item -LiteralPath $resolvedItem -Recurse -Force
        Write-Host "Removed $resolvedItem"
    }
}
