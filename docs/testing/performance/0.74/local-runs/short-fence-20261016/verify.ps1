$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Check($ok, [string]$why) { if (-not $ok) { throw $why } }
function Log([string]$key) { Get-Content -LiteralPath (Join-Path $packet ($key + '.log')) -Raw }
function Contains([string]$key, [string]$needle) {
    Check ((Log $key).Contains($needle)) ("Missing receipt: $key / $needle")
}
function Results([string]$key, [string]$expected) {
    $found = @([regex]::Matches((Log $key), 'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;') |
        ForEach-Object { $_.Groups[1].Value + '/' + $_.Groups[2].Value })
    Check (($found -join ',') -ceq $expected) ("Result counts differ: $key")
}
function SamePaths($actual, $expected, [string]$why) {
    Check ((@($actual | Sort-Object) -join "`n") -ceq (@($expected | Sort-Object) -join "`n")) $why
}
Check ($m.format_version -eq 1) 'Format differs'
Check ($m.source_commit -ceq '5d536ed073e1f8dbd8d9298352d33ec618ea0914') 'Source differs'
Check ($m.source_tree -ceq '4297e1ef47ad1dbae3cca6694e099ebfbde056e5') 'Source tree differs'
Check ($m.baseline_commit -ceq '166c3d1a6c0fef770e5ee17174b79a0a14181a78') 'Baseline differs'
Check ($m.preregistration_commit -ceq 'e02d90370054dc01f281f47abe65f9cbed894d31') 'Preregistration differs'
Check ($m.clarification_commit -ceq 'a6b14ce03d38347faef9c6537eb9d2f2e948fcb1') 'Clarification differs'
Check ($m.scope.local_causal_defect_proved -is [bool] -and $m.scope.local_causal_defect_proved -eq $true) 'Scope differs: local_causal_defect_proved'
Check ($m.scope.short_fence_corrected -is [bool] -and $m.scope.short_fence_corrected -eq $true) 'Scope differs: short_fence_corrected'
Check ($m.scope.owned_fork_fixture -is [bool] -and $m.scope.owned_fork_fixture -eq $true) 'Scope differs: owned_fork_fixture'
Check ($m.scope.original_admission_preserved -is [bool] -and $m.scope.original_admission_preserved -eq $true) 'Scope differs: original_admission_preserved'
Check ($m.scope.historical_occurrences_attributed -is [bool] -and $m.scope.historical_occurrences_attributed -eq $false) 'Scope differs: historical_occurrences_attributed'
Check ($m.scope.campaign_construction_errors_fixed -is [bool] -and $m.scope.campaign_construction_errors_fixed -eq $false) 'Scope differs: campaign_construction_errors_fixed'
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
    'crates/xtask/tests/support/performance_contract_074_short_fence.rs',
    'crates/xtask/tests/support/performance_contract_074_worker_proof.rs',
    'tools/long-run-supervisor-074/src/diagnostic_fixed_campaign_admission_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_output_preparation.rs',
    'tools/long-run-supervisor-074/src/host_execution.rs',
    'tools/long-run-supervisor-074/src/host_execution_fixed_admission_tests.rs',
    'tools/long-run-supervisor-074/src/host_short_fence_tests.rs'
)
$frozenPaths = @(
    'Cargo.toml',
    'Cargo.lock',
    'tools/long-run-supervisor-074/Cargo.toml',
    'docs/COMPAT.md',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'docs/testing/gated-test-registry.toml',
    'tools/long-run-supervisor-074/src/diagnostic_ipc.rs',
    'tools/long-run-supervisor-074/src/diagnostic_lease.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_proof_reuse_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_validation_cost_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_fence.rs',
    'tools/long-run-supervisor-074/src/diagnostic_empty_reservation_fence.rs',
    'tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs',
    'tools/long-run-supervisor-074/src/diagnostic_fixed_campaign_admission.rs',
    'tools/long-run-supervisor-074/src/state_store.rs',
    'tools/long-run-supervisor-074/tests/diagnostic_ipc.rs',
    'tools/long-run-supervisor-074/tests/diagnostic_lease.rs',
    'tools/long-run-supervisor-074/tests/host_execution.rs',
    'tools/long-run-supervisor-074/tests/diagnostic_original_fence.rs',
    'docs/testing/performance/0.74/local-runs/worker-proof-reuse-v2-20261016/manifest.json'
)
$plan = 'docs/testing/performance/0.74/diagnostic-short-fence-plan-20261016.md'
SamePaths @($m.selected_sources.path) $sourcePaths 'Selected source inventory differs'
SamePaths @($m.frozen_inputs.path) $frozenPaths 'Frozen inventory differs'
Push-Location $repo
try {
    $tree = & git rev-parse ($m.source_commit + '^{tree}')
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Git tree differs'
    foreach ($pair in @(@($m.baseline_commit,$m.preregistration_commit),
        @($m.preregistration_commit,$m.clarification_commit),@($m.clarification_commit,$m.source_commit))) {
        & git merge-base --is-ancestor $pair[0] $pair[1]
        Check ($LASTEXITCODE -eq 0) 'Preregistration ancestry differs'
    }
    $changed = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Check ($LASTEXITCODE -eq 0) 'Cannot inspect source delta'
    SamePaths $changed @($sourcePaths + $plan) 'Unapproved candidate path'
    foreach ($f in @($m.selected_sources) + @($m.frozen_inputs) + @($m.plan)) {
        Check ($f.blob -cmatch '^[0-9a-f]{40}$' -and $f.sha256 -cmatch '^[0-9a-f]{64}$') 'Invalid source digest'
        $blob = & git rev-parse ($m.source_commit + ':' + $f.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $f.blob) ('Source blob differs: ' + $f.path)
        $actual = (Get-FileHash -LiteralPath (Join-Path $repo $f.path) -Algorithm SHA256).Hash.ToLowerInvariant()
        Check ($actual -ceq $f.sha256) ('Working bytes differ: ' + $f.path)
        if ($frozenPaths -ccontains $f.path) {
            $old = & git rev-parse ($m.baseline_commit + ':' + $f.path)
            Check ($LASTEXITCODE -eq 0 -and $old -ceq $f.blob) ('Frozen input changed: ' + $f.path)
        }
    }
    Check ($m.plan.path -ceq $plan) 'Plan path differs'
    $clarified = & git rev-parse ($m.clarification_commit + ':' + $plan)
    Check ($LASTEXITCODE -eq 0 -and $clarified -ceq $m.plan.blob) 'Plan changed after clarification'
} finally { Pop-Location }
$expectedKeys = @(
    'red',
    'type-development-red',
    'green',
    'static-development',
    'static-truncated',
    'linux-1.94.0-check',
    'linux-1.94.0-clippy',
    'linux-1.88.0-check',
    'linux-1.88.0-clippy',
    'windows-1.94.0-check',
    'windows-1.94.0-clippy',
    'windows-1.88.0-check',
    'windows-1.88.0-clippy',
    'static',
    'fmt',
    'doc-check',
    'registry',
    'local-contract',
    'require-ship',
    'host-unit-1.94.0',
    'scan-unit-1.94.0',
    'preparation-unit-1.94.0',
    'worker-proof-1.94.0',
    'integration-1.94.0',
    'original-1.94.0',
    'host-unit-1.88.0',
    'scan-unit-1.88.0',
    'preparation-unit-1.88.0',
    'worker-proof-1.88.0',
    'integration-1.88.0',
    'original-1.88.0',
    'windows-integration-1.94.0',
    'windows-integration-1.88.0',
    'final-doc-check',
    'final-registry',
    'final-local-contract',
    'final-require-ship'
)
SamePaths @($m.captures.key) $expectedKeys 'Command inventory differs'
$negative = @('red','type-development-red','static-development','require-ship','final-require-ship')
foreach ($c in $m.captures) {
    $exit = if ($negative -ccontains $c.key) { 1 } else { 0 }
    Check ($c.exit_code -is [long] -or $c.exit_code -is [int]) 'Invalid exit code type'
    Check ($c.exit_code -eq $exit) ('Unexpected exit: ' + $c.key)
    $truncated = $c.key -cin @('static-development','static-truncated')
    Check ($c.truncated -is [bool] -and $c.truncated -eq $truncated) ('Truncation differs: ' + $c.key)
    Check ($c.command -match '--offline --locked|fmt --all -- --check|^\.\\target\\debug\\xtask.exe ') 'Unrecognized command'
    Check ($c.command -notmatch '--test-threads|--skip|--ignored') 'Old test selection changed'
    Check ($c.log -ceq ($c.key + '.log')) 'Log mapping differs'
}
$artifactNames = @($expectedKeys | ForEach-Object { $_ + '.log' }) + @('README.md','verify.ps1')
SamePaths @($m.artifacts.path) $artifactNames 'Artifact inventory differs'
SamePaths @((Get-ChildItem -LiteralPath $packet -File).Name) @($artifactNames + 'manifest.json') 'Unexpected packet file'
foreach ($f in $m.artifacts) {
    Check ($f.path -cmatch '^[A-Za-z0-9.-]+$' -and $f.path -ne 'manifest.json') 'Unsafe artifact path'
    $file = Get-Item -LiteralPath (Join-Path $packet $f.path)
    $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    Check ($f.sha256 -cmatch '^[0-9a-f]{64}$' -and $hash -ceq $f.sha256 -and $file.Length -eq $f.bytes) ('Artifact differs: ' + $f.path)
}
Contains 'red' 'test result: FAILED. 1 passed; 3 failed;'
Contains 'red' 'corrupt-ledger handler returned but inherited alias retained lock: Host(Busy)'
Contains 'red' 'short_fence raw_close_busy=true cloexec=true explicit_child_unlock_releases=true'
Contains 'type-development-red' 'error[E0308]'
Contains 'type-development-red' 'error[E0277]'
Contains 'static-development' 'test result: FAILED. 176 passed; 2 failed;'
Results 'static-truncated' '178/0'
Results 'static' '178/0'
Results 'green' '4/0'
foreach ($version in @('1.94.0','1.88.0')) {
    Results ("host-unit-$version") '13/0'
    Results ("scan-unit-$version") '4/0'
    Results ("preparation-unit-$version") '14/0'
    Results ("worker-proof-$version") '5/0'
    Results ("integration-$version") '24/0,20/0,5/0,6/0'
    Results ("original-$version") '99/4'
    Results ("windows-integration-$version") '21/0,18/0,5/0'
    Contains ("host-unit-$version") 'short_fence admission_error_release=true'
    Contains ("host-unit-$version") 'short_fence raw_close_busy=true cloexec=true explicit_child_unlock_releases=true'
    Contains ("host-unit-$version") 'short_fence drop_release=true failed_contender_preserves=true late_old_unlock_preserves_next=true'
    Contains ("host-unit-$version") 'short_fence corruption_cases=7 backend_calls=0 state_and_ledger_preserved=true'
    foreach ($kind in 0..6) {
        Contains ("host-unit-$version") ("short_fence corruption_kind=$kind transaction_refused=true inherited_child_ready=true")
    }
    foreach ($platform in @('linux','windows')) {
        foreach ($action in @('check','clippy')) { Contains ("$platform-$version-$action") 'Finished' }
    }
}
foreach ($prefix in @('','final-')) {
    Contains ("${prefix}doc-check") 'doc-check: OK'
    Contains ("${prefix}registry") 'gated-test-check: OK (176 discovered gates, all registered)'
    Contains ("${prefix}local-contract") 'OK (local, non-promotable)'
    Contains ("${prefix}require-ship") 'ship admission is closed'
}
Write-Output 'short-fence evidence: OK (local defect class corrected; historical owners unproved; campaign errors/start/server closed)'
