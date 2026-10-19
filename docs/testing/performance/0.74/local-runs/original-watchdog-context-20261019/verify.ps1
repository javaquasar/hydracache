param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$utf8 = [Text.UTF8Encoding]::new($false,$true)
$inventory = Join-Path $PSScriptRoot 'manifest.json'
function CanonicalBytes([string]$Path) {
    $text = $utf8.GetString([IO.File]::ReadAllBytes($Path))
    if ($text.StartsWith([string][char]0xfeff)) { $text=$text.Substring(1) }
    return ,$utf8.GetBytes($text.Replace("`r`n","`n"))
}
function Verify($m) {
    if ($m.schema_version -ne 1 -or $m.kind -cne 'local-evidence-inventory' -or
        $m.source_commit -cne 'c9b0651bbd0042c5c67217731ad4f96de73e690f' -or
        $m.text_normalization -cne 'utf8-lf-no-bom' -or
        $m.qualification_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or
        $m.product -isnot [bool] -or $m.product -or
        $m.original_send -isnot [bool] -or $m.original_send) { throw 'Inventory scope drift' }
    $all = @(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force)
    if (@($all | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }).Count) { throw 'Reparse point in packet' }
    $actual = @($all | Where-Object { -not $_.PSIsContainer -and $_.FullName -cne $inventory } |
        ForEach-Object { [IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/') })
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in $m.files) {
        if ($file.path -notmatch '^[A-Za-z0-9][A-Za-z0-9._/-]*$' -or
            $file.path.Split('/') -contains '..' -or $file.path.Split('/') -contains '.' -or
            -not $seen.Add($file.path) -or $file.sha256 -cnotmatch '^[0-9a-f]{64}$' -or
            $file.bytes -lt 0) { throw 'Inventory shape drift' }
        $bytes = CanonicalBytes (Join-Path $PSScriptRoot $file.path)
        $hash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
        if ($bytes.Length -ne $file.bytes -or $hash -cne $file.sha256) { throw "Inventory mismatch: $($file.path)" }
    }
    if (@($m.files).Count -ne $actual.Count -or
        @(Compare-Object -CaseSensitive ($actual | Sort-Object) (@($m.files.path) | Sort-Object)).Count) { throw 'Inventory is not closed' }
    $expected = @('ok','retained-owner','retained-identity','retained-origin','retained-bounds','retained-deadline','retained-fence','retained-alarm','retained-alias','closing-context-fence','closing-context-deadline','closing-context-origin','names-after','inside-custody-missing')
    $sharedBundle = $null
    foreach ($cohort in @('gnu194-source','gnu188-source')) {
        $receipt = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/receipt.json") | ConvertFrom-Json
        $build = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/build.json") | ConvertFrom-Json
        $compiler = if ($cohort -ceq 'gnu194-source') { '1.94.0' } else { '1.88.0' }
        if ($receipt.source_commit -cne $m.source_commit -or $build.source_commit -cne $m.source_commit -or
            $receipt.compiler -cne $compiler -or $build.compiler -cne $compiler -or
            $receipt.nonroot_uid -notmatch '^[1-9][0-9]*$' -or $build.nonroot_uid -cne $receipt.nonroot_uid -or
            $receipt.frame_cases -ne 14 -or $receipt.clock_service_before -cne 'active' -or $receipt.clock_service_after -cne 'active' -or
            $receipt.image_before -cnotmatch '^[0-9a-f]{64}  /' -or $receipt.image_before -cne $receipt.image_after -or
            $build.image -cne $receipt.image_before -or $receipt.qualification_sha256 -cne $m.qualification_sha256 -or
            $receipt.test_channel_only -isnot [bool] -or -not $receipt.test_channel_only) { throw 'Cohort drift' }
        foreach ($flag in @('product','original_send','watchdog_spawn')) {
            if ($receipt.$flag -isnot [bool] -or $receipt.$flag) { throw 'Cohort authority drift' }
        }
        foreach ($gate in @(@('units',4),@('old-exchange',30),@('frame',1))) {
            $log = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/$($gate[0]).log")
            if ($log -notmatch "test result: ok\. $($gate[1]) passed; 0 failed") { throw "Gate missing: $($gate[0])" }
        }
        $frame = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/frame.log")
        $cases = @([regex]::Matches($frame,'HC074-ORIGINAL-WATCHDOG case=([a-z-]+) ') | ForEach-Object { $_.Groups[1].Value })
        if ($cases.Count -ne 14 -or @(Compare-Object -CaseSensitive ($cases | Sort-Object) ($expected | Sort-Object)).Count -or
            $frame -notmatch 'case=ok frame_auth=34 channel_checks=2' -or
            $frame -notmatch 'case=retained-owner frame_auth=0 channel_checks=0' -or
            $frame -notmatch 'case=inside-custody-missing frame_auth=0 channel_checks=1') { throw 'Frame matrix drift' }
        $guardians = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/guardian-matrix.log")
        if ($guardians -notmatch 'completed=true cases=17' -or
            @([regex]::Matches($guardians,'HC074-REDUNDANT case=')).Count -ne 17 -or
            $guardians -notmatch ('image_sha256=' + $receipt.image_before.Substring(0,64))) { throw 'Guardian regression drift' }
        $before = $utf8.GetString((CanonicalBytes (Join-Path $PSScriptRoot "$cohort/bundle-before.log")))
        $after = $utf8.GetString((CanonicalBytes (Join-Path $PSScriptRoot "$cohort/bundle-after.log")))
        if ($before -cne $after -or @($before.Trim().Split("`n")).Count -ne 9 -or
            ($null -ne $sharedBundle -and $sharedBundle -cne $before)) { throw 'Bundle input drift' }
        $sharedBundle = $before
    }
}
$parsed = Get-Content -Raw -LiteralPath $inventory | ConvertFrom-Json -AsHashtable
Verify $parsed
$negative = 0
if ($SelfTest) {
    foreach ($case in @('hash','bytes','escape','duplicate','missing','source','qualification','send')) {
        $bad = $parsed | ConvertTo-Json -Depth 8 | ConvertFrom-Json -AsHashtable
        switch ($case) {
            'hash' { $bad.files[0].sha256 = '0' * 64 }
            'bytes' { $bad.files[0].bytes++ }
            'escape' { $bad.files[0].path = '../outside' }
            'duplicate' { $bad.files += $bad.files[0] }
            'missing' { $bad.files = @($bad.files | Select-Object -Skip 1) }
            'source' { $bad.source_commit = '0' * 40 }
            'qualification' { $bad.qualification_sha256 = '0' * 64 }
            'send' { $bad.original_send = $true }
        }
        $refused = $false
        try { Verify $bad } catch { $refused = $true }
        if (-not $refused) { throw "Negative accepted: $case" }
        $negative++
    }
}
"inventory verified: $(@($parsed.files).Count) files; $negative in-memory negative controls; no runtime authority"
