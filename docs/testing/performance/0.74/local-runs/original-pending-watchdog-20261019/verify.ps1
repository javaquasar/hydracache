param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$utf8 = [Text.UTF8Encoding]::new($false,$true)
$inventory = Join-Path $PSScriptRoot 'manifest.json'
function CanonicalRawBytes([byte[]]$Raw) {
    $value = $utf8.GetString($Raw)
    if ($value.StartsWith([string][char]0xfeff,[StringComparison]::Ordinal)) { $value = $value.Substring(1) }
    return ,$utf8.GetBytes($value.Replace("`r`n","`n"))
}
function CanonicalBytes([string]$Path) {
    return ,(CanonicalRawBytes ([IO.File]::ReadAllBytes($Path)))
}
function Verify($m) {
    if ($m.schema_version -ne 1 -or $m.kind -cne 'local-evidence-inventory' -or
        $m.source_commit -cne '4b946bbeee1341088c2a1af9504ae3ae4f57da60' -or
        $m.text_normalization -cne 'utf8-lf-no-bom' -or
        $m.qualification_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or
        $m.pending_spawn -isnot [bool] -or -not $m.pending_spawn) { throw 'Inventory scope drift' }
    foreach ($flag in @('product','original_send','target_enrollment')) {
        if ($m.$flag -isnot [bool] -or $m.$flag) { throw 'Inventory authority drift' }
    }
    $beforeRecipe = $utf8.GetString((CanonicalBytes (Join-Path $PSScriptRoot 'run-before-settle.ps1')))
    $afterRecipe = $utf8.GetString((CanonicalBytes (Join-Path $PSScriptRoot 'run.ps1')))
    $strictPattern = '(?s)        # Workstation observation BEFORE fixture/admission creation; no clock writes\..*?(?=        \$frame = @\()'
    $oldStrict = [regex]::Matches($beforeRecipe,$strictPattern)
    $newStrict = [regex]::Matches($afterRecipe,$strictPattern)
    if ($oldStrict.Count -ne 1 -or $newStrict.Count -ne 1 -or $oldStrict[0].Value -cne $newStrict[0].Value) { throw 'Strict clock preflight was changed' }
    $all = @(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -Force)
    if (@($all | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }).Count) { throw 'Reparse point in packet' }
    $actual = @($all | Where-Object { -not $_.PSIsContainer -and $_.FullName -cne $inventory } |
        ForEach-Object { [IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/') })
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in $m.files) {
        if ($file.path -notmatch '^[A-Za-z0-9][A-Za-z0-9._/-]*$' -or
            $file.path.Split('/') -contains '..' -or $file.path.Split('/') -contains '.' -or
            -not $seen.Add($file.path) -or $file.sha256 -cnotmatch '^[0-9a-f]{64}$' -or $file.bytes -lt 0) { throw 'Inventory shape drift' }
        $bytes = CanonicalBytes (Join-Path $PSScriptRoot $file.path)
        $hash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
        if ($bytes.Length -ne $file.bytes -or $hash -cne $file.sha256) { throw "Inventory mismatch: $($file.path)" }
    }
    if (@($m.files).Count -ne $actual.Count -or
        @(Compare-Object -CaseSensitive ($actual | Sort-Object) (@($m.files.path) | Sort-Object)).Count) { throw 'Inventory is not closed' }
    $pendingCases = @('ok','clone-first','clone-second','restore-first','restore-second','guardian-first-dead','guardian-second-dead','retained-origin','closing-origin','inside-custody-missing')
    $contextCases = @('ok','retained-owner','retained-identity','retained-origin','retained-bounds','retained-deadline','retained-fence','retained-alarm','retained-alias','closing-context-fence','closing-context-deadline','closing-context-origin','names-after','inside-custody-missing')
    $sharedBundle = $null
    foreach ($cohort in @('gnu194-settled','gnu188-settled')) {
        $receipt = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/receipt.json") | ConvertFrom-Json
        $build = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/build.json") | ConvertFrom-Json
        $compiler = if ($cohort -ceq 'gnu194-settled') { '1.94.0' } else { '1.88.0' }
        if ($receipt.source_commit -cne $m.source_commit -or $build.source_commit -cne $m.source_commit -or
            $receipt.compiler -cne $compiler -or $build.compiler -cne $compiler -or
            $receipt.nonroot_uid -notmatch '^[1-9][0-9]*$' -or $build.nonroot_uid -cne $receipt.nonroot_uid -or
            $receipt.pending_cases -ne 10 -or $receipt.context_cases -ne 14 -or
            $receipt.clock_service_before -cne 'active' -or $receipt.clock_service_after -cne 'active' -or
            $receipt.image_before -cnotmatch '^[0-9a-f]{64}  /' -or $receipt.image_before -cne $receipt.image_after -or
            $build.image -cne $receipt.image_before -or $receipt.qualification_sha256 -cne $m.qualification_sha256 -or
            $receipt.runner_sha256 -cne $build.runner_sha256 -or
            $receipt.runner_sha256 -cne @($m.files | Where-Object { $_.path -ceq 'run.ps1' })[0].sha256) { throw 'Cohort drift' }
        foreach ($flag in @('test_channel_only','pending_spawn')) {
            if ($receipt.$flag -isnot [bool] -or -not $receipt.$flag) { throw 'Cohort positive scope drift' }
        }
        foreach ($flag in @('product','original_send','target_enrollment')) {
            if ($receipt.$flag -isnot [bool] -or $receipt.$flag) { throw 'Cohort authority drift' }
        }
        foreach ($gate in @(@('kernel-units',4),@('constructor',1),@('old-context-units',4),@('old-exchange',30),@('pending-frame',1),@('context-frame',1))) {
            $log = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/$($gate[0]).log")
            if ($log -notmatch "test result: ok\. $($gate[1]) passed; 0 failed") { throw "Gate missing: $($gate[0])" }
        }
        foreach ($gate in @('check','clippy')) {
            $log = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/$gate.log")
            if ($log -notmatch 'Finished .dev. profile' -or $log -match '(?m)^error') { throw "Build/lint missing: $gate" }
        }
        $pending = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/pending-frame.log")
        $cases = @([regex]::Matches($pending,'HC074-ORIGINAL-PENDING case=([a-z-]+) ') | ForEach-Object { $_.Groups[1].Value })
        if ($cases.Count -ne 10 -or @(Compare-Object -CaseSensitive ($cases | Sort-Object) ($pendingCases | Sort-Object)).Count) { throw 'Pending cases differ' }
        foreach ($case in $pendingCases) {
            $auth = if ($case -ceq 'ok') { 68 } elseif ($case -ceq 'retained-origin') { 0 } else { 34 }
            $checks = if ($case -ceq 'ok') { 4 } elseif ($case -ceq 'retained-origin') { 0 } elseif ($case -ceq 'inside-custody-missing') { 3 } else { 2 }
            if ($pending -notmatch "case=$case frame_auth=$auth channel_checks=$checks original_bounds=true test_channel_only=true enroll=false send=false") { throw 'Pending counts drift' }
        }
        $context = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/context-frame.log")
        $cases = @([regex]::Matches($context,'HC074-ORIGINAL-WATCHDOG case=([a-z-]+) ') | ForEach-Object { $_.Groups[1].Value })
        if ($cases.Count -ne 14 -or @(Compare-Object -CaseSensitive ($cases | Sort-Object) ($contextCases | Sort-Object)).Count -or
            $context -notmatch 'case=ok frame_auth=34 channel_checks=2') { throw 'Original context regression drift' }
        $guardians = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/guardian-matrix.log")
        if ($guardians -notmatch 'completed=true cases=17' -or @([regex]::Matches($guardians,'HC074-REDUNDANT case=')).Count -ne 17 -or
            $guardians -notmatch ('image_sha256=' + $receipt.image_before.Substring(0,64))) { throw 'Guardian regression drift' }
        $probe = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/clock-probe.log")
        if ($probe -notmatch 'clock_probe_complete=true backward_samples=0 samples=[1-9][0-9]*') { throw 'Workstation clock probe missing' }
        $settle = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$cohort/clock-settle.log")
        if ($settle -notmatch 'clock_settle_complete=true stable_seconds=65 noticed_backward_steps=[0-9]+ admission_created=false') { throw 'Environment stabilization missing' }
        $before = $utf8.GetString((CanonicalBytes (Join-Path $PSScriptRoot "$cohort/bundle-before.log")))
        $after = $utf8.GetString((CanonicalBytes (Join-Path $PSScriptRoot "$cohort/bundle-after.log")))
        if ($before -cne $after -or @($before.Trim().Split("`n")).Count -ne 9 -or
            ($null -ne $sharedBundle -and $sharedBundle -cne $before)) { throw 'Bundle input drift' }
        $sharedBundle = $before
    }
    foreach ($gate in @(@('source-selected',274),@('windows',49),@('doctests',17))) {
        $log = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$($gate[0]).log")
        if ($log -notmatch "test result: ok\. $($gate[1]) passed; 0 failed") { throw "Local gate missing: $($gate[0])" }
    }
    foreach ($refused in @(@('gnu194-source','5969402419'),@('gnu194-warm','5851547969'))) {
        $failedProbe = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$($refused[0])/clock-probe.log")
        if ($failedProbe -notmatch "RuntimeError: CLOCK_REALTIME moved backwards by $($refused[1])ns" -or
            (Test-Path (Join-Path $PSScriptRoot "$($refused[0])/receipt.json")) -or
            (Test-Path (Join-Path $PSScriptRoot "$($refused[0])/pending-frame.log")) -or
            (Test-Path (Join-Path $PSScriptRoot "$($refused[0])/context-frame.log"))) { throw 'Refused preflight was overwritten or promoted' }
    }
}
$parsed = Get-Content -Raw -LiteralPath $inventory | ConvertFrom-Json -AsHashtable
Verify $parsed
$negative = 0
if ($SelfTest) {
    $ascii = $utf8.GetBytes('param')
    $asciiHash = [Convert]::ToHexString((CanonicalRawBytes $ascii))
    if ($asciiHash -cne [Convert]::ToHexString($ascii)) { throw 'ASCII prefix lost during canonicalization' }
    $bom = [byte[]](@(0xef,0xbb,0xbf) + @($ascii))
    if ([Convert]::ToHexString((CanonicalRawBytes $bom)) -cne $asciiHash) { throw 'Actual BOM not removed exactly' }
    $zeroWidth = $utf8.GetBytes(([string][char]0x200b) + 'param')
    if ([Convert]::ToHexString((CanonicalRawBytes $zeroWidth)) -cne [Convert]::ToHexString($zeroWidth)) { throw 'Non-BOM Unicode prefix lost' }
    if ([Convert]::ToHexString((CanonicalRawBytes ($utf8.GetBytes("p`r`nq")))) -cne [Convert]::ToHexString($utf8.GetBytes("p`nq"))) { throw 'CRLF normalization differs' }
    $changed = $utf8.GetBytes('qaram')
    if ([Convert]::ToHexString((CanonicalRawBytes $changed)) -ceq $asciiHash) { throw 'First-byte mutation was hidden' }
    $invalidRefused = $false
    try { [void](CanonicalRawBytes ([byte[]]@(0xff))) } catch { $invalidRefused = $true }
    if (-not $invalidRefused) { throw 'Invalid UTF-8 accepted' }
    foreach ($case in @('hash','bytes','escape','duplicate','missing','source','qualification','enroll')) {
        $bad = $parsed | ConvertTo-Json -Depth 8 | ConvertFrom-Json -AsHashtable
        switch ($case) {
            'hash' { $bad.files[0].sha256 = '0' * 64 }
            'bytes' { $bad.files[0].bytes++ }
            'escape' { $bad.files[0].path = '../outside' }
            'duplicate' { $bad.files += $bad.files[0] }
            'missing' { $bad.files = @($bad.files | Select-Object -Skip 1) }
            'source' { $bad.source_commit = '0' * 40 }
            'qualification' { $bad.qualification_sha256 = '0' * 64 }
            'enroll' { $bad.target_enrollment = $true }
        }
        $refused = $false
        try { Verify $bad } catch { $refused = $true }
        if (-not $refused) { throw "Negative accepted: $case" }
        $negative++
    }
}
"inventory verified: $(@($parsed.files).Count) files; $negative in-memory negative controls; canonical prefix controls=$([int]$SelfTest.IsPresent * 6); no runtime authority"
