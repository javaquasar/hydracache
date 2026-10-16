$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Check($ok,[string]$why) { if (-not $ok) { throw $why } }
Check ($m.format_version -eq 1 -and $m.source_commit -ceq '8f87e6a1a50230c020a7f629004c590b2860411e' -and $m.baseline_commit -ceq '88a623f69f00345867cb0c2404b1b6a79a4af5f9') 'Source identity differs'
Check ($m.test_only_attribution -eq $true -and $m.host_busy_negative_retained -eq $true) 'Scope differs'
foreach ($f in @('production_optimization','authorization_window_changed','old_tests_skipped_or_serialized','historical_time_cause_proved','historical_kernel_invalid_cause_proved','shared_load_fixed','timing_is_cpu_measurement','starting_proved','runtime_handoff','server_admitted','qualification_or_performance_claim','binary_or_clean_build_attested','full_msrv_original_group_rerun','new_sigkill_matrix_run')) { Check ($m.$f -eq $false) ('Scope must stay closed: ' + $f) }
$sourcePaths = @(
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_causal.rs',
    'crates/xtask/tests/support/performance_contract_074_validation_cost.rs',
    'docs/testing/gated-test-registry.toml',
    'docs/testing/performance/0.74/diagnostic-validation-cost-design.md',
    'tools/long-run-supervisor-074/src/diagnostic_collector_progress_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_ipc.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_refusal_trace.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs',
    'tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs',
    'tools/long-run-supervisor-074/src/diagnostic_validation_trace_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_validation_cost_tests.rs'
)
$frozenPaths = @(
    'Cargo.lock',
    'Cargo.toml',
    'tools/long-run-supervisor-074/Cargo.toml',
    'docs/COMPAT.md',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'docs/testing/performance/0.74/local-runs/collector-causal-20261016/manifest.json',
    'docs/testing/performance/0.74/local-runs/collector-progress-20261016/manifest.json',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_progress.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_trust_input.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_build_input.rs',
    'tools/long-run-supervisor-074/src/diagnostic_oidc_input.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_progress.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_causal_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved_crash_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_attempt_crash_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs',
    'tools/long-run-supervisor-074/src/diagnostic_lease.rs',
    'tools/long-run-supervisor-074/src/server.rs'
)
Push-Location $repo
try {
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.source_commit)
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Source tree differs'
    & git merge-base --is-ancestor $m.baseline_commit $m.source_commit
    Check ($LASTEXITCODE -eq 0) 'Wrong ancestry'
    Check ($m.source_files.Count -eq 12 -and @($m.source_files.path | Sort-Object -Unique).Count -eq 12) 'Source inventory differs'
    foreach ($s in $m.source_files) {
        Check ($sourcePaths -ccontains $s.path) 'Unexpected source'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit,$s.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $s.git_blob) 'Source blob differs'
        & git diff --exit-code $m.source_commit -- $s.path
        Check ($LASTEXITCODE -eq 0) ('Working source differs: ' + $s.path)
    }
    $changed = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Check ($LASTEXITCODE -eq 0 -and $changed.Count -eq 13) 'Changed inventory differs'
    foreach ($p in $changed) { Check (($sourcePaths + @('docs/testing/performance/0.74/diagnostic-validation-cost-plan-20261016.md')) -ccontains $p) ('Unexpected change: ' + $p) }
    Check ($m.frozen_files.Count -eq 27 -and @($m.frozen_files.path | Sort-Object -Unique).Count -eq 27) 'Frozen inventory differs'
    foreach ($s in $m.frozen_files) {
        Check ($frozenPaths -ccontains $s.path) 'Unexpected unchanged input'
        foreach ($c in @($m.baseline_commit,$m.source_commit)) {
            $blob = & git rev-parse ('{0}:{1}' -f $c,$s.path)
            Check ($LASTEXITCODE -eq 0 -and $blob -ceq $s.git_blob) 'Unchanged blob differs'
        }
        Check ((Get-FileHash -LiteralPath $s.path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $s.sha256) ('Unchanged bytes differ: ' + $s.path)
    }
} finally { Pop-Location }
$names = @()
foreach ($a in $m.artifacts) {
    Check ($a.file -cmatch '^[A-Za-z0-9][A-Za-z0-9.-]*$' -and $names -cnotcontains $a.file) 'Unsafe/duplicate artifact'
    $names += $a.file
    $p = Join-Path $packet $a.file
    Check ((Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a.sha256 -and (Get-Item -LiteralPath $p).Length -eq $a.bytes) ('Artifact differs: ' + $a.file)
}
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object Name -CNE 'manifest.json' | ForEach-Object Name | Sort-Object)
Check (($actual -join "`n") -ceq (($names | Sort-Object) -join "`n")) 'Artifact inventory differs'
Check ($m.captures.Count -eq 43 -and @($m.captures.file | Sort-Object -Unique).Count -eq 43 -and $m.artifacts.Count -eq 45) 'Capture count differs'
foreach ($c in $m.captures) {
    Check ($names -ccontains $c.file -and $c.exit_code -eq $c.expected_exit_code -and -not [string]::IsNullOrWhiteSpace($c.cmd)) 'Capture metadata differs'
    $negative = @('compiler-red.log','semantics-1.94.0.log','require-ship.log') -ccontains $c.file
    Check ($c.exit_code -eq $(if ($negative) { 1 } else { 0 })) 'Unexpected command outcome'
}
function Log([string]$name) { Get-Content -LiteralPath (Join-Path $packet ($name + '.log')) -Raw }
Check ((Log 'baseline') -match '1 passed; 0 failed' -and (Log 'baseline') -match 'composition_case=3') 'Missing baseline'
Check ((Log 'compiler-red') -match 'E0432' -and (Log 'compiler-red') -match 'validation_trace') 'Missing compiler red'
Check ((Log 'unit-development') -match '5 passed; 0 failed') 'Missing development green'
foreach ($p in @('linux','windows')) { foreach ($v in @('1.94.0','1.88.0')) {
    Check ((Log ('final-' + $p + '-' + $v + '-unit')) -match '6 passed; 0 failed') 'Portable trace tests differ'
    foreach ($mode in @('check','clippy')) {
        $c = $m.captures | Where-Object file -CEQ ('final-' + $p + '-' + $v + '-' + $mode + '.log')
        Check ($c.exit_code -eq 0 -and $c.cmd -match '--all-targets --all-features --offline --locked') 'Build lane differs'
        if ($mode -ceq 'clippy') { Check ($c.cmd -match '-D warnings') 'Strict Clippy differs' }
    }
} }
Check ((Log 'original-1.94.0') -match '99 passed; 0 failed; 4 ignored' -and (Log 'original-1.94.0') -notmatch 'HC074-VALIDATION-COST|HC074-KERNEL-FIRST-REFUSAL') 'Ordinary regression differs'
$c = $m.captures | Where-Object file -CEQ 'original-1.94.0.log'
Check ($c.cmd -notmatch 'test-threads|--skip') 'Original regression parallelism changed'
Check ((Log 'semantics-1.94.0') -match '23 passed; 1 failed' -and (Log 'semantics-1.94.0') -match 'Host\(Busy\)') 'Missing IPC negative'
foreach ($name in @('semantics-repeat-1.94.0','semantics-1.88.0')) {
    foreach ($n in @(30,24,15)) { Check ((Log $name) -match ($n.ToString() + ' passed; 0 failed')) 'Semantic regression differs' }
    $c = $m.captures | Where-Object file -CEQ ($name + '.log')
    Check ($c.cmd -notmatch 'test-threads|--skip') 'Semantic parallelism changed'
}
foreach ($v in @('1.94.0','1.88.0')) {
    Check ((Log ('kernel-' + $v)) -match '29 passed; 0 failed; 1 ignored') 'Kernel regression differs'
    $d = Log ('docs-' + $v)
    Check ($d -match '28 passed; 0 failed' -and $d -notmatch 'E0432|E0433|E0603') 'Negative API boundary differs'
}
Check ($m.observations.Count -eq 3) 'Probe inventory differs'
$expected = @{Setup=@(11,99);Fence=@(8,72);Retain=@(96,908);Observe=@(32,308);Publish=@(272,2696);Bind=@(66,652)}
foreach ($o in $m.observations) {
    Check (@('probe-final-1.94.0-r1.log','probe-final-1.94.0-r2.log','probe-final-1.88.0-r1.log') -ccontains $o.capture) 'Unexpected observation'
    Check ($o.stages.Count -eq 6 -and @($o.stages.stage | Sort-Object -Unique).Count -eq 6) 'Stage inventory differs'
    $raw = Get-Content -LiteralPath (Join-Path $packet $o.capture) -Raw
    Check ($raw -match '1 passed; 0 failed') 'Probe failed'
    $c = $m.captures | Where-Object file -CEQ $o.capture
    Check ($c.cmd -match '--ignored --exact --nocapture$' -and $c.cmd -notmatch 'test-threads|--skip') 'Probe command differs'
    foreach ($s in $o.stages) {
        Check ($expected.ContainsKey($s.stage) -and $s.authentication_calls -eq $expected[$s.stage][0] -and $s.signature_calls -eq $s.authentication_calls -and $s.worker_policy_calls -eq $expected[$s.stage][1]) 'Stage counts differ'
        $line = 'HC074-VALIDATION-STAGE {0} Stats {{ wall_ns: {1}, authentication_calls: {2}, authentication_ns: {3}, signature_calls: {4}, signature_ns: {5}, worker_policy_calls: {6}, worker_policy_ns: {7} }}' -f $s.stage,$s.wall_ns,$s.authentication_calls,$s.authentication_ns,$s.signature_calls,$s.signature_ns,$s.worker_policy_calls,$s.worker_policy_ns
        Check ($raw.Contains($line)) 'Stage timing transcribed incorrectly'
        Check ($s.wall_ns -gt 0 -and $s.authentication_ns -ge $s.signature_ns -and $s.wall_ns -ge $s.worker_policy_ns) 'Invalid duration relationship'
    }
}
Check ((Log 'static') -match '176 passed; 0 failed' -and (Log 'static') -match 'validation_cost_equivalence') 'Static contracts differ'
Check ((Log 'doc-check') -match 'doc-check: OK' -and (Log 'registry') -match '176 discovered gates, all registered') 'Read-only gates differ'
Check ((Log 'performance-contract') -match 'OK \(local, non-promotable\)' -and (Log 'require-ship') -match 'ship admission is closed') 'Admission differs'
Write-Output 'validation-cost evidence OK; IPC negative retained, runtime unchanged, historical cause and server admission remain open'
