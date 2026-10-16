param([string]$ManifestPath = (Join-Path $PSScriptRoot 'manifest.json'))
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../../../..')).Path
function Check($ok, [string]$message) { if (-not $ok) { throw $message } }
function Same($actual, $expected, [string]$message) {
    Check ((@($actual | Sort-Object) -join '|') -ceq (@($expected | Sort-Object) -join '|')) $message
}
Check ((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'candidate.patch') -Algorithm SHA256).Hash.ToLowerInvariant() -ceq 'cfeb99884063d51dfdaea98f23c3a0529a25bc19acae54c529f1ac7c99f4377f') 'Candidate patch differs'
$m = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
Check ($m.format_version -eq 1 -and $m.kind -ceq 'uncommitted-candidate-working-copy-observation') 'Wrong packet kind'
Check ($m.parent_commit -ceq 'fc374ea72624a5757456970aa56f2b86d185c18f') 'Parent differs'
Check ($m.preregistration_commit -ceq $m.parent_commit) 'Preregistration differs'
Check ($m.baseline_commit -ceq '47372f9fe5d634193c8eeae90d1d7b3005d5a380') 'Baseline differs'
$scope = @('candidate_committed','live_fault_matrix_complete','all_release_budget_chains_complete','durable_starting_written','new_sigkill_matrix_run','authenticated_process_started','all_writers_revoked','server_mutated','product_workload_run','deadline_changed','time_service_changed','qualification_changed','server_ready','full_release_verify_run')
Same @($m.scope.PSObject.Properties.Name) $scope 'Scope inventory differs'
foreach ($key in $scope) { Check ($m.scope.$key -is [bool] -and -not $m.scope.$key) ('Unsupported scope claim: ' + $key) }
$selected = @('crates/xtask/tests/performance_contract_074.rs','crates/xtask/tests/support/performance_contract_074_consent_budget.rs','crates/xtask/tests/support/performance_contract_074_validation_frame.rs','docs/testing/gated-test-registry.toml','tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs','tools/long-run-supervisor-074/src/diagnostic_original_start_consent_tests.rs','tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs','tools/long-run-supervisor-074/src/diagnostic_validation_frame_tests.rs','tools/long-run-supervisor-074/src/diagnostic_validation_frame_budget_tests.rs')
$frozen = @('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml','docs/COMPAT.md','tools/long-run-supervisor-074/src/diagnostic_consent_budget_tests.rs','tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs','tools/long-run-supervisor-074/src/diagnostic_oidc_input.rs','tools/long-run-supervisor-074/src/diagnostic_controller_build_input.rs','tools/long-run-supervisor-074/src/diagnostic_controller_trust_input.rs','tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs','tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs','tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs','tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs','tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs','tools/long-run-supervisor-074/src/diagnostic_kernel_start_consent.rs','tools/long-run-supervisor-074/src/diagnostic_worker_progress.rs','tools/long-run-supervisor-074/src/diagnostic_worker_start_consent.rs','tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs','tools/long-run-supervisor-074/src/diagnostic_collector_progress.rs','tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs','tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs','tools/long-run-supervisor-074/src/diagnostic_transition.rs','docs/testing/performance/0.74/qualification-manifest.toml')
Same @($m.selected_sources.path) $selected 'Selected source inventory differs'
Same @($m.frozen_inputs.path) $frozen 'Frozen source inventory differs'
Check ($m.selected_sources.Count -eq $selected.Count -and $m.frozen_inputs.Count -eq $frozen.Count) 'Duplicate source'
Push-Location $repo
try {
    & git merge-base --is-ancestor $m.baseline_commit $m.preregistration_commit
    Check ($LASTEXITCODE -eq 0) 'Preregistration ancestry differs'
    foreach ($file in @($m.selected_sources) + @($m.frozen_inputs)) {
        Check ((Get-FileHash -LiteralPath $file.path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) ('Working bytes differ: ' + $file.path)
        $blob = & git hash-object -- $file.path
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $file.git_blob) ('Working Git content differs: ' + $file.path)
        if ($frozen -ccontains $file.path) {
            $old = & git rev-parse ($m.baseline_commit + ':' + $file.path)
            Check ($LASTEXITCODE -eq 0 -and $old -ceq $blob) ('Frozen predicate differs: ' + $file.path)
        }
    }
} finally { Pop-Location }
$keys = @('frame-red-api','frame-fast-194','frame-budget-dev-194','frame-budget-dev-188','frame-release-build-194','frame-release-build-188','frame-static-focused','frame-static-full','frame-static-budget-fix','frame-static-budget-fixed','frame-budget-release-194','frame-budget-release-188','frame-fault-194','frame-clock-observation','frame-clock-context','frame-clock-windows','frame-linux-194-check','frame-linux-188-check','frame-linux-194-clippy','frame-linux-188-clippy','frame-win-194-check','frame-win-188-check','frame-win-194-clippy','frame-win-188-clippy','frame-win-194-xtask-clippy','frame-win-188-xtask-clippy','frame-linux-188-docs','frame-linux-194-fast','frame-static-final','frame-doc-check','frame-registry','frame-local-contract','frame-ship-guard','frame-fmt','frame-linux-194-docs','frame-linux-188-fast','frame-linux-194-regression','frame-linux-188-regression','frame-release-binary-observations','frame-doc-check-final')
Same @($m.commands.key) $keys 'Command inventory differs'
Same @($m.artifacts.path) @($keys | ForEach-Object { 'logs/' + $_ + '.json' }) 'Artifact inventory differs'
Check ($m.commands.Count -eq $keys.Count -and $m.artifacts.Count -eq $keys.Count) 'Duplicate capture'
$failures = @('frame-red-api','frame-static-full','frame-static-budget-fix','frame-fault-194','frame-ship-guard')
$logs = @{}
foreach ($cmd in $m.commands) {
    $path = 'logs/' + $cmd.key + '.json'
    $artifact = @($m.artifacts | Where-Object path -CEQ $path)
    Check ($artifact.Count -eq 1) ('Missing artifact: ' + $cmd.key)
    $file = Join-Path $PSScriptRoot $path
    Check ((Get-Item -LiteralPath $file).Length -eq $artifact[0].bytes) ('Length differs: ' + $path)
    Check ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $artifact[0].sha256) ('Hash differs: ' + $path)
    $capture = Get-Content -LiteralPath $file -Raw | ConvertFrom-Json
    $expectedExit = if ($failures -ccontains $cmd.key) { 1 } else { 0 }
    Check ($capture.format_version -eq 1 -and $capture.key -ceq $cmd.key) ('Capture identity differs: ' + $cmd.key)
    Check ($cmd.exit_code -eq $expectedExit -and $capture.exit_code -eq $expectedExit) ('Exit differs: ' + $cmd.key)
    Check (-not [string]::IsNullOrWhiteSpace($capture.command)) ('Missing command: ' + $cmd.key)
    $logs[$cmd.key] = $capture.output
}
$expected = @{
    'frame-budget-dev-194' = @(6,8,11)
    'frame-budget-dev-188' = @(10,10,11)
    'frame-budget-release-194' = @(19,19,5)
    'frame-budget-release-188' = @(19,19,19)
}
Same @($m.budget_samples.key) @($expected.Keys) 'Budget cohort differs'
Check ($m.budget_samples.Count -eq 4) 'Duplicate budget cohort'
foreach ($sample in $m.budget_samples) {
    $key = $sample.key
    Check ($sample.requested -eq 19 -and ($sample.completed -join ',') -ceq ($expected[$key] -join ',')) ('Budget counts differ: ' + $key)
    $rows = [regex]::Matches($logs[$key], 'HC074-FRAME-BUDGET-SUMMARY sample=(\d) completed=(\d+) requested=19 publication_free=true admitted=(true|false)')
    Check ($rows.Count -eq 3) ('Budget summaries missing: ' + $key)
    for ($i = 0; $i -lt 3; $i++) {
        $n = $expected[$key][$i]
        Check ($rows[$i].Groups[1].Value -eq $i -and $rows[$i].Groups[2].Value -eq $n) ('Budget row differs: ' + $key)
        Check (($rows[$i].Groups[3].Value -ceq 'true') -eq ($n -eq 19)) ('Completion label differs: ' + $key)
        $checks = [regex]::Matches($logs[$key], ('HC074-FRAME-BUDGET sample=' + $i + ' check=(\d+) [^\r\n]* result=Ok\(\(\)\)'))
        Check ($checks.Count -eq $n) ('Successful check count differs: ' + $key)
        for ($j = 0; $j -lt $n; $j++) { Check ($checks[$j].Groups[1].Value -eq $j) ('Check order differs: ' + $key) }
    }
}
$f = $m.fault_matrix
Check ($f.key -ceq 'frame-fault-194' -and $f.unchanged_guard -ceq 'clock-order') 'Fault identity differs'
Check ($f.failed_during_next_frame_construction -is [bool] -and $f.failed_during_next_frame_construction) 'Missing retained failure'
foreach ($name in @('msrv_matrix_run','reference_controls_reached','concurrent_control_reached')) {
    Check ($f.$name -is [bool] -and -not $f.$name) ('Unsupported fault claim: ' + $name)
}
Check ($f.completed_cut_kind_pairs.Count -eq 5) 'Fault count differs'
for ($i=0; $i -lt 5; $i++) { Check (($f.completed_cut_kind_pairs[$i] -join ',') -ceq ('0,' + $i)) 'Fault pair differs' }
$drifts = [regex]::Matches($logs['frame-fault-194'], 'HC074-FRAME-DRIFT cut=(\d) kind=(\d) refused=true fence_retained=true')
Check ($drifts.Count -eq 5) 'Fault rows differ'
for ($i=0; $i -lt 5; $i++) { Check ($drifts[$i].Groups[1].Value -eq 0 -and $drifts[$i].Groups[2].Value -eq $i) 'Fault row differs' }
Check ($logs['frame-fault-194'].Contains('site: "clock-order"') -and $logs['frame-budget-release-194'].Contains('site: "clock-order"')) 'Missing clock refusals'
Check (-not $logs['frame-fault-194'].Contains('HC074-FRAME-REFERENCE') -and -not $logs['frame-fault-194'].Contains('HC074-FRAME-CONCURRENT')) 'Overstated fault completion'
Check ($logs['frame-static-final'].Contains('183 passed; 0 failed')) 'Static final is not green'
Check ($logs['frame-red-api'].Contains('no method named') -and $logs['frame-red-api'].Contains('into_validation_frame')) 'Missing red API'
foreach ($lane in @('194','188')) {
    Check ($logs[('frame-linux-' + $lane + '-docs')].Contains('6 passed; 0 failed')) 'Compile-fail gate differs'
    Check ($logs[('frame-linux-' + $lane + '-fast')].Contains('2 passed; 0 failed; 2 ignored')) 'Fast gate differs'
    foreach ($n in @(24,20,5,6)) {
        Check ($logs[('frame-linux-' + $lane + '-regression')].Contains(($n.ToString() + ' passed; 0 failed'))) 'Regression count differs'
    }
}
Check ($logs['frame-registry'].Contains('176 discovered gates, all registered')) 'Registry differs'
Check ($logs['frame-ship-guard'].Contains('ship admission is closed')) 'Ship guard missing'
Check ([regex]::Matches($logs['frame-clock-observation'], 'HC074-CLOCK ').Count -eq 75) 'Clock observation incomplete'
$hashes = @('65f901ca510c240e4beff482c19d58ac6279ab685ee592c072142f98e4a91590','c2e6ff7a751a7b6c0278f5d37910dba2c8128ea7356793a9af63373a7f013187')
Check ($m.release_binary_observations.Count -eq 2) 'Binary inventory differs'
for ($i=0; $i -lt 2; $i++) {
    Check ($m.release_binary_observations[$i].sha256 -ceq $hashes[$i] -and $logs['frame-release-binary-observations'].Contains($hashes[$i])) 'Observed binary differs'
}
'Validation frame evidence: intact; candidate UNCOMMITTED, live fault gate INCOMPLETE, server admission CLOSED'
