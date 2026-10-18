$ErrorActionPreference='Stop'
$repo=(& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
$base='5a2cb676b58f7e17a2d2880ded2c56aae93b7a41'
foreach ($item in @(
    @('initial-test-source.patch','c5d6ad2d0e8539ea5140eaf1f88095321300468ab613d39382f0d0536ca2c732'),
    @('offset-baseline-source.patch','a1afd37602bd72c9a2810f9a274aeabf716ad659b9ec2ebbe1f09ec1e2e0e016')
)) {
    $patch=Join-Path $PSScriptRoot $item[0]
    if ((Get-FileHash -LiteralPath $patch).Hash.ToLowerInvariant() -cne $item[1]) { throw 'Negative capsule drift' }
    $raw=[IO.File]::ReadAllText($patch).Replace("`r`n","`n")
    $paths=@([regex]::Matches($raw,'(?m)^diff --git a/([^\r\n ]+) b/[^\r\n ]+$') | ForEach-Object { $_.Groups[1].Value })
    if ($paths.Count -ne 9 -or @($paths | Sort-Object -Unique).Count -ne 9 -or
        @($paths | Where-Object { $_ -notmatch '^(tools/long-run-supervisor-074/src/[^/]+\.rs|crates/xtask/tests/support/[^/]+\.rs|docs/testing/gated-test-registry\.toml)$' }).Count -ne 0) { throw 'Negative capsule scope differs' }
    $existing=@(& git -C $repo ls-tree -r --name-only $base -- @paths)
    if ($LASTEXITCODE -ne 0) { throw 'Baseline unavailable' }
    $parent=[IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
    $name='hc074-descriptor-negative-'+[Guid]::NewGuid().ToString('N')
    $scratch=Join-Path $parent $name
    $view=Join-Path $scratch 'source'
    New-Item -ItemType Directory -Path $scratch | Out-Null
    try {
        $archive=Join-Path $scratch 'baseline.zip'
        & git -C $repo archive --format=zip --output=$archive $base -- @existing
        if ($LASTEXITCODE -ne 0) { throw 'Baseline archive failed' }
        Expand-Archive -LiteralPath $archive -DestinationPath $view
        foreach ($path in $existing) {
            $target=Join-Path $view $path
            [IO.File]::WriteAllText($target,[IO.File]::ReadAllText($target).Replace("`r`n","`n"),[Text.UTF8Encoding]::new($false))
        }
        $normalized=Join-Path $scratch 'filtered.patch'
        [IO.File]::WriteAllText($normalized,$raw,[Text.UTF8Encoding]::new($false))
        & git -C $view -c core.autocrlf=false apply --check $normalized
        if ($LASTEXITCODE -ne 0) { throw 'Negative capsule cannot apply' }
        & git -C $view -c core.autocrlf=false apply $normalized
        if ($LASTEXITCODE -ne 0) { throw 'Negative reconstruction failed' }
        if ($item[0] -ceq 'initial-test-source.patch') {
            $rows=@(Get-Content (Join-Path $PSScriptRoot 'initial-test-source-hashes.log'))
            if ($rows.Count -ne 9) { throw 'Initial source inventory differs' }
            foreach ($line in $rows) {
                if ($line -cnotmatch '^([a-f0-9]{64})  ([A-Za-z0-9_./-]+)$') { throw 'Invalid initial source record' }
                $hash,$path=$Matches[1],$Matches[2]
                if ($path -cnotin $paths) { throw 'Hash outside initial capsule' }
                $text=[IO.File]::ReadAllText((Join-Path $view $path))
                $forms=@($text,$text.Replace("`n","`r`n")) | ForEach-Object { [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($_))).ToLowerInvariant() }
                if ($hash -cnotin $forms) { throw 'Initial build source differs' }
            }
        } else {
            $source=[IO.File]::ReadAllText((Join-Path $view 'tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan.rs'))
            if ($source.Contains('if position != 0') -or -not $source.Contains('fn original_descriptor_capture_refuses_nonzero_sink_position()')) { throw 'Offset baseline is not the uncorrected regression' }
        }
    } finally {
        $resolved=(Resolve-Path -LiteralPath $scratch).Path
        if ([IO.Path]::GetFullPath($resolved) -ne [IO.Path]::GetFullPath($scratch) -or
            [IO.Path]::GetDirectoryName($resolved).TrimEnd('\') -ne $parent -or [IO.Path]::GetFileName($resolved) -ne $name) { throw 'Temporary cleanup target refused' }
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
Write-Output 'HC074-ORIGINAL-DESCRIPTORS-NEGATIVES capsules=2 initial_sources=9 offset_baseline=true admitted=false'
