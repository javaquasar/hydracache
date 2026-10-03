[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$sums = Get-Content -LiteralPath (Join-Path $root "SHA256SUMS")

foreach ($line in $sums) {
    if ([string]::IsNullOrWhiteSpace($line)) {
        continue
    }
    $parts = $line -split "  ", 2
    if ($parts.Count -ne 2) {
        throw "Invalid SHA256SUMS line: $line"
    }
    $path = Join-Path $root $parts[1]
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
    if ($actual -ne $parts[0]) {
        throw "SHA-256 mismatch for $($parts[1]): expected $($parts[0]), got $actual"
    }
}

Write-Output "HydraCache 0.73 interrupted confirmation archive: OK"
