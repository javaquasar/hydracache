$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Check($ok, [string]$why) { if (-not $ok) { throw $why } }
Check ($m.format_version -eq 1 -and $m.source_commit -ceq '2799318acf18aa5fc11f5895cbe3d0c558e4488c' -and $m.baseline_commit -ceq '7dff8a6bb04f3786016e699020c87d5ee8889c4e') 'Source identity differs'
Check ($m.post_ack_causal_preflight -eq $true -and $m.ordinary_attempt_before_tiering_failed -eq $true -and $m.new_live_proof_tier -ceq 'manual' -and $m.final_ordinary_regression_failed -eq $true) 'Scope/tiering differs'
foreach ($f in @('starting_proved','runtime_authority_handoff','admission_renewal','continuous_controller_presence','full_record_review_proved','runtime_validation_cost_fixed','shared_load_robustness_proved','production_enrollment','server_admitted','qualification_or_performance_claim','binary_or_clean_build_attested','full_msrv_original_group_rerun','new_sigkill_matrix_run','historical_kernel_invalid_cause_proved')) { Check ($m.$f -eq $false) ('Scope must remain closed: ' + $f) }
$sourcePaths = @(
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_causal.rs',
    'docs/COMPAT.md',
    'docs/testing/performance/0.74/diagnostic-collector-causal-contract.toml',
    'docs/testing/performance/0.74/diagnostic-collector-causal-design.md',
    'tools/long-run-supervisor-074/src/diagnostic_collector_progress.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_progress_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_causal_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_causal_codec_tests.rs',
    'docs/testing/gated-test-registry.toml',
    'crates/xtask/tests/support/performance_contract_074_collector_reserved.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_intent.rs'
)
$frozenPaths = @(
    'Cargo.lock',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'docs/testing/performance/0.74/local-runs/collector-progress-20261016/manifest.json',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_progress.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs',
    'tools/long-run-supervisor-074/src/diagnostic_lease.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_fence_preflight_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved_crash_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_attempt_crash_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_starting_intent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector.rs',
    'tools/long-run-supervisor-074/src/diagnostic_intent_projection.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_intent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs',
    'tools/long-run-supervisor-074/src/server.rs'
)
Check ($m.source_files.Count -eq 13 -and @($m.source_files.path | Sort-Object -Unique).Count -eq 13) 'Source inventory differs'
Check ($m.frozen_files.Count -eq 21 -and @($m.frozen_files.path | Sort-Object -Unique).Count -eq 21) 'Unchanged inventory differs'
Push-Location $repo
try {
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.source_commit)
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Source tree differs'
    & git merge-base --is-ancestor $m.baseline_commit $m.source_commit
    Check ($LASTEXITCODE -eq 0) 'Wrong ancestry'
    foreach ($s in $m.source_files) {
        Check ($sourcePaths -ccontains $s.path) 'Unexpected source'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit,$s.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $s.git_blob) 'Source blob differs'
        & git diff --exit-code $m.source_commit -- $s.path
        Check ($LASTEXITCODE -eq 0) ('Working source differs: ' + $s.path)
    }
    $allowed = $sourcePaths + @('docs/testing/performance/0.74/diagnostic-collector-causal-plan-20261016.md','docs/testing/performance/0.74/diagnostic-collector-causal-tiering-20261016.md')
    $changed = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Check ($LASTEXITCODE -eq 0 -and $changed.Count -eq 15) 'Changed inventory differs'
    foreach ($p in $changed) { Check ($allowed -ccontains $p) ('Unexpected change: ' + $p) }
    foreach ($s in $m.frozen_files) {
        Check ($frozenPaths -ccontains $s.path) 'Unexpected unchanged input'
        foreach ($commit in @($m.baseline_commit,$m.source_commit)) {
            $blob = & git rev-parse ('{0}:{1}' -f $commit,$s.path)
            Check ($LASTEXITCODE -eq 0 -and $blob -ceq $s.git_blob) 'Unchanged blob differs'
        }
        Check ((Get-FileHash -LiteralPath $s.path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $s.sha256) ('Unchanged bytes differ: ' + $s.path)
    }
    $current = (Get-Content -LiteralPath 'tools/long-run-supervisor-074/src/diagnostic_collector_causal_tests.rs' -Raw).Replace("`r",'')
    $ignore = '#[ignore = "manual nonroot fixed-window causal proof; no server"]' + "`n"
    Check (($current.Split(@($ignore),[System.StringSplitOptions]::None)).Count -eq 2) 'Manual annotation differs'
    $before = (Get-Content -LiteralPath (Join-Path $packet 'ordinary-composition-before-tiering.rs') -Raw).Replace("`r",'')
    Check ($current.Replace($ignore,'') -ceq $before) 'Tiering changed the live proof body'
    Check ((Get-FileHash -LiteralPath (Join-Path $packet 'ordinary-composition-before-tiering.rs') -Algorithm SHA256).Hash.ToLowerInvariant() -ceq 'd739cb212a90d6ac4febe7b4585b287abc2d0afb9e3d3d98b2f1c71fe3d7038c') 'Before-tiering source differs'
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
Check ($m.captures.Count -eq 55 -and @($m.captures.file | Sort-Object -Unique).Count -eq 55 -and $m.artifacts.Count -eq 58) 'Capture count differs'
foreach ($c in $m.captures) { Check ($names -ccontains $c.file -and $c.exit_code -eq $c.expected_exit_code -and -not [string]::IsNullOrWhiteSpace($c.cmd)) 'Capture outcome differs' }
function Log([string]$name) { Get-Content -LiteralPath (Join-Path $packet ($name + '.log')) -Raw }
Check ((Log 'baseline') -match '6 passed; 0 failed') 'Missing baseline'
Check ((Log 'red') -match 'E0599' -and (Log 'red') -match 'issue_progress_challenge') 'Missing API compiler red'
Check ((Log 'first-focused') -match '5 passed; 0 failed' -and (Log 'first-focused') -match 'live_case=2') 'Missing three-case development proof'
Check ((Log 'before-tiering-original') -match '96 passed; 4 failed; 2 ignored' -and (Log 'before-tiering-original') -match 'Authentication\(Time\)') 'Missing untiered negative'
Check ((Log 'final-original-1.94.0') -match '98 passed; 1 failed; 3 ignored' -and (Log 'final-original-1.94.0') -match 'Authentication\(Time\)' -and (Log 'final-original-1.94.0') -match 'collector_progress_retains_both_owners_without_mutation_and_cross_refuses') 'Missing tiered ordinary negative'
$o = $m.captures | Where-Object file -CEQ 'final-original-1.94.0.log'
Check ($o.exit_code -eq 1 -and $o.expected_exit_code -eq 1 -and $o.cmd -notmatch '--skip|test-threads') 'Ordinary regression must keep default parallelism'
Check ((Log 'final-original-1.94.0') -notmatch 'HC074-KERNEL-FIRST-REFUSAL') 'Unexpected original-input refusal trace'
Check ((Log 'final-focused-1.88.0') -match '4 passed; 0 failed; 1 ignored') 'MSRV ordinary causal tests differ'
foreach ($v in @('1.94.0','1.88.0')) {
    Check ((Log ('final-focused-' + $v)) -match '4 passed; 0 failed; 1 ignored') 'Ordinary causal group differs'
    Check ((Log ('final-kernel-' + $v)) -match '29 passed; 0 failed; 1 ignored') 'Kernel regression differs'
    Check ((Log ('final-bytes-' + $v)) -match '30 passed; 0 failed') 'Projection regression differs'
    $d = Log ('final-docs-' + $v)
    Check ($d -match '28 passed; 0 failed' -and $d -notmatch 'E0432|E0433|E0603') 'Compile-fail API boundary differs'
    $p = Log ('final-manual-' + $v)
    Check ($p -match '1 passed; 0 failed' -and $p -match 'live_case=0' -and $p -match 'live_case=1' -and $p -match 'live_case=2' -and $p -match 'live_case=3') 'Manual four-case proof differs'
    $c = $m.captures | Where-Object file -CEQ ('final-manual-' + $v + '.log')
    Check ($c.cmd -match '--ignored --exact --nocapture$' -and $c.cmd -notmatch 'test-threads|--skip') 'Manual proof command differs'
}
foreach ($platform in @('linux','windows')) { foreach ($v in @('1.94.0','1.88.0')) { foreach ($mode in @('check','clippy')) {
    $c = $m.captures | Where-Object file -CEQ ('final-' + $platform + '-' + $v + '-' + $mode + '.log')
    Check ($c.exit_code -eq 0 -and $c.cmd -match '--all-targets --all-features --offline --locked') 'Final build lane differs'
    if ($mode -ceq 'clippy') { Check ($c.cmd -match '-D warnings') 'Strict Clippy differs' }
} } }
Check ((Log 'final-static') -match '175 passed; 0 failed' -and (Log 'final-static') -match 'collector_causal_contract') 'Final static contract differs'
Check ((Log 'final-registry') -match '176 discovered gates, all registered') 'Registry differs'
Check ((Log 'final-doc-check') -match 'doc-check: OK') 'Documentation differs'
Check ((Log 'final-performance-contract') -match 'OK \(local, non-promotable\)') 'Local contract differs'
Check ((Log 'final-require-ship') -match 'ship admission is closed') 'Ship admission must remain closed'
Write-Output 'collector-causal evidence OK; both parallel negatives retained, manual proof explicit, Starting/runtime/server closed'
