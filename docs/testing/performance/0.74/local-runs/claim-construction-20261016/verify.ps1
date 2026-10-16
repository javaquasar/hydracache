$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Check($ok,[string]$why) { if (-not $ok) { throw $why } }
function Log([string]$key) { Get-Content -LiteralPath (Join-Path $packet ($key + '.log')) -Raw }
function Has([string]$key,[string]$text) { Check ((Log $key).Contains($text)) ("Missing receipt: $key / $text") }
function Paths($actual,$expected,[string]$why) {
    Check ((@($actual | Sort-Object) -join "`n") -ceq (@($expected | Sort-Object) -join "`n")) $why
}
function Results([string]$key,[string]$expected) {
    $rows = @([regex]::Matches((Log $key),'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;') |
        ForEach-Object { $_.Groups[1].Value + '/' + $_.Groups[2].Value })
    Check (($rows -join ',') -ceq $expected) ("Counts differ: $key")
}
Check ($m.format_version -eq 1 -and $m.case_count -eq 23) 'Schema/case count differs'
Check ($m.source_commit -ceq '09d5b28d0eca68eeedc2c402ae0f73901f62509c' -and $m.source_tree -ceq 'af29b89c10f77d1c33c380c20122f83eb8320890') 'Source identity differs'
Check ($m.baseline_commit -ceq 'eaebeb8fe8473a439cda9adf96f0342f0d7f72ce') 'Baseline differs'
Check ($m.preregistration_commit -ceq '79b3de2aa957c81c4ff89908954b9cf661ed47a9') 'Preregistration differs'
Check ($m.scope.local_causal_defect_proved -is [bool] -and $m.scope.local_causal_defect_proved -eq $true) 'Scope differs: local_causal_defect_proved'
Check ($m.scope.campaign_construction_corrected -is [bool] -and $m.scope.campaign_construction_corrected -eq $true) 'Scope differs: campaign_construction_corrected'
Check ($m.scope.same_file_success_transfer -is [bool] -and $m.scope.same_file_success_transfer -eq $true) 'Scope differs: same_file_success_transfer'
Check ($m.scope.old_predicates_and_markers_preserved -is [bool] -and $m.scope.old_predicates_and_markers_preserved -eq $true) 'Scope differs: old_predicates_and_markers_preserved'
Check ($m.scope.short_owner_unchanged -is [bool] -and $m.scope.short_owner_unchanged -eq $true) 'Scope differs: short_owner_unchanged'
Check ($m.scope.unlock_is_best_effort -is [bool] -and $m.scope.unlock_is_best_effort -eq $true) 'Scope differs: unlock_is_best_effort'
Check ($m.scope.historical_occurrences_attributed -is [bool] -and $m.scope.historical_occurrences_attributed -eq $false) 'Scope differs: historical_occurrences_attributed'
Check ($m.scope.all_writer_cessation_proved -is [bool] -and $m.scope.all_writer_cessation_proved -eq $false) 'Scope differs: all_writer_cessation_proved'
Check ($m.scope.original_start_admitted -is [bool] -and $m.scope.original_start_admitted -eq $false) 'Scope differs: original_start_admitted'
Check ($m.scope.server_admitted -is [bool] -and $m.scope.server_admitted -eq $false) 'Scope differs: server_admitted'
Check ($m.scope.product_performance_measured -is [bool] -and $m.scope.product_performance_measured -eq $false) 'Scope differs: product_performance_measured'
Check ($m.scope.prior_timings_requalified -is [bool] -and $m.scope.prior_timings_requalified -eq $false) 'Scope differs: prior_timings_requalified'
Check ($m.scope.new_sigkill_matrix_run -is [bool] -and $m.scope.new_sigkill_matrix_run -eq $false) 'Scope differs: new_sigkill_matrix_run'
Check ($m.scope.full_release_verify_run -is [bool] -and $m.scope.full_release_verify_run -eq $false) 'Scope differs: full_release_verify_run'
Check ($m.scope.binary_or_clean_build_attested -is [bool] -and $m.scope.binary_or_clean_build_attested -eq $false) 'Scope differs: binary_or_clean_build_attested'
Check ($m.scope.old_tests_skipped_or_serialized -is [bool] -and $m.scope.old_tests_skipped_or_serialized -eq $false) 'Scope differs: old_tests_skipped_or_serialized'
Check ($m.scope.ipc_retried -is [bool] -and $m.scope.ipc_retried -eq $false) 'Scope differs: ipc_retried'
Check ($m.scope.authorization_window_changed -is [bool] -and $m.scope.authorization_window_changed -eq $false) 'Scope differs: authorization_window_changed'
Check ($m.scope.push_performed -is [bool] -and $m.scope.push_performed -eq $false) 'Scope differs: push_performed'
$sourcePaths = @(
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_claim_construction.rs',
    'crates/xtask/tests/support/performance_contract_074_short_fence.rs',
    'tools/long-run-supervisor-074/src/host_claim_construction_tests.rs',
    'tools/long-run-supervisor-074/src/host_execution.rs',
    'tools/long-run-supervisor-074/src/host_short_fence_tests.rs'
)
$frozenPaths = @(
    'Cargo.lock',
    'Cargo.toml',
    'crates/xtask/tests/support/performance_contract_074_worker_proof.rs',
    'docs/COMPAT.md',
    'docs/testing/gated-test-registry.toml',
    'docs/testing/performance/0.74/local-runs/short-fence-20261016/manifest.json',
    'docs/testing/performance/0.74/local-runs/worker-proof-reuse-v2-20261016/manifest.json',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'tools/long-run-supervisor-074/Cargo.toml',
    'tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs',
    'tools/long-run-supervisor-074/src/diagnostic_empty_reservation_fence.rs',
    'tools/long-run-supervisor-074/src/diagnostic_fixed_campaign_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_fixed_campaign_admission_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_ipc.rs',
    'tools/long-run-supervisor-074/src/diagnostic_lease.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_fence.rs',
    'tools/long-run-supervisor-074/src/diagnostic_output_preparation.rs',
    'tools/long-run-supervisor-074/src/diagnostic_validation_cost_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_proof_reuse_tests.rs',
    'tools/long-run-supervisor-074/src/host_execution_fixed_admission_tests.rs',
    'tools/long-run-supervisor-074/src/state_store.rs',
    'tools/long-run-supervisor-074/tests/diagnostic_ipc.rs',
    'tools/long-run-supervisor-074/tests/diagnostic_lease.rs',
    'tools/long-run-supervisor-074/tests/diagnostic_original_fence.rs',
    'tools/long-run-supervisor-074/tests/host_execution.rs'
)
$plan = 'docs/testing/performance/0.74/diagnostic-claim-construction-plan-20261016.md'
Paths @($m.selected_sources.path) $sourcePaths 'Selected inventory differs'
Paths @($m.frozen_inputs.path) $frozenPaths 'Frozen inventory differs'
Push-Location $repo
try {
    $tree = & git rev-parse ($m.source_commit + '^{tree}')
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Git tree differs'
    $parent = & git rev-parse ($m.source_commit + '^')
    Check ($LASTEXITCODE -eq 0 -and $parent -ceq $m.preregistration_commit) 'Source must follow preregistration'
    & git merge-base --is-ancestor $m.baseline_commit $m.preregistration_commit
    Check ($LASTEXITCODE -eq 0) 'Baseline ancestry differs'
    $delta = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Check ($LASTEXITCODE -eq 0) 'Cannot inspect delta'
    Paths $delta @($sourcePaths + $plan) 'Unapproved source path'
    Check ($m.plan.path -ceq $plan) 'Plan path differs'
    foreach ($f in @($m.selected_sources) + @($m.frozen_inputs) + @($m.plan)) {
        Check ($f.blob -cmatch '^[0-9a-f]{40}$' -and $f.sha256 -cmatch '^[0-9a-f]{64}$') 'Invalid digest'
        $blob = & git rev-parse ($m.source_commit + ':' + $f.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $f.blob) ('Source blob differs: ' + $f.path)
        $hash = (Get-FileHash -LiteralPath (Join-Path $repo $f.path) -Algorithm SHA256).Hash.ToLowerInvariant()
        Check ($hash -ceq $f.sha256) ('Working bytes differ: ' + $f.path)
        if ($frozenPaths -ccontains $f.path) {
            $old = & git rev-parse ($m.baseline_commit + ':' + $f.path)
            Check ($LASTEXITCODE -eq 0 -and $old -ceq $f.blob) ('Frozen input differs: ' + $f.path)
        }
    }
    $prereg = & git rev-parse ($m.preregistration_commit + ':' + $plan)
    Check ($LASTEXITCODE -eq 0 -and $prereg -ceq $m.plan.blob) 'Plan changed after preregistration'
    $tests = (Get-Content -LiteralPath (Join-Path $repo 'tools/long-run-supervisor-074/src/host_claim_construction_tests.rs') -Raw).Replace("`r",'').TrimEnd()
    $overlay = (Get-Content -LiteralPath (Join-Path $packet 'baseline-tests.rs.txt') -Raw).Replace("`r",'').TrimEnd()
    Check ($tests -ceq $overlay) 'Baseline formatted test overlay differs from final assertions'
} finally { Pop-Location }
$keys = @(
    'baseline-red',
    'development-green',
    'linux-1.94.0-check',
    'linux-1.94.0-clippy',
    'linux-1.88.0-check',
    'linux-1.88.0-clippy',
    'windows-1.94.0-check',
    'windows-1.94.0-clippy',
    'windows-1.88.0-check',
    'windows-1.88.0-clippy',
    'host-unit-1.94.0',
    'scan-unit-1.94.0',
    'preparation-unit-1.94.0',
    'worker-proof-1.94.0',
    'integration-1.94.0',
    'host-unit-1.88.0',
    'scan-unit-1.88.0',
    'preparation-unit-1.88.0',
    'worker-proof-1.88.0',
    'integration-1.88.0',
    'original-1.94.0',
    'original-1.88.0',
    'windows-integration-1.94.0',
    'windows-integration-1.88.0',
    'static',
    'fmt',
    'doc-check',
    'registry',
    'local-contract',
    'require-ship',
    'final-doc-check',
    'final-registry',
    'final-local-contract',
    'final-require-ship'
)
Paths @($m.captures.key) $keys 'Capture inventory differs'
foreach ($c in $m.captures) {
    $expected = if ($c.key -cin @('baseline-red','require-ship','final-require-ship')) { 1 } else { 0 }
    Check ($c.exit_code -eq $expected -and ($c.exit_code -is [int] -or $c.exit_code -is [long])) ('Exit differs: ' + $c.key)
    Check ($c.truncated -is [bool] -and $c.truncated -eq $false) 'Capture must be complete'
    Check ($c.log -ceq ($c.key + '.log')) 'Log mapping differs'
    Check ($c.command -match '--offline --locked|fmt --all -- --check|^\.\\target\\debug\\xtask.exe ') 'Unknown command'
    Check ($c.command -notmatch '--test-threads|--skip|--ignored') 'Old test selection changed'
}
$names = @($keys | ForEach-Object { $_ + '.log' }) + @('baseline-overlay.patch','baseline-tests.rs.txt','README.md','verify.ps1')
Paths @($m.artifacts.path) $names 'Artifact inventory differs'
Paths @((Get-ChildItem -LiteralPath $packet -File).Name) @($names + 'manifest.json') 'Unexpected packet file'
foreach ($a in $m.artifacts) {
    Check ($a.path -cmatch '^[A-Za-z0-9.-]+$' -and $a.path -ne 'manifest.json') 'Unsafe artifact path'
    $f = Get-Item -LiteralPath (Join-Path $packet $a.path)
    $hash = (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    Check ($a.sha256 -cmatch '^[0-9a-f]{64}$' -and $hash -ceq $a.sha256 -and $f.Length -eq $a.bytes) ('Artifact differs: ' + $a.path)
}
Has 'baseline-red' 'test result: FAILED. 1 passed; 5 failed;'
Has 'baseline-red' 'constructor returned but inherited alias retained flock'
Results 'development-green' '6/0'
Results 'static' '179/0'
$cases = @()
foreach ($create in @('true','false')) {
    $last = if ($create -ceq 'true') { 4 } else { 5 }
    foreach ($kind in 0..$last) { $cases += "claim_construction named_create=$create kind=$kind release=true preserved=true" }
}
foreach ($kind in 0..3) { $cases += "claim_construction active_error=$kind release=true preserved=true" }
foreach ($context in @('false','true')) { $cases += "claim_construction active_none_context=$context release=true preserved=true" }
foreach ($route in 0..2) {
    $cases += "claim_construction late_route=$route release=true marker_retained=true"
    $cases += "claim_construction success_route=$route held_until_claim_drop=true marker_retained=true"
}
Check ($cases.Count -eq 23) 'Verifier case inventory differs'
foreach ($version in @('1.94.0','1.88.0')) {
    Results ("host-unit-$version") '19/0'
    Results ("scan-unit-$version") '4/0'
    Results ("preparation-unit-$version") '14/0'
    Results ("worker-proof-$version") '5/0'
    Results ("integration-$version") '24/0,20/0,5/0,6/0'
    Results ("original-$version") '99/4'
    Results ("windows-integration-$version") '21/0,18/0,5/0'
    $observed = @((Log "host-unit-$version") -split "`r?`n" | Where-Object { $_.StartsWith('claim_construction ') })
    Paths $observed $cases ("Case receipts differ: $version")
    Has ("host-unit-$version") 'short_fence raw_close_busy=true cloexec=true explicit_child_unlock_releases=true'
    Has ("host-unit-$version") 'short_fence corruption_cases=7 backend_calls=0 state_and_ledger_preserved=true'
    foreach ($platform in @('linux','windows')) {
        foreach ($action in @('check','clippy')) { Has ("$platform-$version-$action") 'Finished' }
    }
}
foreach ($prefix in @('','final-')) {
    Has ("${prefix}doc-check") 'doc-check: OK'
    Has ("${prefix}registry") 'OK (176 discovered gates, all registered)'
    Has ("${prefix}local-contract") 'OK (local, non-promotable)'
    Has ("${prefix}require-ship") 'ship admission is closed'
}
Write-Output 'claim-construction evidence: OK (23 cases; local ownership correction only; original-start/writers/server closed)'
