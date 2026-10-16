$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Check($ok,[string]$why) { if (-not $ok) { throw $why } }
function Log([string]$name) { Get-Content -LiteralPath (Join-Path $packet ($name + '.log')) -Raw }
function NextState([uint32]$value) {
    $value = [uint32]($value -bxor (([uint64]$value -shl 13) -band 4294967295L))
    $value = [uint32]($value -bxor ([uint64]$value -shr 17))
    [uint32]($value -bxor (([uint64]$value -shl 5) -band 4294967295L))
}
function Bootstrap([double[]]$ratios) {
    Check ($ratios.Count -eq 5) 'Bootstrap needs five pairs'
    $logs = @($ratios | ForEach-Object { Check ($_ -gt 0 -and -not [double]::IsInfinity($_) -and -not [double]::IsNaN($_)) 'Invalid ratio'; [Math]::Log($_) })
    [uint32]$state = 0x0740c0a0
    $samples = New-Object double[] 10000
    for ($b=0; $b -lt 10000; $b++) {
        [double]$sum = 0
        for ($i=0; $i -lt 5; $i++) { $state = NextState $state; $sum += $logs[[int]($state % 5)] }
        $samples[$b] = [Math]::Exp($sum / 5)
    }
    [Array]::Sort($samples)
    $bounds = @()
    foreach ($p in @(0.025,0.975)) {
        $rank = 9999 * $p
        $j = [int][Math]::Floor($rank)
        $fraction = $rank - $j
        $bounds += $samples[$j] + $fraction * ($samples[[int][Math]::Ceiling($rank)] - $samples[$j])
    }
    [pscustomobject]@{ ratio=[Math]::Exp(($logs | Measure-Object -Sum).Sum/5); lower95=$bounds[0]; upper95=$bounds[1] }
}
[uint32]$seed = 0x0740c0a0
$indices = @()
for ($i=0;$i -lt 10;$i++) { $seed=NextState $seed; $indices += [int]($seed % 5) }
Check (($indices -join ',') -ceq '0,0,0,2,3,0,4,3,4,2') 'PRNG differs'
foreach ($constant in @(1.0,0.5)) {
    $probe = Bootstrap @($constant,$constant,$constant,$constant,$constant)
    foreach ($field in @('ratio','lower95','upper95')) { Check ([Math]::Abs($probe.$field - $constant) -lt 1e-12) 'Estimator selfcheck failed' }
}
Check ($m.format_version -eq 1 -and $m.source_commit -ceq 'cca2cb68a6d00e8fa4af6f08f9b59cdf199956a6' -and $m.baseline_commit -ceq '73f71f07e95696ae9263939e82dac6a73f09ce8c') 'Source identity differs'
foreach ($f in @('local_tooling_effect_established','five_pairs_complete','full_constructors_preserved','live_guards_unchanged','prior_failed_screen_unchanged')) { Check ($m.$f -eq $true) ('Required scope: ' + $f) }
foreach ($f in @('qualification_or_product_performance_claim','timing_is_cpu_measurement','allocation_effect_measured','historical_cause_proved','shared_load_fixed','ipc_retried','screen_retried','old_tests_skipped_or_serialized','authorization_window_changed','starting_proved','runtime_handoff','writer_cessation_proved','server_admitted','binary_or_clean_build_attested','new_sigkill_matrix_run','full_release_verify_run','push_performed')) { Check ($m.$f -eq $false) ('Scope must stay closed: ' + $f) }
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
    'docs/testing/performance/0.74/local-runs/validation-cost-20261016/manifest.json',
    'docs/testing/performance/0.74/local-runs/worker-proof-reuse-20261016/manifest.json',
    'docs/testing/performance/0.74/diagnostic-worker-proof-reuse-plan-20261016.md'
)
Push-Location $repo
try {
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.source_commit)
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Tree differs'
    $prereg = & git rev-parse ab1a3ee7
    Check ($LASTEXITCODE -eq 0 -and $prereg -ceq $m.preregistration_commit) 'Preregistration identity differs'
    & git merge-base --is-ancestor $m.baseline_commit $prereg
    Check ($LASTEXITCODE -eq 0) 'Baseline ancestry differs'
    & git merge-base --is-ancestor $prereg $m.source_commit
    Check ($LASTEXITCODE -eq 0) 'Preregistration was not before candidate'
    $plan = 'docs/testing/performance/0.74/diagnostic-worker-proof-reuse-v2-plan-20261016.md'
    foreach ($c in @($prereg,$m.source_commit)) {
        $blob = & git rev-parse ('{0}:{1}' -f $c,$plan)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $m.preregistration_blob) 'Preregistration changed'
    }
    & git diff --exit-code $m.source_commit -- $plan
    Check ($LASTEXITCODE -eq 0) 'Working preregistration changed'
    Check ($m.source_files.Count -eq 8 -and @($m.source_files.path | Sort-Object -Unique).Count -eq 8) 'Source inventory differs'
    foreach ($s in $m.source_files) {
        Check ($sourcePaths -ccontains $s.path) 'Unexpected source'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit,$s.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $s.git_blob) 'Source blob differs'
    }
    & git diff --exit-code $m.source_commit -- @sourcePaths
    Check ($LASTEXITCODE -eq 0) 'Working source differs'
    $changed = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Check ($LASTEXITCODE -eq 0 -and $changed.Count -eq 9) 'Changed inventory differs'
    foreach ($p in $changed) { Check (($sourcePaths + @($plan)) -ccontains $p) 'Unexpected candidate change' }
    & git diff --exit-code 78d8d9fc $m.baseline_commit -- tools/long-run-supervisor-074/src crates/xtask/tests docs/testing/gated-test-registry.toml
    Check ($LASTEXITCODE -eq 0) 'Retained binary reference source differs'
    Check ($m.frozen_files.Count -eq 33 -and @($m.frozen_files.path | Sort-Object -Unique).Count -eq 33) 'Frozen inventory differs'
    foreach ($s in $m.frozen_files) {
        Check ($frozenPaths -ccontains $s.path) 'Unexpected unchanged input'
        foreach ($c in @($m.baseline_commit,$m.source_commit)) {
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
    $path = Join-Path $packet $a.file
    Check ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a.sha256 -and (Get-Item -LiteralPath $path).Length -eq $a.bytes) ('Artifact differs: ' + $a.file)
}
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object Name -CNE 'manifest.json' | ForEach-Object Name | Sort-Object)
Check (($actual -join "`n") -ceq (($names | Sort-Object) -join "`n")) 'Artifact inventory differs'
$expected = @('baseline-source','baseline-binary','baseline-preflight','red','green','fmt-development-red','fmt','binary-command-development-red','binary-before','binary-after','static','doc-check','registry','local-contract','require-ship','prebuild-1.94.0','prebuild-1.88.0','final-doc-check','final-registry','final-local-contract','final-require-ship')
foreach ($p in @('linux','windows')) { foreach ($v in @('1.94.0','1.88.0')) { foreach ($mode in @('check','clippy','unit')) { $expected += "$p-$v-$mode" } } }
foreach ($v in @('1.94.0','1.88.0')) { foreach ($mode in @('original','kernel','semantics','credentials','causal','docs')) { $expected += "$mode-$v" } }
$order = @()
for ($pair=1;$pair -le 5;$pair++) { $sides=if ($pair % 2 -eq 1) { @('B','C') } else { @('C','B') }; foreach ($side in $sides) { $key="pair-$pair-$side"; $expected += $key; $order += "$key.log" } }
Check ($expected.Count -eq 55 -and $m.captures.Count -eq 55 -and @($m.captures.file | Sort-Object -Unique).Count -eq 55 -and $m.artifacts.Count -eq 57) 'Capture inventory differs'
$negative = @('red','fmt-development-red','binary-command-development-red','require-ship','final-require-ship')
foreach ($c in $m.captures) {
    $key = $c.file -creplace '\.log$',''
    Check ($expected -ccontains $key -and $names -ccontains $c.file -and -not [string]::IsNullOrWhiteSpace($c.cmd)) 'Unexpected capture'
    Check ($c.exit_code -eq $c.expected_exit_code -and $c.exit_code -eq $(if ($negative -ccontains $key) { 1 } else { 0 })) ('Unexpected outcome: ' + $key)
}
Check ((Log 'red') -match '3 passed; 2 failed' -and (Log 'red') -match 'left: 18' -and (Log 'red') -match 'left: 10' -and (Log 'green') -match '5 passed; 0 failed') 'Mechanism development differs'
Check ((Log 'fmt-development-red') -match 'diagnostic_validation_cost_tests.rs' -and (Log 'binary-command-development-red') -match 'No such file or directory') 'Development failures missing'
Check ((Log 'baseline-preflight') -match '1 passed; 0 failed' -and (Log 'static') -match '177 passed; 0 failed') 'Baseline or contracts differ'
foreach ($p in @('linux','windows')) { foreach ($v in @('1.94.0','1.88.0')) {
    Check ((Log "$p-$v-unit") -match '5 passed; 0 failed') 'Portable proof differs'
    foreach ($mode in @('check','clippy')) {
        $c = $m.captures | Where-Object file -CEQ ("$p-$v-$mode.log")
        Check ($c.cmd -match '--all-targets --all-features --offline --locked') 'Build scope differs'
        if ($mode -ceq 'clippy') { Check ($c.cmd -match '-D warnings') 'Clippy strictness differs' }
    }
} }
foreach ($v in @('1.94.0','1.88.0')) {
    Check ((Log "original-$v") -match '99 passed; 0 failed; 4 ignored' -and (Log "original-$v") -notmatch 'HC074-VALIDATION-COST|HC074-KERNEL-FIRST-REFUSAL') 'Original regression differs'
    Check ((Log "kernel-$v") -match '29 passed; 0 failed; 1 ignored' -and (Log "credentials-$v") -match '15 passed; 0 failed') 'Kernel or credential regression differs'
    foreach ($n in @(30,24,13,15)) { Check ((Log "semantics-$v") -match "$n passed; 0 failed") 'Semantics differs' }
    $causal = Log "causal-$v"
    Check ($causal -match '1 passed; 0 failed' -and [regex]::Matches($causal,'collector_causal live_case=[0-3]').Count -eq 4) 'Causal cases differ'
    $d = Log "docs-$v"
    Check ($d -match '28 passed; 0 failed' -and $d -notmatch 'E0432|E0433|E0603') 'Compile-fail boundary differs'
}
foreach ($c in $m.captures | Where-Object { $_.file -match '^(original-|semantics-)' }) { Check ($c.cmd -notmatch 'test-threads|--skip') 'Regression skipped or serialized' }
Check ($m.baseline_binary_sha256 -ceq '7e8247610919a8dc39f3f6b7f6644944084a36a78cf949045fe9a0eaa7a946ad' -and $m.candidate_binary_sha256 -ceq '65efc5332917079da35262bd9814099944bfd9551f8c5c33e753ae83d367f32e') 'Binary identity differs'
Check ((Log 'binary-before') -ceq (Log 'binary-after')) 'Binary changed during screen'
foreach ($side in @('baseline','candidate')) { Check ((Log 'binary-before').Contains($m.("${side}_binary_sha256") + '  ' + $m.("${side}_binary_path"))) 'Binary receipt missing' }
Check ($m.observations.Count -eq 10 -and ($m.observations.capture -join ',') -ceq ($order -join ',')) 'Observation order differs'
$counts = @{Setup=@(11,99);Fence=@(8,72);Retain=@(96,908);Observe=@(32,308);Publish=@(272,2696);Bind=@(66,652)}
foreach ($o in $m.observations) {
    Check ($o.stages.Count -eq 6 -and @($o.stages.stage | Sort-Object -Unique).Count -eq 6 -and $o.capture -ceq ("pair-" + $o.pair + '-' + $o.side + '.log')) 'Stage identity differs'
    $raw = Get-Content -LiteralPath (Join-Path $packet $o.capture) -Raw
    Check ($raw -match '1 passed; 0 failed' -and [regex]::Matches($raw,'HC074-VALIDATION-STAGE ').Count -eq 6 -and $raw.Contains('validation_cost original_guards=true source=debug_fixture authority_renewal=false starting=false')) 'Probe semantic guard differs'
    $binary = if ($o.side -ceq 'B') { $m.baseline_binary_path } else { $m.candidate_binary_path }
    $c = $m.captures | Where-Object file -CEQ $o.capture
    Check ($c.cmd -ceq ('wsl.exe -d Ubuntu --cd /mnt/c/Workspace/prj/jq/cashe/hydracache-074 --exec ' + $binary + ' ' + $m.probe_fqn + ' --ignored --exact --nocapture')) 'Probe command differs'
    foreach ($s in $o.stages) {
        $workers = if ($o.side -ceq 'B') { $counts[$s.stage][1] } elseif ($s.stage -ceq 'Setup') { 2 } else { 0 }
        Check ($counts.ContainsKey($s.stage) -and $s.authentication_calls -eq $counts[$s.stage][0] -and $s.signature_calls -eq $s.authentication_calls -and $s.worker_policy_calls -eq $workers) 'Stage counts differ'
        $line = 'HC074-VALIDATION-STAGE {0} Stats {{ wall_ns: {1}, authentication_calls: {2}, authentication_ns: {3}, signature_calls: {4}, signature_ns: {5}, worker_policy_calls: {6}, worker_policy_ns: {7} }}' -f $s.stage,$s.wall_ns,$s.authentication_calls,$s.authentication_ns,$s.signature_calls,$s.signature_ns,$s.worker_policy_calls,$s.worker_policy_ns
        Check ($raw.Contains($line) -and $s.wall_ns -gt 0 -and $s.authentication_ns -ge $s.signature_ns -and $s.wall_ns -ge $s.worker_policy_ns) 'Stage duration transcription differs'
        if ($workers -eq 0) { Check ($s.worker_policy_ns -eq 0) 'Absent verifier charged time' }
    }
}
Check ($m.pairs.Count -eq 5 -and ($m.pairs.pair -join ',') -ceq '1,2,3,4,5' -and $m.statistics.Count -eq 2 -and ($m.statistics.stage -join ',') -ceq 'Publish,Bind') 'Pair inventory differs'
foreach ($stage in @('Publish','Bind')) {
    $ratios = @()
    foreach ($pair in $m.pairs) {
        Check ($pair.order -ceq $(if ($pair.pair % 2 -eq 1) { 'BC' } else { 'CB' })) 'Pair order differs'
        $b = ($m.observations | Where-Object { $_.pair -eq $pair.pair -and $_.side -ceq 'B' }).stages | Where-Object stage -CEQ $stage
        $c = ($m.observations | Where-Object { $_.pair -eq $pair.pair -and $_.side -ceq 'C' }).stages | Where-Object stage -CEQ $stage
        Check ($pair.primary.$stage.baseline_ns -eq $b.wall_ns -and $pair.primary.$stage.candidate_ns -eq $c.wall_ns) 'Pair transcription differs'
        $ratios += [double]$c.wall_ns / [double]$b.wall_ns
    }
    $result = Bootstrap $ratios
    $recorded = $m.statistics | Where-Object stage -CEQ $stage
    foreach ($field in @('ratio','lower95','upper95')) { Check ([Math]::Abs($result.$field - $recorded.$field) -lt 1e-12) 'Statistics differ' }
    Check ($recorded.pass -eq $true -and $result.upper95 -le 0.80) 'Practical floor failed'
}
foreach ($prefix in @('','final-')) {
    Check ((Log ($prefix+'doc-check')) -match 'doc-check: OK' -and (Log ($prefix+'registry')) -match '176 discovered gates, all registered') 'Read-only gates differ'
    Check ((Log ($prefix+'local-contract')) -match 'OK \(local, non-promotable\)' -and (Log ($prefix+'require-ship')) -match 'ship admission is closed') 'Admission differs'
}
Write-Output 'worker proof second screen OK; five pairs and fixed bootstrap reproduced; local tooling only, historical IPC and server gates open'
