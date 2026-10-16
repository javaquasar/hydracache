$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Check($ok, [string]$reason) { if (-not $ok) { throw $reason } }
Check ($m.format_version -eq 1 -and $m.source_commit -ceq '0ec52ddc7aa09b24ceda776f5ec4ca92cdd09a1a' -and $m.baseline_commit -ceq 'b9fb32e1da407d16b7073fbc94820667f3bfdfab') 'Source identity differs'
Check ($m.authenticated_message_preflight -eq $true -and $m.default_parallel_first_attempt_failed -eq $true) 'Missing preflight/negative scope'
foreach ($flag in @('starting_proved','runtime_lease_renewal','continuous_controller_presence_proved','queued_message_causal_freshness_proved','overlap_causality_proved','production_enrollment','server_admitted','performance_claim','binary_or_clean_build_attested','full_msrv_original_group_rerun','new_sigkill_matrix_run')) { Check ($m.$flag -eq $false) ('Scope must stay closed: ' + $flag) }
$paths = @(
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_reserved.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_progress.rs',
    'docs/testing/performance/0.74/diagnostic-collector-progress-contract.toml',
    'docs/testing/performance/0.74/diagnostic-collector-progress-design.md',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_progress.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_progress.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_progress_tests.rs'
)
$frozen = @(
    'Cargo.lock',
    'docs/COMPAT.md',
    'docs/testing/gated-test-registry.toml',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'docs/testing/performance/0.74/local-runs/collector-reserved-20261016/manifest.json',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs',
    'tools/long-run-supervisor-074/src/diagnostic_lease.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs',
    'tools/long-run-supervisor-074/src/diagnostic_early_attempt_storage.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_fence_preflight_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved_crash_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_attempt_crash_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_reservation_publication_storage.rs',
    'tools/long-run-supervisor-074/src/diagnostic_starting_intent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_intent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_intent_projection.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_collector.rs',
    'tools/long-run-supervisor-074/src/diagnostic_observed_reservation.rs',
    'tools/long-run-supervisor-074/src/server.rs'
)
Check ($m.source_files.Count -eq $paths.Count -and @($m.source_files.path | Sort-Object -Unique).Count -eq $paths.Count) 'Source inventory differs'
Check ($m.frozen_files.Count -eq $frozen.Count -and @($m.frozen_files.path | Sort-Object -Unique).Count -eq $frozen.Count) 'Unchanged inventory differs'
Push-Location $repo
try {
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.source_commit)
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Source tree differs'
    & git merge-base --is-ancestor $m.baseline_commit $m.source_commit
    Check ($LASTEXITCODE -eq 0) 'Wrong ancestry'
    foreach ($s in $m.source_files) {
        Check ($paths -ccontains $s.path) 'Unexpected source'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit,$s.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $s.git_blob) 'Source blob differs'
        & git diff --exit-code $m.source_commit -- $s.path
        Check ($LASTEXITCODE -eq 0) ('Working source differs: ' + $s.path)
    }
    $changed = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Check ($LASTEXITCODE -eq 0 -and $changed.Count -eq ($paths.Count + 1)) 'Changed inventory differs'
    $allowed = $paths + 'docs/testing/performance/0.74/diagnostic-collector-progress-plan-20261016.md'
    foreach ($p in $changed) { Check ($allowed -ccontains $p) ('Out-of-scope source: ' + $p) }
    foreach ($s in $m.frozen_files) {
        Check ($frozen -ccontains $s.path) 'Unexpected unchanged input'
        foreach ($commit in @($m.baseline_commit,$m.source_commit)) {
            $blob = & git rev-parse ('{0}:{1}' -f $commit,$s.path)
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
Check ($m.captures.Count -eq 34 -and $m.artifacts.Count -eq 36 -and @($m.captures.file | Sort-Object -Unique).Count -eq 34) 'Capture count differs'
foreach ($c in $m.captures) { Check ($names -ccontains $c.file -and $c.exit_code -eq $c.expected_exit_code -and -not [string]::IsNullOrWhiteSpace($c.cmd)) 'Capture command/outcome differs' }
function Log([string]$name) { Get-Content -LiteralPath (Join-Path $packet ($name + '.log')) -Raw }
Check ((Log 'baseline') -match '1 passed; 0 failed' -and (Log 'baseline-mistarget') -match '0 passed; 0 failed') 'Baseline selection differs'
Check ((Log 'red') -match 'E0599' -and (Log 'red') -match 'bind_authenticated_progress') 'Missing API red'
Check ((Log 'composition-first-failed') -match '4 passed; 1 failed' -and (Log 'composition-first-failed') -match 'Kernel\(Invalid\)') 'Missing development refusal'
Check ((Log 'composition-development') -match '5 passed; 0 failed') 'Missing development composition'
Check ((Log 'original-1.94.0') -match '93 passed; 2 failed; 2 ignored' -and (Log 'original-1.94.0') -match 'Authentication\(Time\)') 'Missing parallel negative'
Check ((Log 'original-1.94.0-isolated') -match '95 passed; 0 failed; 2 ignored') 'Missing unchanged parallel repeat'
Check ((Log 'original-1.94.0-isolated') -notmatch 'HC074-KERNEL-FIRST-REFUSAL') 'Unexpected final original-input trace'
$a = $m.captures | Where-Object file -CEQ 'original-1.94.0.log'
$b = $m.captures | Where-Object file -CEQ 'original-1.94.0-isolated.log'
Check ($a.cmd -ceq $b.cmd -and $b.cmd -notmatch 'test-threads|--skip') 'Parallel repeat must use the same command'
foreach ($v in @('1.94.0','1.88.0')) {
    Check ((Log ('kernel-' + $v)) -match '29 passed; 0 failed; 1 ignored' -and (Log ('kernel-' + $v)) -match 'real_expiry=true receive_time_unchanged=true deadline_extended=false') 'Kernel/actual expiry proof differs'
    Check ((Log ('bytes-' + $v)) -match '30 passed; 0 failed') 'Projection regression differs'
    $d = Log ('docs-' + $v)
    Check ($d -match '21 passed; 0 failed' -and $d -notmatch 'E0432|E0433|E0603') 'Compile-fail boundary differs'
}
Check ((Log 'focused-1.88.0') -match '6 passed; 0 failed' -and (Log 'focused-1.88.0') -match 'binding_mutations=12' -and (Log 'focused-1.88.0') -match 'composition_case=3 preflight_writes=false') 'Missing MSRV composition'
foreach ($platform in @('linux','windows')) { foreach ($v in @('1.94.0','1.88.0')) { foreach ($mode in @('check','clippy')) {
    $name = $platform + '-' + $v + '-' + $mode + '.log'
    $c = $m.captures | Where-Object file -CEQ $name
    Check ($c.exit_code -eq 0 -and $c.cmd -match '--all-targets --all-features --offline --locked') 'Build lane differs'
    if ($mode -ceq 'clippy') { Check ($c.cmd -match '-D warnings') 'Strict Clippy differs' }
} } }
Check ((Log 'static') -match '174 passed; 0 failed') 'Static contract differs'
Check ((Log 'registry') -match '176 discovered gates, all registered') 'Registry differs'
Check ((Log 'doc-check') -match 'doc-check: OK') 'Documentation differs'
Check ((Log 'performance-contract') -match 'OK \(local, non-promotable\)') 'Local contract differs'
Check ((Log 'require-ship') -match 'ship admission is closed') 'Ship admission must stay closed'
Write-Output 'collector-progress evidence OK; message preflight only, overlap negative retained, Starting/runtime/server admission closed'
