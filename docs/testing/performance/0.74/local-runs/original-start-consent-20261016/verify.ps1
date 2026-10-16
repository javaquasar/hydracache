param([string]$ManifestPath = (Join-Path $PSScriptRoot 'manifest.json'))
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../../../..')).Path
function Check($ok, [string]$message) { if (-not $ok) { throw $message } }
function SamePaths($actual, $expected, [string]$message) {
    Check ((@($actual | Sort-Object) -join "\n") -ceq (@($expected | Sort-Object) -join "\n")) $message
}
$m = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
Check ($m.format_version -eq 1) 'Unknown format'
Check ($m.source_commit -ceq '4fbc4f3b3574afe955684bb88c8b43ecfd662c03') 'Source differs'
Check ($m.source_tree -ceq 'f120d957e9c2ed6f52925572c6f84fd0bb35ca2f') 'Tree differs'
Check ($m.parent_commit -ceq '2c2d4fbbf2c488639144a9fbbc1acdfd72b54106') 'Parent differs'
Check ($m.baseline_commit -ceq '287cd72a08ac8c8c381f13500ec4432669bc80c5') 'Baseline differs'
$trueScope = @('kernel_peer_and_time_observed','same_original_receiver_and_owners','canonical_frozen_body_checked','old_guard_predicates_preserved')
$falseScope = @('production_trust_enrolled','durable_replay_protection','durable_starting_published','process_started','all_writer_cessation_proved','server_admitted','product_workload_run','qualification_changed','full_release_verify_run','ordinary_original_group_run','new_sigkill_matrix_run','default_debug_joined_pass','deadline_changed','binary_attested')
SamePaths @($m.scope.PSObject.Properties.Name) @($trueScope + $falseScope) 'Scope inventory differs'
foreach ($key in $trueScope) { Check ($m.scope.$key -is [bool] -and $m.scope.$key) ('Scope differs: ' + $key) }
foreach ($key in $falseScope) { Check ($m.scope.$key -is [bool] -and -not $m.scope.$key) ('Scope differs: ' + $key) }
$sourcePaths = @(
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_intent.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_progress.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_reserved.rs',
    'crates/xtask/tests/support/performance_contract_074_start_consent.rs',
    'crates/xtask/tests/support/performance_contract_074_original_start_consent.rs',
    'docs/testing/gated-test-registry.toml',
    'tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_progress_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_start_consent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_start_consent_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_start_consent_binding_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_start_consent_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_start_consent.rs'
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
    'tools/long-run-supervisor-074/src/host_execution.rs'
)
SamePaths @($m.selected_sources.path) $sourcePaths 'Source inventory differs'
SamePaths @($m.frozen_inputs.path) $frozenPaths 'Frozen inventory differs'
Push-Location $repo
try {
    $tree = & git rev-parse ($m.source_commit + '^{tree}')
    Check ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Git tree differs'
    $parent = & git rev-parse ($m.source_commit + '^')
    Check ($LASTEXITCODE -eq 0 -and $parent -ceq $m.parent_commit) 'Git parent differs'
    & git merge-base --is-ancestor $m.baseline_commit $m.parent_commit
    Check ($LASTEXITCODE -eq 0) 'Baseline ancestry differs'
    $delta = @(& git diff --name-only $m.parent_commit $m.source_commit)
    Check ($LASTEXITCODE -eq 0) 'Cannot inspect source delta'
    SamePaths $delta $sourcePaths 'Source delta differs'
    foreach ($file in @($m.selected_sources) + @($m.frozen_inputs)) {
        $blob = & git rev-parse ($m.source_commit + ':' + $file.path)
        Check ($LASTEXITCODE -eq 0 -and $blob -ceq $file.blob) ('Blob differs: ' + $file.path)
        Check ((Get-FileHash -LiteralPath (Join-Path $repo $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) ('Working bytes differ: ' + $file.path)
        if ($frozenPaths -ccontains $file.path) {
            $old = & git rev-parse ($m.baseline_commit + ':' + $file.path)
            Check ($LASTEXITCODE -eq 0 -and $old -ceq $file.blob) ('Frozen input differs: ' + $file.path)
        }
    }
} finally { Pop-Location }
$keys = @(
    'linux-194-check-final',
    'linux-188-check-final',
    'win-194-check',
    'win-188-check',
    'linux-194-clippy',
    'linux-188-clippy',
    'win-194-clippy',
    'win-188-clippy',
    'linux-194-test-fast',
    'linux-188-fast',
    'linux-194-doc',
    'linux-188-doc',
    'win-194-codec',
    'win-188-codec',
    'linux-194-codec',
    'linux-188-codec',
    'win-194-portable-doc',
    'win-188-portable-doc',
    'linux-194-integration',
    'linux-188-integration',
    'win-static',
    'fmt-check',
    'registry',
    'doc-check',
    'local-contract',
    'require-ship',
    'linux-194-live',
    'linux-188-live',
    'native-build-cli'
)
SamePaths @($m.commands.key) $keys 'Command inventory differs'
SamePaths @($m.artifacts.path) @(@($keys | ForEach-Object { $_ + '.log' }) + @('README.md','verify.ps1','verification-results.json')) 'Artifact inventory differs'
foreach ($command in $m.commands) {
    $wanted = if ($command.key -ceq 'require-ship') { 1 } else { 0 }
    Check ($command.exit_code -eq $wanted -and $command.truncated -is [bool] -and -not $command.truncated) ('Command outcome differs: ' + $command.key)
    Check ($command.log -ceq ($command.key + '.log') -and $command.command.Length -gt 0) 'Command binding differs'
    $log = Get-Content -LiteralPath (Join-Path $PSScriptRoot $command.log) -Raw
    switch -Regex ($command.key) {
        '^linux-.*-(test-fast|fast)$' { Check ($log.Contains('4 passed; 0 failed; 1 ignored')) 'Fast cases missing' }
        '^linux-.*-live$' {
            foreach ($case in 0..3) { Check ($log.Contains("live_case=$case original_fence=true durable_starting=false process_start=false")) 'Live case missing' }
            Check ($log.Contains('1 passed; 0 failed')) 'Live result differs'
            foreach ($flag in @('--config profile.dev.package.ed25519-dalek.opt-level=3','--config profile.dev.package.curve25519-dalek.opt-level=3')) {
                Check ($command.command.Contains($flag)) 'Crypto build lane differs'
            }
        }
        '^linux-.*-doc$' { Check ($log.Contains('7 passed; 0 failed')) 'New API checks missing' }
        '-codec$' { Check ($log.Contains('7 passed; 0 failed')) 'Portable codec checks missing' }
        '-portable-doc$' { Check ($log.Contains('4 passed; 0 failed')) 'Portable API checks missing' }
        '-integration$' { foreach ($count in 24,20,5,6) { Check ($log.Contains("$count passed; 0 failed")) 'Old integrations missing' } }
        '^win-static$' { Check ($log.Contains('181 passed; 0 failed')) 'Static contracts missing' }
        '^require-ship$' { Check ($log.Contains('ship admission is closed')) 'Expected closed ship result missing' }
    }
}
foreach ($file in $m.artifacts) {
    Check ($file.path -cmatch '^[a-zA-Z0-9][a-zA-Z0-9.-]+$') 'Unsafe artifact name'
    $path = Join-Path $PSScriptRoot $file.path
    Check ((Get-Item -LiteralPath $path).Length -eq $file.bytes) ('Artifact length differs: ' + $file.path)
    Check ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) ('Artifact digest differs: ' + $file.path)
}
Write-Output 'original-start-consent evidence: OK; durable/process/server admission remains closed'
