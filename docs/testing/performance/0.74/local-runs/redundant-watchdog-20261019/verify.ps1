param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$utf8 = [System.Text.UTF8Encoding]::new($false, $true)
$inventory = Join-Path $PSScriptRoot 'manifest.json'
function CanonicalBytes([string]$Path) {
    $text = $utf8.GetString([System.IO.File]::ReadAllBytes($Path))
    if ($text.StartsWith([string][char]0xfeff)) { $text = $text.Substring(1) }
    return ,$utf8.GetBytes($text.Replace("`r`n", "`n"))
}
function Verify($m) {
    if ($m.schema_version -ne 1 -or $m.kind -cne 'local-evidence-inventory' -or
        $m.source_commit -cne 'c3539d171118d796812a3d1dbad16cfdb4296f6e' -or
        $m.text_normalization -cne 'utf8-lf-no-bom' -or
        $m.qualification_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or
        $m.original_authority -isnot [bool] -or $m.original_authority -or
        $m.product -isnot [bool] -or $m.product) { throw 'Inventory scope drift' }
    $all = @(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force)
    if (@($all | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }).Count) { throw 'Reparse point in packet' }
    $actual = @($all | Where-Object { -not $_.PSIsContainer -and $_.FullName -cne $inventory } |
        ForEach-Object { [IO.Path]::GetRelativePath($PSScriptRoot, $_.FullName).Replace('\','/') })
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in $m.files) {
        if ($file.path -notmatch '^[A-Za-z0-9][A-Za-z0-9._/-]*$' -or
            $file.path.Split('/') -contains '..' -or $file.path.Split('/') -contains '.' -or
            -not $seen.Add($file.path) -or $file.sha256 -cnotmatch '^[0-9a-f]{64}$' -or
            $file.bytes -lt 0) { throw 'Inventory entry shape drift' }
        $bytes = CanonicalBytes (Join-Path $PSScriptRoot $file.path)
        $hash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
        if ($bytes.Length -ne $file.bytes -or $hash -cne $file.sha256) { throw "Inventory mismatch: $($file.path)" }
    }
    if (@($m.files).Count -ne $actual.Count -or
        @(Compare-Object -CaseSensitive ($actual | Sort-Object) (@($m.files.path) | Sort-Object)).Count) { throw 'Inventory is not closed' }
    $expected = @('healthy-expiry','owner-loss','first-dead-owner-loss','second-dead-owner-loss','first-stopped-owner-loss','second-stopped-owner-loss','first-dead-expiry','second-dead-expiry','first-stopped-expiry','second-stopped-expiry','lost-second-ack-first-dead','lost-second-ack-first-stopped','partial-enrollment-loss','duplicate-global','hidden-writer-first-dead','both-dead','both-stopped')
    foreach ($cohort in @('gnu194-source','gnu188-source')) {
        $receipt = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/receipt.json") | ConvertFrom-Json
        $compiler = if ($cohort -ceq 'gnu194-source') { '1.94.0' } else { '1.88.0' }
        if ($receipt.source_commit -cne $m.source_commit -or $receipt.pair_cases -ne 17 -or
            $receipt.compiler -cne $compiler -or $receipt.nonroot_uid -notmatch '^[1-9][0-9]*$' -or
            $receipt.image_before -cnotmatch '^[0-9a-f]{64}  /' -or
            $receipt.image_before -cne $receipt.image_after -or
            $receipt.qualification_sha256 -cne $m.qualification_sha256) { throw 'Cohort drift' }
        foreach ($flag in @('product','whole_cgroup','original_authority')) {
            if ($receipt.$flag -isnot [bool] -or $receipt.$flag) { throw 'Cohort authority drift' }
        }
        foreach ($check in @(@('units',18),@('old-exchange',30),@('old-barrier',1))) {
            $log = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/$($check[0]).log")
            if ($log -notmatch "test result: ok\. $($check[1]) passed; 0 failed") { throw "Missing gate: $cohort/$($check[0])" }
        }
        $matrix = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/pair-matrix.log")
        $cases = @([regex]::Matches($matrix,'HC074-REDUNDANT case=([a-z-]+) ') | ForEach-Object { $_.Groups[1].Value })
        $old = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/old-owner-matrix.log")
        if ($old -notmatch 'completed=true cases=12' -or @([regex]::Matches($old,'HC074-PREARMED case=')).Count -ne 12) { throw 'Prior owner matrix drift' }
        if ($cases.Count -ne 17 -or @(Compare-Object -CaseSensitive ($cases | Sort-Object) ($expected | Sort-Object)).Count -or
            $matrix -notmatch 'completed=true cases=17' -or
            $matrix -notmatch 'case=both-dead[^\r\n]*fence_retained=False expected_negative=True' -or
            $matrix -notmatch 'case=both-stopped[^\r\n]*fence_retained=True expected_negative=True' -or
            $matrix -notmatch ('image_sha256=' + $receipt.image_before.Substring(0,64))) { throw 'Kernel matrix drift' }
    }
}
$parsed = Get-Content -Raw -LiteralPath $inventory | ConvertFrom-Json -AsHashtable
Verify $parsed
$negative = 0
if ($SelfTest) {
    foreach ($case in @('hash','bytes','escape','duplicate','missing','source','qualification','authority')) {
        $bad = $parsed | ConvertTo-Json -Depth 8 | ConvertFrom-Json -AsHashtable
        switch ($case) {
            'hash' { $bad.files[0].sha256 = '0' * 64 }
            'bytes' { $bad.files[0].bytes++ }
            'escape' { $bad.files[0].path = '../outside' }
            'duplicate' { $bad.files += $bad.files[0] }
            'missing' { $bad.files = @($bad.files | Select-Object -Skip 1) }
            'source' { $bad.source_commit = '0' * 40 }
            'qualification' { $bad.qualification_sha256 = '0' * 64 }
            'authority' { $bad.original_authority = $true }
        }
        $refused = $false
        try { Verify $bad } catch { $refused = $true }
        if (-not $refused) { throw "Negative accepted: $case" }
        $negative++
    }
}
"inventory verified: $(@($parsed.files).Count) files; $negative in-memory negative controls; no runtime authority"
