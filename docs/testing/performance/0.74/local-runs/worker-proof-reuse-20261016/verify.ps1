$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Check($ok,[string]$why) { if (-not $ok) { throw $why } }
function Log([string]$name) { Get-Content -LiteralPath (Join-Path $packet ($name + '.log')) -Raw }
Check ($m.format_version -eq 1 -and $m.source_commit -ceq 'e29694bd2d9c989d1416940c988e4918df605da5') 'Source identity differs'
Check ($m.candidate_reverted -eq $true -and $m.screen_failed -eq $true -and $m.host_busy_negative_retained -eq $true) 'Negative or rollback differs'
foreach ($f in @('effect_established','candidate_active','remaining_pairs_run','screen_retried','ipc_retried','old_tests_skipped_or_serialized','authorization_window_changed','historical_cause_proved','shared_load_fixed','timing_is_cpu_measurement','starting_proved','runtime_handoff','server_admitted','qualification_or_product_performance_claim','binary_or_clean_build_attested','new_sigkill_matrix_run','push_performed')) { Check ($m.$f -eq $false) ('Scope must stay closed: ' + $f) }
Check ($null -eq $m.statistics -and $m.completed_pairs -eq 0 -and $m.started_pairs -eq 1) 'Incomplete screen became an estimate'
$sourcePaths = @(
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_validation_cost.rs',
    'crates/xtask/tests/support/performance_contract_074_worker_proof.rs',
    'docs/testing/gated-test-registry.toml',
    'docs/testing/performance/0.74/diagnostic-worker-proof-reuse-design.md',
    'tools/long-run-supervisor-074/src/diagnostic_validation_cost_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_proof_reuse_tests.rs'
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
    'tools/long-run-supervisor-074/src/server.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_trust_input_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_progress_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs',
    'docs/testing/performance/0.74/local-runs/validation-cost-20261016/manifest.json'
)
Push-Location $repo
try {
    $baseline = & git rev-parse '78d8d9fc^{commit}'
    Check ($LASTEXITCODE -eq 0 -and $baseline -ceq $m.baseline_commit) 'Wrong baseline'
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.source_commit)
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Source tree differs'
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.rollback_commit)
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.rollback_tree) 'Rollback tree differs'
    $plan = 'docs/testing/performance/0.74/diagnostic-worker-proof-reuse-plan-20261016.md'
    $prereg = & git rev-parse e7d84c67
    Check ($LASTEXITCODE -eq 0 -and $prereg -ceq $m.final_preregistration_commit) 'Preregistration identity differs'
    & git merge-base --is-ancestor $prereg $m.source_commit
    Check ($LASTEXITCODE -eq 0) 'Preregistration was not before candidate'
    foreach ($c in @($prereg,$m.source_commit,$m.rollback_commit)) {
        $blob = & git rev-parse ('{0}:{1}' -f $c,$plan)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $m.preregistration_blob) 'Preregistration changed'
    }
    & git diff --exit-code $m.source_commit -- $plan
    Check ($LASTEXITCODE -eq 0) 'Working preregistration changed'
    foreach ($pair in @(@($m.baseline_commit,$m.source_commit),@($m.source_commit,$m.rollback_commit))) {
        & git merge-base --is-ancestor $pair[0] $pair[1]
        Check ($LASTEXITCODE -eq 0) 'Wrong ancestry'
    }
    Check ($m.source_files.Count -eq 8 -and @($m.source_files.path | Sort-Object -Unique).Count -eq 8) 'Source inventory differs'
    foreach ($s in $m.source_files) {
        Check ($sourcePaths -ccontains $s.path) 'Unexpected source'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit,$s.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $s.git_blob) 'Candidate source blob differs'
    }
    $changed = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Check ($LASTEXITCODE -eq 0 -and $changed.Count -eq 9) 'Candidate changed inventory differs'
    foreach ($p in $changed) { Check (($sourcePaths + @('docs/testing/performance/0.74/diagnostic-worker-proof-reuse-plan-20261016.md')) -ccontains $p) ('Unexpected candidate change: ' + $p) }
    $rolled = @(& git diff --name-only $m.source_commit $m.rollback_commit)
    Check ($LASTEXITCODE -eq 0 -and ($rolled -join "`n") -ceq (($sourcePaths | Sort-Object -CaseSensitive) -join "`n")) 'Rollback is not the exact candidate'
    $left = @(& git diff --name-only $m.baseline_commit $m.rollback_commit)
    Check ($LASTEXITCODE -eq 0 -and $left.Count -eq 1 -and $left[0] -ceq 'docs/testing/performance/0.74/diagnostic-worker-proof-reuse-plan-20261016.md') 'Rollback changed baseline or lost preregistration'
    & git diff --exit-code $m.rollback_commit -- @sourcePaths
    Check ($LASTEXITCODE -eq 0) 'Working candidate paths differ from rollback'
    foreach ($p in @('crates/xtask/tests/support/performance_contract_074_worker_proof.rs','tools/long-run-supervisor-074/src/diagnostic_worker_proof_reuse_tests.rs','docs/testing/performance/0.74/diagnostic-worker-proof-reuse-design.md')) { Check (-not (Test-Path -LiteralPath $p)) 'Deleted candidate path reappeared' }
    Check ($m.frozen_files.Count -eq 31 -and @($m.frozen_files.path | Sort-Object -Unique).Count -eq 31) 'Frozen inventory differs'
    foreach ($s in $m.frozen_files) {
        Check ($frozenPaths -ccontains $s.path) 'Unexpected frozen path'
        foreach ($c in @($m.baseline_commit,$m.source_commit,$m.rollback_commit)) {
            $blob = & git rev-parse ('{0}:{1}' -f $c,$s.path)
            Check ($LASTEXITCODE -eq 0 -and $blob -ceq $s.git_blob) 'Frozen blob differs'
        }
        Check ((Get-FileHash -LiteralPath $s.path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $s.sha256) ('Frozen bytes differ: ' + $s.path)
    }
} finally { Pop-Location }
$names = @()
foreach ($a in $m.artifacts) {
    Check ($a.file -cmatch '^[A-Za-z0-9][A-Za-z0-9.-]*$' -and $names -cnotcontains $a.file) 'Unsafe or duplicate artifact'
    $names += $a.file
    $p = Join-Path $packet $a.file
    Check ((Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a.sha256 -and (Get-Item -LiteralPath $p).Length -eq $a.bytes) ('Artifact differs: ' + $a.file)
}
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object Name -CNE 'manifest.json' | ForEach-Object Name | Sort-Object)
Check (($actual -join "`n") -ceq (($names | Sort-Object) -join "`n")) 'Artifact inventory differs'
$expectedKeys = @('baseline','red','green','static-development-red','static','fmt','doc-check','registry','performance-contract','require-ship','prebuild-1.94.0','prebuild-1.88.0','semantics-unreached-1.94.0','binary-before','pair-1-B','pair-1-C','binary-after','rollback-static','rollback-fmt','rollback-original','final-doc-check','final-registry','final-performance-contract','final-require-ship')
foreach ($p in @('linux','windows')) { foreach ($v in @('1.94.0','1.88.0')) {
    foreach ($mode in @('check','clippy','unit')) { $expectedKeys += "$p-$v-$mode" }
    foreach ($mode in @('check','clippy','policy')) { $expectedKeys += "rollback-$p-$v-$mode" }
} }
foreach ($v in @('1.94.0','1.88.0')) { foreach ($mode in @('original','kernel','semantics','credentials','causal','docs')) { $expectedKeys += "$mode-$v" } }
Check ($expectedKeys.Count -eq 60 -and $m.captures.Count -eq 60 -and @($m.captures.file | Sort-Object -Unique).Count -eq 60 -and $m.artifacts.Count -eq 64) 'Capture inventory differs'
$negative = @('red','static-development-red','semantics-1.94.0','require-ship','pair-1-C','final-require-ship')
foreach ($c in $m.captures) {
    $key = $c.file -creplace '\.log$',''
    Check ($expectedKeys -ccontains $key -and $names -ccontains $c.file -and -not [string]::IsNullOrWhiteSpace($c.cmd)) 'Unexpected capture'
    Check ($c.exit_code -eq $c.expected_exit_code -and $c.exit_code -eq $(if ($negative -ccontains $key) { 1 } else { 0 })) ('Unexpected outcome: ' + $key)
}
Check ((Log 'red') -match '3 passed; 1 failed' -and (Log 'red') -match 'left: 10' -and (Log 'green') -match '4 passed; 0 failed') 'Mechanism development differs'
Check ((Log 'static-development-red') -match '176 passed; 1 failed' -and (Log 'static') -match '177 passed; 0 failed' -and (Log 'rollback-static') -match '176 passed; 0 failed') 'Static gates differ'
foreach ($p in @('linux','windows')) { foreach ($v in @('1.94.0','1.88.0')) {
    Check ((Log "$p-$v-unit") -match '4 passed; 0 failed') 'Candidate proof tests differ'
    $n = if ($p -ceq 'linux') { 15 } else { 14 }
    Check ((Log "rollback-$p-$v-policy") -match "$n passed; 0 failed") 'Restored worker tests differ'
    foreach ($prefix in @('','rollback-')) { foreach ($mode in @('check','clippy')) {
        $c = $m.captures | Where-Object file -CEQ ("$prefix$p-$v-$mode.log")
        Check ($c.cmd -match '--all-targets --all-features --offline --locked') 'Build scope differs'
        if ($mode -ceq 'clippy') { Check ($c.cmd -match '-D warnings') 'Strict Clippy differs' }
    } }
} }
foreach ($v in @('1.94.0','1.88.0')) {
    Check ((Log "original-$v") -match '99 passed; 0 failed; 4 ignored') 'Original candidate regression differs'
    Check ((Log "kernel-$v") -match '29 passed; 0 failed; 1 ignored') 'Kernel regression differs'
    Check ((Log "credentials-$v") -match '15 passed; 0 failed') 'Credential regression differs'
    $causal = Log "causal-$v"
    Check ($causal -match '1 passed; 0 failed' -and [regex]::Matches($causal,'collector_causal live_case=[0-3]').Count -eq 4) 'Four causal cases differ'
    $d = Log "docs-$v"
    Check ($d -match '28 passed; 0 failed' -and $d -notmatch 'E0432|E0433|E0603') 'Compile-fail boundary differs'
}
Check ((Log 'rollback-original') -match '99 passed; 0 failed; 4 ignored') 'Rollback original regression differs'
Check ((Log 'semantics-1.94.0') -match '23 passed; 1 failed' -and (Log 'semantics-1.94.0') -match 'Host\(Busy\)' -and (Log 'semantics-1.94.0') -match 'corrupt_future_noncanonical_or_pending_ledgers_fail_closed') 'IPC negative missing'
foreach ($n in @(30,24,13,15)) { Check ((Log 'semantics-1.88.0') -match "$n passed; 0 failed") 'MSRV semantics differs' }
foreach ($n in @(13,15)) { Check ((Log 'semantics-unreached-1.94.0') -match "$n passed; 0 failed") 'Unreached targets differ' }
foreach ($c in $m.captures | Where-Object { $_.file -match '^(original-|semantics-|rollback-original)' }) { Check ($c.cmd -notmatch 'test-threads|--skip') 'Regression skipped or serialized' }
Check ((Log 'pair-1-B') -match '1 passed; 0 failed') 'First baseline failed'
$candidate = Log 'pair-1-C'
Check ($candidate -match '0 passed; 1 failed' -and $candidate -match 'diagnostic_validation_cost_tests.rs:61:9' -and $candidate -match 'left: 2' -and $candidate -match 'right: 1' -and $candidate -notmatch 'HC074-VALIDATION-STAGE') 'First candidate negative differs'
Check (($m.pair_captures -join ',') -ceq 'pair-1-B.log,pair-1-C.log') 'Screen order differs'
foreach ($side in @('B','C')) {
    $binary = if ($side -ceq 'B') { $m.baseline_binary_path } else { $m.candidate_binary_path }
    $cmd = 'wsl.exe -d Ubuntu --cd /mnt/c/Workspace/prj/jq/cashe/hydracache-074 --exec ' + $binary + ' ' + $m.probe_fqn + ' --ignored --exact --nocapture'
    $c = $m.captures | Where-Object file -CEQ ("pair-1-$side.log")
    Check ($c.cmd -ceq $cmd) 'Direct binary invocation differs'
}
Check ($m.baseline_binary_sha256 -ceq '7e8247610919a8dc39f3f6b7f6644944084a36a78cf949045fe9a0eaa7a946ad' -and $m.candidate_binary_sha256 -ceq 'c6e9cc00e6f4c0f40b9267429edb6f8ee1a0810cf7c4ced710e821bb35c0bcf6') 'Binary identities differ'
Check ((Log 'binary-before') -ceq (Log 'binary-after')) 'Binary changed during screen'
foreach ($s in @('baseline','candidate')) { Check ((Log 'binary-before').Contains($m.("${s}_binary_sha256") + '  ' + $m.("${s}_binary_path"))) 'Missing binary receipt' }
$counts = @{Setup=@(11,99);Fence=@(8,72);Retain=@(96,908);Observe=@(32,308);Publish=@(272,2696);Bind=@(66,652)}
Check ($m.baseline_stage_observations.Count -eq 6 -and @($m.baseline_stage_observations.stage | Sort-Object -Unique).Count -eq 6) 'Baseline stages differ'
foreach ($s in $m.baseline_stage_observations) {
    Check ($counts.ContainsKey($s.stage) -and $s.authentication_calls -eq $counts[$s.stage][0] -and $s.signature_calls -eq $s.authentication_calls -and $s.worker_policy_calls -eq $counts[$s.stage][1]) 'Baseline counts differ'
    $line = 'HC074-VALIDATION-STAGE {0} Stats {{ wall_ns: {1}, authentication_calls: {2}, authentication_ns: {3}, signature_calls: {4}, signature_ns: {5}, worker_policy_calls: {6}, worker_policy_ns: {7} }}' -f $s.stage,$s.wall_ns,$s.authentication_calls,$s.authentication_ns,$s.signature_calls,$s.signature_ns,$s.worker_policy_calls,$s.worker_policy_ns
    Check ((Log 'pair-1-B').Contains($line)) 'Baseline timing transcription differs'
}
foreach ($prefix in @('','final-')) {
    Check ((Log ($prefix+'doc-check')) -match 'doc-check: OK' -and (Log ($prefix+'registry')) -match '176 discovered gates, all registered') 'Read-only gates differ'
    Check ((Log ($prefix+'performance-contract')) -match 'OK \(local, non-promotable\)' -and (Log ($prefix+'require-ship')) -match 'ship admission is closed') 'Ship admission changed'
}
Write-Output 'worker proof screen negative retained; exact candidate rolled back; IPC and server gates remain open'
