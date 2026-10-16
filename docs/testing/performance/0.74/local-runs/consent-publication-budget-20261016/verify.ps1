param([string]$ManifestPath = (Join-Path $PSScriptRoot 'manifest.json'))
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../../../..')).Path
function Check($ok, [string]$message) { if (-not $ok) { throw $message } }
function Same($a, $b, [string]$message) {
    Check ((@($a | Sort-Object) -join '|') -ceq (@($b | Sort-Object) -join '|')) $message
}
$m = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
Check ($m.format_version -eq 1) 'Unknown manifest'
Check ($m.source_commit -ceq '17d3ebdada7ebffd9d99e04960651b6adf3d2dc1') 'Source differs'
Check ($m.source_tree -ceq 'c44cf0f13e9830e14ce5122a2a4d59e8b0e3971f') 'Tree differs'
Check ($m.parent_commit -ceq 'd560e969aaf3466ea239e7c16de4e339e23fe38f') 'Parent differs'
Check ($m.baseline_commit -ceq '4fbc4f3b3574afe955684bb88c8b43ecfd662c03') 'Baseline differs'
Check ($m.preregistrations.initial_budget -ceq 'fcdb2b47' -and $m.preregistrations.release_control -ceq '8fb8589f' -and $m.preregistrations.owner_attribution -ceq 'd560e969') 'Preregistration differs'
$falseScope = @('publication_budget_pass','durable_starting_written','validation_composition_implemented','new_sigkill_matrix_run','authenticated_process_started','all_writers_revoked','server_mutated','product_workload_run','qualification_changed','deadline_changed','binary_attested','full_release_verify_run')
Same @($m.scope.PSObject.Properties.Name) $falseScope 'Scope inventory differs'
foreach ($key in $falseScope) { Check ($m.scope.$key -is [bool] -and -not $m.scope.$key) ('Scope differs: ' + $key) }
$sourcePaths = @(
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_original_start_consent.rs',
    'crates/xtask/tests/support/performance_contract_074_consent_budget.rs',
    'docs/testing/gated-test-registry.toml',
    'tools/long-run-supervisor-074/src/diagnostic_original_start_consent_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_consent_budget_tests.rs'
)
$frozenPaths = @(
    'Cargo.toml',
    'Cargo.lock',
    'tools/long-run-supervisor-074/Cargo.toml',
    'docs/COMPAT.md',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'tools/long-run-supervisor-074/src/diagnostic_ipc.rs',
    'tools/long-run-supervisor-074/src/diagnostic_lease.rs',
    'tools/long-run-supervisor-074/src/diagnostic_transition.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs',
    'tools/long-run-supervisor-074/src/host_execution.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_start_consent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_start_consent.rs'
)
Same @($m.selected_sources.path) $sourcePaths 'Selected source inventory differs'
Same @($m.frozen_inputs.path) $frozenPaths 'Frozen input inventory differs'
Push-Location $repo
try {
    $tree = & git rev-parse ($m.source_commit + '^{tree}')
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Git tree differs'
    $parent = & git rev-parse ($m.source_commit + '^')
    Check ($LASTEXITCODE -eq 0 -and $parent -ceq $m.parent_commit) 'Git parent differs'
    foreach ($preregistration in @('fcdb2b47','8fb8589f','d560e969')) {
        & git merge-base --is-ancestor $preregistration $m.source_commit
        Check ($LASTEXITCODE -eq 0) ('Preregistration ancestry differs: ' + $preregistration)
    }
    $delta = @(& git diff --name-only $m.parent_commit $m.source_commit)
    Check ($LASTEXITCODE -eq 0) 'Cannot read source delta'
    Same $delta $sourcePaths 'Source delta differs'
    foreach ($file in @($m.selected_sources) + @($m.frozen_inputs)) {
        $blob = & git rev-parse ($m.source_commit + ':' + $file.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $file.blob) ('Git source differs: ' + $file.path)
        Check ((Get-FileHash -LiteralPath (Join-Path $repo $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) ('Working bytes differ: ' + $file.path)
        if ($frozenPaths -ccontains $file.path) {
            $baseline = & git rev-parse ($m.baseline_commit + ':' + $file.path)
            Check ($LASTEXITCODE -eq 0 -and $baseline -ceq $file.blob) ('Frozen input changed: ' + $file.path)
        }
    }
} finally { Pop-Location }
$keys = @(
    'budget-194',
    'budget-188',
    'budget-release-194',
    'budget-release-188-build',
    'budget-release-188',
    'budget-owner-194',
    'budget-owner-188',
    'budget-final-win-194-check',
    'budget-final-win-194-clippy',
    'budget-final-win-188-check',
    'budget-final-win-188-clippy',
    'budget-final-linux-194-check',
    'budget-final-linux-194-clippy',
    'budget-final-linux-188-check',
    'budget-final-linux-188-clippy',
    'budget-static-final',
    'budget-doc-check',
    'budget-registry',
    'budget-local-contract',
    'budget-ship-guard',
    'budget-fmt',
    'budget-registry-postcommit',
    'budget-doc-check-postcommit'
)
Same @($m.commands.key) $keys 'Command inventory differs'
Same @($m.artifacts.path) @($keys | ForEach-Object { 'logs/' + $_ + '.json' }) 'Artifact inventory differs'
Check ($m.commands.Count -eq $keys.Count -and $m.artifacts.Count -eq $keys.Count) 'Duplicate entries'
$logs = @{}
foreach ($cmd in $m.commands) {
    $path = 'logs/' + $cmd.key + '.json'
    $artifact = @($m.artifacts | Where-Object path -CEQ $path)
    Check ($artifact.Count -eq 1) ('Missing artifact: ' + $cmd.key)
    $file = Join-Path $PSScriptRoot $path
    Check ((Get-Item -LiteralPath $file).Length -eq $artifact[0].bytes) ('Length differs: ' + $path)
    Check ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $artifact[0].sha256) ('Hash differs: ' + $path)
    $capture = Get-Content -LiteralPath $file -Raw | ConvertFrom-Json
    $expectedExit = if ($cmd.key -ceq 'budget-ship-guard') { 1 } else { 0 }
    Check ($cmd.exit_code -eq $expectedExit -and $capture.exit_code -eq $expectedExit) ('Command outcome differs: ' + $cmd.key)
    Check ($cmd.command -ceq $capture.command -and -not [string]::IsNullOrEmpty($cmd.command)) ('Command differs: ' + $cmd.key)
    $logs[$cmd.key] = $capture.output
}
$expected = @{
    'budget-194' = @(0,1,1)
    'budget-188' = @(1,1,1)
    'budget-release-194' = @(4,5,5)
    'budget-release-188' = @(4,5,5)
}
Same @($m.budget_samples.key) @($expected.Keys) 'Budget cohort inventory differs'
foreach ($key in $expected.Keys) {
    $rows = [regex]::Matches($logs[$key], 'HC074-CONSENT-BUDGET-SUMMARY sample=(\d+) completed=(\d+) requested=19 publication_free=true admitted=false')
    Check ($rows.Count -eq 3) ('Incomplete budget samples: ' + $key)
    $entry = @($m.budget_samples | Where-Object key -CEQ $key)
    Check ($entry.Count -eq 1) ('Duplicate budget cohort: ' + $key)
    Check ((@($entry[0].completed) -join ',') -ceq (@($expected[$key]) -join ',')) ('Manifest sample counts differ: ' + $key)
    for ($i = 0; $i -lt 3; $i++) {
        Check ([int]$rows[$i].Groups[1].Value -eq $i -and [int]$rows[$i].Groups[2].Value -eq $expected[$key][$i]) ('Sample differs: ' + $key)
        $checks = [regex]::Matches($logs[$key], ('HC074-CONSENT-BUDGET sample=' + $i + ' check=(\d+) elapsed_ns=(\d+) reserved_age_ns=(\d+) heartbeat_age_ns=(\d+) result=([^\r\n]+)'))
        Check ($checks.Count -eq (1 + $expected[$key][$i])) ('Missing check rows: ' + $key)
        for ($j = 0; $j -lt $checks.Count; $j++) {
            Check ([int]$checks[$j].Groups[1].Value -eq $j) ('Check order differs: ' + $key)
            $result = $checks[$j].Groups[5].Value
            Check ($(if ($j -lt $expected[$key][$i]) { $result -ceq 'Ok(())' } else { $result.StartsWith('Err(') })) ('Check result differs: ' + $key)
        }
    }
    Check ($logs[$key].Contains('1 passed; 0 failed')) ('Budget fixture did not complete: ' + $key)
}
$counts = @{'kernel-worker'=0; 'signed-controller'=1; 'reviewed-build'=2; 'oidc-controller'=4}
foreach ($key in @('budget-owner-194','budget-owner-188')) {
    $rows = [regex]::Matches($logs[$key], 'HC074-CONSENT-OWNER sample=(\d+) owner=([a-z-]+) repetition=(\d+) inclusive_wall_ns=(\d+) user_us=(\d+) system_us=(\d+) ipc_authentication_calls=(\d+) ipc_signature_calls=(\d+) ipc_signature_ns=(\d+)')
    Check ($rows.Count -eq 36) ('Incomplete owner samples: ' + $key)
    $seen = @{}
    foreach ($row in $rows) {
        $sample = [int]$row.Groups[1].Value
        $owner = $row.Groups[2].Value
        $repetition = [int]$row.Groups[3].Value
        $id = "$sample|$owner|$repetition"
        Check ($sample -lt 3 -and $repetition -lt 3 -and $counts.ContainsKey($owner) -and -not $seen.ContainsKey($id)) ('Owner inventory differs: ' + $key)
        $seen[$id] = $true
        Check ([long]$row.Groups[4].Value -gt 0 -and [int]$row.Groups[7].Value -eq $counts[$owner] -and [int]$row.Groups[8].Value -eq $counts[$owner]) ('Owner counters differ: ' + $key)
    }
    Check ($logs[$key].Contains('1 passed; 0 failed')) ('Owner fixture did not complete: ' + $key)
}
Check ($logs['budget-static-final'].Contains('182 passed; 0 failed')) 'Static contracts incomplete'
Check ($logs['budget-registry-postcommit'].Contains('all registered')) 'Registration incomplete'
Check ($logs['budget-doc-check-postcommit'].Contains('doc-check: OK')) 'Documentation failed'
Check ($logs['budget-local-contract'].Contains('local, non-promotable')) 'Local contract differs'
Check ($logs['budget-ship-guard'].Contains('0.74 ship admission is closed')) 'Ship refusal differs'
'Consent budget evidence: OK; publication budget FAILED; server admission CLOSED'
