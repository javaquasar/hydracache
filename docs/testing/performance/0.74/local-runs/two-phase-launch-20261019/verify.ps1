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
        $m.source_commit -cne 'fab1798c74f8a579184ec19893c2c76472421cbe' -or
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
    foreach ($cohort in @('gnu194-source','gnu188-source')) {
        $receipt = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/receipt.json") | ConvertFrom-Json
        if ($receipt.source_commit -cne $m.source_commit -or $receipt.kernel_cases -ne 12 -or
            $receipt.image_before -cne $receipt.image_after -or
            $receipt.qualification_sha256 -cne $m.qualification_sha256 -or
            $receipt.product -or $receipt.systemd -or $receipt.original_authority) { throw 'Cohort drift' }
        foreach ($check in @(@('units',9),@('old-exchange',30),@('hidden-writer',1),@('kernel-matrix',1))) {
            $log = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/$($check[0]).log")
            if ($log -notmatch "test result: ok\. $($check[1]) passed; 0 failed") { throw "Missing positive gate: $cohort/$($check[0])" }
        }
        $cases = @(Select-String -LiteralPath (Join-Path $PSScriptRoot "$cohort/kernel-matrix.log") -Pattern 'two_phase_barrier case=')
        if ($cases.Count -ne 12) { throw 'Kernel case count drift' }
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
