param([switch]$Verify)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$utf8 = [Text.UTF8Encoding]::new($false,$true)
$baseline = 'f94b4b56b5d0050a5b100989dc84c99a8e0ca4bf'
$directories = @('two-phase-launch-20261019','prearmed-watchdog-20261019','redundant-watchdog-20261019','original-watchdog-context-20261019')
$packets = @()
Push-Location $root
try {
    foreach ($name in $directories) {
        $relative = "docs/testing/performance/0.74/local-runs/$name"
        & git diff --quiet $baseline -- $relative
        if ($LASTEXITCODE -ne 0) { throw "Historical Git content drift: $name" }
        $tracked = @(& git ls-tree -r --name-only $baseline -- $relative)
        if ($LASTEXITCODE -ne 0 -or -not $tracked.Count) { throw "Historical packet missing: $name" }
        $all = @(Get-ChildItem -LiteralPath (Join-Path $root $relative) -Force -Recurse)
        if (@($all | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }).Count) { throw 'Historical reparse point' }
        $files = @($all | Where-Object { -not $_.PSIsContainer } | Sort-Object FullName)
        $actual = @($files | ForEach-Object { [IO.Path]::GetRelativePath($root,$_.FullName).Replace('\','/') })
        if (@(Compare-Object -CaseSensitive ($actual | Sort-Object) ($tracked | Sort-Object)).Count) { throw "Historical file set drift: $name" }
        $bindings = @($files | ForEach-Object {
            $value = $utf8.GetString([IO.File]::ReadAllBytes($_.FullName))
            if ($value.StartsWith([string][char]0xfeff,[StringComparison]::Ordinal)) { $value = $value.Substring(1) }
            $bytes = $utf8.GetBytes($value.Replace("`r`n","`n"))
            [ordered]@{ path=[IO.Path]::GetRelativePath($root,$_.FullName).Replace('\','/'); bytes=$bytes.Length; sha256=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant() }
        })
        $packets += [ordered]@{ name=$name; original_files_unchanged=$true; files=$bindings }
    }
    $result = [ordered]@{ schema_version=1; kind='historical-prefix-audit-local'; git_baseline=$baseline; normalization='utf8-lf-no-bom-ordinal'; legacy_verifiers_safe_for_new_inventories=$false; runtime_authority=$false; product=$false; packets=$packets }
    $json = $result | ConvertTo-Json -Depth 8
    if ($Verify) {
        $expected = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'historical-prefix-audit.json') | ConvertFrom-Json -AsHashtable
        if (($expected | ConvertTo-Json -Depth 8) -cne $json) { throw 'Corrected historical binding drift' }
        "historical prefix audit verified: $($packets.Count) packets; $(@($packets | ForEach-Object { $_.files }).Count) files; unchanged Git baseline; no runtime authority"
    } else { $json }
}
finally { Pop-Location }
