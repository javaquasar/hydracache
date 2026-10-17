$ErrorActionPreference = 'Stop'
$repo = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
$base = '725f9677512a2249b1952962826d8032a764a4cf'
$patch = Join-Path $PSScriptRoot 'double-frame-source.patch'
if ((Get-FileHash -LiteralPath $patch).Hash -cne '30C4C108D3F698831DEA7E690198507118CD50EF930A0F2B85E1944DBD4C59B4') { throw 'Rejected capsule drift' }
$paths = @([regex]::Matches([IO.File]::ReadAllText($patch), '(?m)^diff --git a/([^\r\n ]+) b/[^\r\n ]+\r?$') | ForEach-Object { $_.Groups[1].Value })
if ($paths.Count -ne 14 -or @($paths | Sort-Object -Unique).Count -ne 14 -or
    @($paths | Where-Object { $_ -notmatch '^(tools/long-run-supervisor-074/src/[^/]+\.rs|crates/xtask/tests/support/[^/]+\.rs|docs/testing/gated-test-registry\.toml|docs/testing/performance/0\.74/diagnostic-original-custody-join-plan-20261018\.md)$' }).Count -ne 0) { throw 'Unexpected capsule scope' }
$existing = @(& git -C $repo ls-tree -r --name-only $base -- @paths)
if ($LASTEXITCODE -ne 0) { throw 'Baseline tree unavailable' }
$parent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
$name = 'hc074-custody-negative-' + [Guid]::NewGuid().ToString('N')
$scratch = Join-Path $parent $name
$view = Join-Path $scratch 'source'
New-Item -ItemType Directory -Path $scratch | Out-Null
try {
    $archive = Join-Path $scratch 'baseline.zip'
    & git -C $repo archive --format=zip --output=$archive $base -- @existing
    if ($LASTEXITCODE -ne 0) { throw 'Baseline archive failed' }
    Expand-Archive -LiteralPath $archive -DestinationPath $view
    foreach ($path in $existing) {
        $target = Join-Path $view $path
        [IO.File]::WriteAllText($target, [IO.File]::ReadAllText($target).Replace("`r`n", "`n"), [Text.UTF8Encoding]::new($false))
    }
    $normalized = Join-Path $scratch 'filtered.patch'
    [IO.File]::WriteAllText($normalized, [IO.File]::ReadAllText($patch).Replace("`r`n", "`n"), [Text.UTF8Encoding]::new($false))
    & git -C $view -c core.autocrlf=false apply --check $normalized
    if ($LASTEXITCODE -ne 0) { throw 'Rejected capsule cannot apply to baseline' }
    & git -C $view -c core.autocrlf=false apply $normalized
    if ($LASTEXITCODE -ne 0) { throw 'Rejected capsule reconstruction failed' }
    $count = 0
    foreach ($line in (Get-Content (Join-Path $PSScriptRoot 'double-frame-source-hashes.log'))) {
        if ($line -cnotmatch '^([a-f0-9]{64})  ([A-Za-z0-9_./-]+)$') { throw 'Invalid rejected source hash' }
        $hash, $path = $Matches[1], $Matches[2]
        if ($path -cnotin $paths) { throw 'Hash outside capsule' }
        $text = [IO.File]::ReadAllText((Join-Path $view $path)).Replace("`r`n", "`n")
        # Git capsules store filtered LF blobs. The saved build inputs may be
        # either checkout newline form; no other source substitution is allowed.
        $forms = @($text, $text.Replace("`n", "`r`n")) | ForEach-Object {
            [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($_))).ToLowerInvariant()
        }
        if ($hash -cnotin $forms) { throw "Rejected source differs: $path" }
        $count++
    }
    if ($count -ne 13) { throw 'Rejected source inventory differs' }
} finally {
    $resolved = (Resolve-Path -LiteralPath $scratch).Path
    if ([IO.Path]::GetFullPath($resolved) -ne [IO.Path]::GetFullPath($scratch) -or
        [IO.Path]::GetDirectoryName($resolved).TrimEnd('\') -ne $parent -or
        [IO.Path]::GetFileName($resolved) -ne $name) { throw 'Temporary cleanup target refused' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
Write-Output 'HC074-CUSTODY-JOIN-NEGATIVE capsule=true source_hashes=13 admitted=false'
