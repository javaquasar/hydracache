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
Check ($m.format_version -eq 1 -and $m.body_field_count -eq 18 -and $m.signed_leaf_count -eq 36 -and $m.grammar_negative_count -eq 30) 'Schema/inventory differs'
Check ($m.source_commit -ceq '3cda2ca518b028a96fceab934b06cc8cd6f40fc6') 'Identity differs: source_commit'
Check ($m.source_tree -ceq 'ca96224f95cc26a309140864235e3f62ec33a2f1') 'Identity differs: source_tree'
Check ($m.baseline_commit -ceq 'fdf0474f29bbada3e623822caf45ed8581ce0894') 'Identity differs: baseline_commit'
Check ($m.preregistration_commit -ceq '9c223c7312085f1f25db4325a6d1995398d21f12') 'Identity differs: preregistration_commit'
Check ($m.clarification_commit -ceq '314005062f26b4d0c10a8e755c40d77c53034887') 'Identity differs: clarification_commit'
Check ($m.scope.portable_signed_data_only -is [bool] -and $m.scope.portable_signed_data_only -eq $true) 'Scope differs: portable_signed_data_only'
Check ($m.scope.explicit_start_domain -is [bool] -and $m.scope.explicit_start_domain -eq $true) 'Scope differs: explicit_start_domain'
Check ($m.scope.old_runtime_and_assertions_preserved -is [bool] -and $m.scope.old_runtime_and_assertions_preserved -eq $true) 'Scope differs: old_runtime_and_assertions_preserved'
Check ($m.scope.ordinary_data_not_runtime_capability -is [bool] -and $m.scope.ordinary_data_not_runtime_capability -eq $true) 'Scope differs: ordinary_data_not_runtime_capability'
Check ($m.scope.kernel_observations_joined -is [bool] -and $m.scope.kernel_observations_joined -eq $false) 'Scope differs: kernel_observations_joined'
Check ($m.scope.actual_freshness_or_replay_checked -is [bool] -and $m.scope.actual_freshness_or_replay_checked -eq $false) 'Scope differs: actual_freshness_or_replay_checked'
Check ($m.scope.trusted_key_enrolled -is [bool] -and $m.scope.trusted_key_enrolled -eq $false) 'Scope differs: trusted_key_enrolled'
Check ($m.scope.durable_starting_published -is [bool] -and $m.scope.durable_starting_published -eq $false) 'Scope differs: durable_starting_published'
Check ($m.scope.authenticated_original_start_proved -is [bool] -and $m.scope.authenticated_original_start_proved -eq $false) 'Scope differs: authenticated_original_start_proved'
Check ($m.scope.all_writer_cessation_proved -is [bool] -and $m.scope.all_writer_cessation_proved -eq $false) 'Scope differs: all_writer_cessation_proved'
Check ($m.scope.server_admitted -is [bool] -and $m.scope.server_admitted -eq $false) 'Scope differs: server_admitted'
Check ($m.scope.product_performance_measured -is [bool] -and $m.scope.product_performance_measured -eq $false) 'Scope differs: product_performance_measured'
Check ($m.scope.full_release_verify_run -is [bool] -and $m.scope.full_release_verify_run -eq $false) 'Scope differs: full_release_verify_run'
Check ($m.scope.original_input_group_rerun -is [bool] -and $m.scope.original_input_group_rerun -eq $false) 'Scope differs: original_input_group_rerun'
Check ($m.scope.new_sigkill_matrix_run -is [bool] -and $m.scope.new_sigkill_matrix_run -eq $false) 'Scope differs: new_sigkill_matrix_run'
Check ($m.scope.binary_or_clean_build_attested -is [bool] -and $m.scope.binary_or_clean_build_attested -eq $false) 'Scope differs: binary_or_clean_build_attested'
Check ($m.scope.ipc_retried -is [bool] -and $m.scope.ipc_retried -eq $false) 'Scope differs: ipc_retried'
Check ($m.scope.old_tests_skipped_or_serialized -is [bool] -and $m.scope.old_tests_skipped_or_serialized -eq $false) 'Scope differs: old_tests_skipped_or_serialized'
Check ($m.scope.authorization_window_changed -is [bool] -and $m.scope.authorization_window_changed -eq $false) 'Scope differs: authorization_window_changed'
Check ($m.scope.push_performed -is [bool] -and $m.scope.push_performed -eq $false) 'Scope differs: push_performed'
$sourcePaths = @(
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_claim_construction.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_causal.rs',
    'crates/xtask/tests/support/performance_contract_074_short_fence.rs',
    'crates/xtask/tests/support/performance_contract_074_start_consent.rs',
    'crates/xtask/tests/support/performance_contract_074_validation_cost.rs',
    'crates/xtask/tests/support/performance_contract_074_worker_proof.rs',
    'docs/COMPAT.md',
    'tools/long-run-supervisor-074/src/diagnostic_start_consent.rs',
    'tools/long-run-supervisor-074/src/lib.rs',
    'tools/long-run-supervisor-074/tests/diagnostic_start_consent.rs'
)
$frozenPaths = @(
    'Cargo.lock',
    'Cargo.toml',
    'docs/testing/gated-test-registry.toml',
    'docs/testing/performance/0.74/local-runs/short-fence-20261016/manifest.json',
    'docs/testing/performance/0.74/local-runs/worker-proof-reuse-v2-20261016/manifest.json',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'tools/long-run-supervisor-074/Cargo.toml',
    'tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_causal_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_intent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs',
    'tools/long-run-supervisor-074/src/diagnostic_empty_reservation_fence.rs',
    'tools/long-run-supervisor-074/src/diagnostic_fixed_campaign_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_fixed_campaign_admission_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_ipc.rs',
    'tools/long-run-supervisor-074/src/diagnostic_lease.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_fence.rs',
    'tools/long-run-supervisor-074/src/diagnostic_output_preparation.rs',
    'tools/long-run-supervisor-074/src/diagnostic_reviewed_trust.rs',
    'tools/long-run-supervisor-074/src/diagnostic_transition.rs',
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
$plan = 'docs/testing/performance/0.74/diagnostic-start-consent-plan-20261016.md'
Paths @($m.selected_sources.path) $sourcePaths 'Selected inventory differs'
Paths @($m.frozen_inputs.path) $frozenPaths 'Frozen inventory differs'
Check ($m.plan.path -ceq $plan) 'Plan path differs'
Push-Location $repo
try {
    $tree = & git rev-parse ($m.source_commit + '^{tree}')
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Tree differs'
    $parent = & git rev-parse ($m.source_commit + '^')
    Check ($LASTEXITCODE -eq 0 -and $parent -ceq $m.clarification_commit) 'Source parent differs'
    foreach ($pair in @(@($m.baseline_commit,$m.preregistration_commit),@($m.preregistration_commit,$m.clarification_commit))) {
        & git merge-base --is-ancestor $pair[0] $pair[1]
        Check ($LASTEXITCODE -eq 0) 'Preregistration ancestry differs'
    }
    $delta = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Check ($LASTEXITCODE -eq 0) 'Cannot inspect source delta'
    Paths $delta @($sourcePaths + $plan) 'Unapproved source path'
    foreach ($f in @($m.selected_sources) + @($m.frozen_inputs) + @($m.plan)) {
        Check ($f.blob -cmatch '^[0-9a-f]{40}$' -and $f.sha256 -cmatch '^[0-9a-f]{64}$') 'Invalid digest'
        $blob = & git rev-parse ($m.source_commit + ':' + $f.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $f.blob) ('Blob differs: ' + $f.path)
        $hash = (Get-FileHash -LiteralPath (Join-Path $repo $f.path) -Algorithm SHA256).Hash.ToLowerInvariant()
        Check ($hash -ceq $f.sha256) ('Working bytes differ: ' + $f.path)
        if ($frozenPaths -ccontains $f.path) {
            $old = & git rev-parse ($m.baseline_commit + ':' + $f.path)
            Check ($LASTEXITCODE -eq 0 -and $old -ceq $f.blob) ('Frozen input differs: ' + $f.path)
        }
    }
    $clarified = & git rev-parse ($m.clarification_commit + ':' + $plan)
    Check ($LASTEXITCODE -eq 0 -and $clarified -ceq $m.plan.blob) 'Plan changed after clarification'
} finally { Pop-Location }
$keys = @(
    'missing-module',
    'private-helper-development',
    'inventory-development',
    'development-green',
    'linux-1.94.0-check',
    'linux-1.94.0-clippy',
    'linux-1.94.0-consent',
    'linux-1.94.0-doctest',
    'linux-1.88.0-check',
    'linux-1.88.0-clippy',
    'linux-1.88.0-consent',
    'linux-1.88.0-doctest',
    'windows-1.94.0-check',
    'windows-1.94.0-clippy',
    'windows-1.94.0-consent',
    'windows-1.94.0-doctest',
    'windows-1.88.0-check',
    'windows-1.88.0-clippy',
    'windows-1.88.0-consent',
    'windows-1.88.0-doctest',
    'integration-1.94.0',
    'integration-1.88.0',
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
    $expected = if ($c.key -cin @('missing-module','private-helper-development','inventory-development','require-ship','final-require-ship')) { 1 } else { 0 }
    Check ($c.exit_code -eq $expected -and ($c.exit_code -is [int] -or $c.exit_code -is [long])) ('Exit differs: ' + $c.key)
    Check ($c.truncated -is [bool] -and $c.truncated -eq $false) 'Capture must be complete'
    Check ($c.log -ceq ($c.key + '.log')) 'Log mapping differs'
    Check ($c.command -match '--offline --locked|fmt --all -- --check|^\.\\target\\debug\\xtask.exe ') 'Unknown command'
    Check ($c.command -notmatch '--test-threads|--skip|--ignored') 'Old test selection changed'
}
$names = @($keys | ForEach-Object { $_ + '.log' }) + @('README.md','verify.ps1')
Paths @($m.artifacts.path) $names 'Artifact inventory differs'
Paths @((Get-ChildItem -LiteralPath $packet -File).Name) @($names + 'manifest.json') 'Unexpected packet file'
foreach ($a in $m.artifacts) {
    Check ($a.path -cmatch '^[A-Za-z0-9.-]+$' -and $a.path -ne 'manifest.json') 'Unsafe artifact path'
    $f = Get-Item -LiteralPath (Join-Path $packet $a.path)
    $hash = (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    Check ($a.sha256 -cmatch '^[0-9a-f]{64}$' -and $hash -ceq $a.sha256 -and $f.Length -eq $a.bytes) ('Artifact differs: ' + $a.path)
}
Has 'missing-module' 'error[E0432]'
Has 'private-helper-development' 'error[E0603]'
Has 'inventory-development' 'test result: FAILED. 6 passed; 1 failed;'
Results 'development-green' '7/0'
Results 'static' '180/0'
foreach ($version in @('1.94.0','1.88.0')) {
    Results ("integration-$version") '24/0,20/0,5/0,6/0'
    foreach ($platform in @('linux','windows')) {
        $prefix = "$platform-$version"
        Results ("$prefix-consent") '7/0'
        Results ("$prefix-doctest") '4/0'
        Has ("$prefix-doctest") 'E0599'
        foreach ($method in @('start_once','publish_starting','release_resources')) { Has ("$prefix-doctest") $method }
        Has ("$prefix-consent") 'signed_leaf_mutations=36 no_unsigned_binding=true'
        Has ("$prefix-consent") 'grammar_cases=30 exclusive_windows=true no_observed_clock=true'
        Has ("$prefix-consent") 'domains=4 legacy_interpretation=false'
        Has ("$prefix-consent") 'supplied_key_only=true repeated_inspection=true freshness_or_replay_authority=false'
        Has ("$prefix-consent") 'independent_threads=4 runtime_authority=false'
        Has ("$prefix-consent") 'bounded_canonical=true unknown_duplicate_partial_refused=true'
        foreach ($action in @('check','clippy')) { Has ("$prefix-$action") 'Finished' }
    }
}
foreach ($prefix in @('','final-')) {
    Has ("${prefix}doc-check") 'doc-check: OK'
    Has ("${prefix}registry") 'OK (176 discovered gates, all registered)'
    Has ("${prefix}local-contract") 'OK (local, non-promotable)'
    Has ("${prefix}require-ship") 'ship admission is closed'
}
Write-Output 'start-consent evidence: OK (signed data only; kernel join/durable Starting/original-start/writers/server closed)'
