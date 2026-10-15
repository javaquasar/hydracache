$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$m = Get-Content -Raw -LiteralPath "$packet/manifest.json" | ConvertFrom-Json
Push-Location -LiteralPath (Resolve-Path -LiteralPath "$packet/../../../../../..").Path
try {
    foreach ($revision in @($m.baseline_commit,$m.source_commit,$m.source_tree,$m.preregistration_commit,$m.implementation_commit) + @($m.commit_chain.commit) + @($m.commit_chain.parent)) {
        if ($revision -cnotmatch '^[0-9a-f]{40}$') { throw 'invalid Git object identifier' }
    }
    $sourcePaths = @(
        'tools/long-run-supervisor-074/src/diagnostic_oidc_input.rs',
        'tools/long-run-supervisor-074/src/diagnostic_oidc_input_tests.rs',
        'tools/long-run-supervisor-074/src/diagnostic_controller_fence_preflight.rs',
        'tools/long-run-supervisor-074/src/diagnostic_controller_fence_preflight_tests.rs',
        'crates/xtask/tests/performance_contract_074.rs',
        'docs/testing/performance/0.74/diagnostic-controller-fence-preflight-contract.toml'
    )
    $frozenPaths = @('Cargo.lock','docs/testing/performance/0.74/qualification-manifest.toml','docs/testing/performance/0.74/local-runs/scm-rights-eof-isolation-20261015/manifest.json')
    if ($m.source_files.Count -ne 6 -or @(Compare-Object @($m.source_files.path) $sourcePaths).Count -ne 0 -or
        $m.frozen_files.Count -ne 3 -or @(Compare-Object @($m.frozen_files.path) $frozenPaths).Count -ne 0) { throw 'source/frozen path inventory' }
    $names = @($m.captures.file)
    $disk = @(Get-ChildItem -LiteralPath $packet -Filter '*.log' -File | Select-Object -ExpandProperty Name)
    if ($names.Count -ne 46 -or @($names | Sort-Object -Unique).Count -ne 46 -or
        @(Compare-Object $names $disk).Count -ne 0) { throw 'capture inventory' }
    foreach ($a in @($m.captures) + @($m.extra_artifacts)) {
        if ($a.file -cnotmatch '^[A-Za-z0-9_.-]+$' -or (Split-Path -Leaf $a.file) -cne $a.file) { throw 'artifact basename' }
        $f = Get-Item -LiteralPath "$packet/$($a.file)"
        if ($f.Length -ne $a.bytes -or (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $a.sha256) { throw "artifact changed: $($a.file)" }
    }
    if ((git rev-parse "$($m.source_commit)^{tree}") -cne $m.source_tree) { throw 'source tree' }
    foreach ($f in $m.source_files) {
        if ((git rev-parse "$($m.source_commit):$($f.path)") -cne $f.git_blob) { throw 'historical source blob' }
    }
    foreach ($f in $m.frozen_files) {
        if ((Get-FileHash -LiteralPath $f.path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $f.sha256 -or
            (git hash-object --path=$($f.path) $f.path) -cne $f.git_blob -or
            (git rev-parse "$($m.baseline_commit):$($f.path)") -cne $f.git_blob) { throw 'frozen input or preceding packet' }
    }
    foreach ($link in $m.commit_chain) {
        if ((git rev-parse "$($link.commit)^") -cne $link.parent) { throw 'commit chain' }
    }
    $changed = @(git diff --name-only $m.baseline_commit $m.source_commit -- crates tools Cargo.toml Cargo.lock .github)
    $expected = @($m.source_files.path | Where-Object { $_ -match '^(crates|tools)/' })
    if ($changed.Count -ne 5 -or @(Compare-Object $changed $expected).Count -ne 0) { throw 'source/product/dependency/workflow inventory' }
    foreach ($spec in $m.additive_registrations) {
        $old = (git show "$($m.baseline_commit):$($spec.path)") -join "`n"
        $new = (git show "$($m.source_commit):$($spec.path)") -join "`n"
        if ([regex]::Matches($new, [regex]::Escape($spec.addition)).Count -ne 1 -or $new.Replace($spec.addition, '') -cne $old) { throw 'old input implementation/tests changed' }
    }
    $path = 'crates/xtask/tests/performance_contract_074.rs'
    $old = (git show "$($m.baseline_commit):$path") -join "`n"
    $new = (git show "$($m.source_commit):$path") -join "`n"
    $entry = '#[test]' + "`n" + 'fn diagnostic_controller_fence_preflight_is_original_read_only_not_replay_or_start() {'
    $start = $new.IndexOf($entry)
    $end = $new.IndexOf('#[test]', $start + $entry.Length)
    if ($start -lt 0 -or $end -lt 0 -or $new.Remove($start, $end - $start) -cne $old) { throw 'old static contracts changed' }
    foreach ($c in $m.captures) {
        $raw = Get-Content -Raw -LiteralPath "$packet/$($c.file)"
        if ($c.outer_test_totals) {
            $totals = [regex]::Matches($raw, '(?m)^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;')
            $p = 0; $failed = 0; $ignored = 0
            foreach ($t in $totals) { $p += [int]$t.Groups[1].Value; $failed += [int]$t.Groups[2].Value; $ignored += [int]$t.Groups[3].Value }
            if ($totals.Count -eq 0 -or $p -ne $c.outer_test_totals.passed -or $failed -ne $c.outer_test_totals.failed -or $ignored -ne 0) { throw "test totals: $($c.file)" }
        }
        if ($c.file -ceq 'doctest-internal-panic.log') {
            if ($c.exit_code -ne 0 -or $c.semantic_gate_pass -ne $false -or $raw -notmatch 'compiler unexpectedly panicked') { throw 'false-green API proof promoted' }
        } elseif ($c.file -like '*doctest*') {
            if ($c.semantic_gate_pass -ne $true -or $raw -match 'compiler unexpectedly panicked|internal compiler error' -or
                [regex]::Matches($raw, 'error\[E\d+\]:').Count -ne 9 -or $raw -notmatch 'error\[E0616\]') { throw 'ordinary compile errors required' }
        }
        $negative = @('missing-api-red.log', 'before-box-linux-1.88.0-clippy.log', 'source-audit-first-failed.log', 'static-link-disk-full.log')
        if ($negative -contains $c.file) {
            if ($c.exit_code -ne 1 -or $c.gate_expected_exit -ne 0 -or $c.semantic_gate_pass -ne $false) { throw 'negative reclassified' }
        } elseif ($c.file -in @('test-first-source.log', 'require-ship.log')) {
            if ($c.exit_code -ne 1 -or $c.gate_expected_exit -ne 1 -or $c.semantic_gate_pass -ne $true) { throw 'diff/closed admission reclassified' }
        } elseif ($c.file -cne 'doctest-internal-panic.log') {
            if ($c.exit_code -ne 0 -or $c.semantic_gate_pass -ne $true) { throw 'unexpected capture outcome' }
        }
    }
    foreach ($row in @(@('baseline-oidc.log',17),@('baseline-fence.log',11),@('postcommit-focused.log',8),@('final-trust-current.log',57),@('final-oidc-msrv.log',25),@('postcommit-root.log',71),@('performance-contract-tests.log',166),@('gate-registry-tests.log',6))) {
        $c = @($m.captures | Where-Object file -CEQ $row[0])
        if ($c.Count -ne 1 -or $c[0].outer_test_totals.passed -ne $row[1]) { throw 'required result missing' }
    }
    $root = Get-Content -Raw -LiteralPath "$packet/postcommit-root.log"
    foreach ($row in @(@('^normal_root_handoff mode=',25),@('^normal_root_transfer kind=',14),@('^normal_root_send_window kind=',6))) {
        if ([regex]::Matches($root, '(?m)' + $row[0]).Count -ne $row[1]) { throw 'root protocol coverage' }
    }
    if ((Get-Content -Raw -LiteralPath "$packet/gate-registry.log") -notmatch '176 discovered gates, all registered') { throw 'registry count' }
    if ((Get-Content -Raw -LiteralPath "$packet/local-contract.log") -notmatch 'local, non-promotable' -or
        (Get-Content -Raw -LiteralPath "$packet/require-ship.log") -notmatch 'ship admission is closed') { throw 'admission boundary' }
    $cleanup = Get-Content -Raw -LiteralPath "$packet/build-cache-cleanup.log" | ConvertFrom-Json
    if ($cleanup.removed.Count -ne 8 -or $cleanup.recursive -ne $false -or $cleanup.freed_bytes -ne 1918267392) { throw 'cleanup record' }
    Write-Output 'Verified 46 captures, extra artifacts, historical source tree/six blobs/commit chain and frozen inputs.'
    Write-Output 'Verified two additive registrations and one additive static gate; old input verifiers, tests, contracts and product/dependency/workflow sources remain unchanged.'
    Write-Output 'Verified final 57/25 input regressions, 8 post-commit cases, 71 root tests with 25/14/6 modes, 166 contracts, 6 registry tests and 176 gates.'
    Write-Output 'The cargo-exit0 compiler panic remains invalid API evidence. Corrected doctests have nine ordinary errors per run, including E0616, and no compiler panic.'
    Write-Output 'Missing API, MSRV large-Err lint, initial audit helper and disk-full link failures stay retained. Rebuildable cache cleanup is recorded; ship remains closed.'
    Write-Output 'Unsigned artifact/historical-source consistency only, not execution attestation, current production enrollment, durable nonce/start authority or a performance claim.'
} finally {
    Pop-Location
}
