$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$m = Get-Content -Raw -LiteralPath "$packet/manifest.json" | ConvertFrom-Json
$repository = (Resolve-Path -LiteralPath "$packet/../../../../../..").Path
Push-Location -LiteralPath $repository
try {
    $names = @($m.captures.file)
    if ($names.Count -ne 49 -or @($names | Sort-Object -Unique).Count -ne 49) {
        throw 'capture inventory'
    }
    $disk = @(Get-ChildItem -LiteralPath $packet -File -Filter '*.log' | Select-Object -ExpandProperty Name)
    if (@(Compare-Object $names $disk).Count -ne 0) { throw 'unlisted raw log' }
    foreach ($a in @($m.captures) + @($m.extra_artifacts)) {
        if ((Split-Path -Leaf $a.file) -cne $a.file) { throw 'artifact basename' }
        $f = Get-Item -LiteralPath "$packet/$($a.file)"
        if ($f.Length -ne $a.bytes -or
            (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $a.sha256) {
            throw "artifact changed: $($a.file)"
        }
    }
    if ((git rev-parse "$($m.source_commit)^{tree}") -cne $m.source_tree) { throw 'source tree' }
    foreach ($f in $m.source_files) {
        if ((git rev-parse "$($m.source_commit):$($f.path)") -cne $f.git_blob -or
            (git hash-object --path=$($f.path) $f.path) -cne $f.git_blob) { throw 'source blob' }
    }
    foreach ($f in $m.frozen_files) {
        if ((Get-FileHash -LiteralPath $f.path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $f.sha256 -or
            (git hash-object --path=$($f.path) $f.path) -cne $f.git_blob -or
            (git rev-parse "$($m.baseline_commit):$($f.path)") -cne $f.git_blob) { throw 'frozen input' }
    }
    if ((git rev-parse "$($m.source_commit):docs/testing/gated-test-registry.toml") -cne $m.registry_blob) {
        throw 'registry blob'
    }
    foreach ($link in $m.commit_chain) {
        if ((git rev-parse "$($link.commit)^") -cne $link.parent) { throw 'commit chain' }
    }
    $changed = @(git diff --name-only $m.baseline_commit $m.source_commit -- tools crates Cargo.toml Cargo.lock .github)
    if ($changed.Count -ne 2 -or @(Compare-Object $changed @($m.source_files.path)).Count -ne 0) {
        throw 'source inventory or product/dependency/workflow drift'
    }
    $unchanged = @(
        'tools/long-run-supervisor-074/src/diagnostic_signer_owned_root.rs',
        'tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_exec.rs',
        'tools/long-run-supervisor-074/src/bin/diagnostic_signer_root_probe.rs',
        'tools/long-run-supervisor-074/tests/support/root_process_guard.rs',
        'tools/long-run-supervisor-074/tests/diagnostic_signer_root_guard.rs',
        'tools/long-run-supervisor-074/tests/diagnostic_signer_root_probe.rs',
        'crates/xtask/tests/performance_contract_074.rs',
        'docs/testing/gated-test-registry.toml'
    )
    foreach ($path in $unchanged) {
        if ((git rev-parse "$($m.baseline_commit):$path") -cne (git rev-parse "$($m.source_commit):$path") -or
            (git rev-parse "$($m.source_commit):$path") -cne (git hash-object --path=$path $path)) {
            throw "existing source/gate changed: $path"
        }
    }
    function Source-At($revision, $path) {
        $lines = @(git show "${revision}:$path")
        if ($LASTEXITCODE -ne 0) { throw 'source read' }
        return $lines -join "`n"
    }
    $loss = 'tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_loss.rs'
    $old = Source-At $m.baseline_commit $loss
    $new = Source-At $m.source_commit $loss
    $boundary = '#[cfg(test)]'
    if ($old.Substring(0, $old.IndexOf($boundary)) -cne $new.Substring(0, $new.IndexOf($boundary))) {
        throw 'normal-build loss source changed'
    }
    if ($new.Contains('Command::')) { throw 'loss source forbidden-token exception' }
    foreach ($name in @(
        'rejected_rights_are_closed_even_for_foreign_credentials_or_truncation',
        'truncated_ancillary_rights_close_every_delivered_or_truncated_writer'
    )) {
        $start = $old.IndexOf("    fn $name() {")
        $end = $old.IndexOf('    #[test]', $start)
        $body = $old.Substring($start, $end - $start)
        $body = $body.Substring($body.IndexOf('{') + 1)
        $body = $body.Substring(0, $body.LastIndexOf('}'))
        $startNew = $new.IndexOf("    fn $name() {")
        $endNew = $new.IndexOf('    #[test]', $startNew)
        $wrapped = $new.Substring($startNew, $endNew - $startNew)
        $wrapped = $wrapped.Substring($wrapped.IndexOf('|| {') + 4)
        $wrapped = $wrapped.Substring(0, $wrapped.LastIndexOf('},'))
        if (($body -replace '\s', '') -cne ($wrapped -replace '\s', '')) {
            throw "original EOF fixture changed: $name"
        }
    }
    $tests = 'tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_tests.rs'
    $before = Source-At $m.baseline_commit $tests
    $after = Source-At $m.source_commit $tests
    $header = 'pub(super) mod images;'
    if ($before.Substring(0, $before.IndexOf($header) + $header.Length) -cne
        $after.Substring(0, $after.IndexOf($header) + $header.Length) -or
        $before.Substring($before.IndexOf('#[test]')) -cne $after.Substring($after.IndexOf('#[test]'))) {
        throw 'existing root tests changed'
    }
    function Assert-Totals($file, $passed, $failed) {
        $raw = Get-Content -Raw -LiteralPath "$packet/$file"
        # Anchored outer libtest summaries only; nested red Output debug strings
        # contain escaped newlines and must not be counted as another execution.
        $totals = [regex]::Matches($raw, '(?m)^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;')
        $p = 0
        $f = 0
        foreach ($t in $totals) {
            $p += [int]$t.Groups[1].Value
            $f += [int]$t.Groups[2].Value
            if ([int]$t.Groups[3].Value -ne 0) { throw 'unreported ignored tests' }
        }
        if ($totals.Count -eq 0 -or $p -ne $passed -or $f -ne $failed) { throw "test totals: $file" }
    }
    Assert-Totals 'baseline.log' 68 0
    Assert-Totals 'inherited-writer-red.log' 0 1
    Assert-Totals 'received-rights-red.log' 0 1
    Assert-Totals 'development-loss-units.log' 13 0
    foreach ($file in @(
        'development-group-1.log', 'development-group-2.log',
        'final-current.log', 'final-msrv.log', 'postcommit-current.log',
        'corrected-current.log', 'corrected-msrv.log', 'corrected-postcommit-current.log'
    )) { Assert-Totals $file 71 0 }
    foreach ($i in 1..5) { Assert-Totals "postcommit-units-$i.log" 51 0 }
    Assert-Totals 'guard-current.log' 9 0
    Assert-Totals 'guard-msrv.log' 9 0
    Assert-Totals 'static-contract-before-placement.log' 164 1
    Assert-Totals 'static-contract-target.log' 1 0
    Assert-Totals 'performance-contract-tests.log' 165 0
    Assert-Totals 'gate-registry-tests.log' 6 0
    foreach ($file in @('corrected-current.log', 'corrected-postcommit-current.log')) {
        $raw = Get-Content -Raw -LiteralPath "$packet/$file"
        foreach ($row in @(@('^normal_root_handoff mode=', 25), @('^normal_root_transfer kind=', 14), @('^normal_root_send_window kind=', 6))) {
            if ([regex]::Matches($raw, '(?m)' + $row[0]).Count -ne $row[1]) { throw 'canonical mode coverage' }
        }
        foreach ($test in @(
            'inherited_cloexec_writer_blocks_eof_until_original_child_exit',
            'received_writer_control_prevents_false_eof',
            'rejected_rights_are_closed_even_for_foreign_credentials_or_truncation',
            'truncated_ancillary_rights_close_every_delivered_or_truncated_writer'
        )) {
            $marker = "isolated_rights_done test=root::launch::loss::tests::$test admission=false"
            if ([regex]::Matches($raw, [regex]::Escape($marker)).Count -ne 1) { throw 'isolated fixture completion' }
        }
    }
    $msrv = Get-Content -Raw -LiteralPath "$packet/corrected-msrv.log"
    foreach ($marker in @('extra_libm_root_handoff_modes=25 refused=true', 'extra_libm_root_transfer_modes=14 refused=true', 'extra_libm_root_send_window_modes=6 refused=true')) {
        if (-not $msrv.Contains($marker)) { throw 'MSRV refusal missing' }
    }
    if ($msrv.Contains('normal_root_handoff mode=good')) { throw 'MSRV refusal promoted' }
    $red = @('inherited-writer-red.log', 'received-rights-red.log', 'static-contract-before-placement.log')
    foreach ($c in $m.captures) {
        if ($red -contains $c.file) {
            if ($c.exit_code -ne 1 -or $c.gate_expected_exit -ne 0) { throw 'negative reclassified' }
        } elseif ($c.file -ceq 'require-ship.log') {
            if ($c.exit_code -ne 1 -or $c.gate_expected_exit -ne 1) { throw 'closed admission reclassified' }
        } elseif ($c.exit_code -ne 0 -or $c.gate_expected_exit -ne 0) { throw 'unexpected capture status' }
    }
    if ((Get-Content -Raw -LiteralPath "$packet/gate-registry.log") -notmatch '176 discovered gates, all registered') { throw 'registry count' }
    if ((Get-Content -Raw -LiteralPath "$packet/local-contract.log") -notmatch 'local, non-promotable') { throw 'local admission' }
    if ((Get-Content -Raw -LiteralPath "$packet/require-ship.log") -notmatch 'ship admission is closed') { throw 'ship admission' }
    Write-Output 'Verified 49 captures, extra artifact digests, source/tree/two blobs/commit chain and three frozen inputs.'
    Write-Output 'Verified unchanged non-test loss source, original EOF bodies, previous root tests, process guard, static launch contract and 176-entry registry.'
    Write-Output 'Verified final 71-test groups, canonical 25/14/6 modes, four isolated completions and explicit MSRV extra-libm refusal.'
    Write-Output 'Two deliberately red EOF controls and one static placement failure remain exit1/expected-gate0. Require-ship remains an expected refusal.'
    Write-Output 'Unsigned consistency only. The historical writer owner, production all-writer cessation and server/release admission are not established.'
} finally {
    Pop-Location
}
