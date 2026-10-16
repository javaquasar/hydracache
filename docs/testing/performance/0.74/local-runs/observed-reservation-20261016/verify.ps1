$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Assert-True($ok, [string]$message) { if (-not $ok) { throw $message } }
Assert-True ($m.format_version -eq 1 -and $m.source_commit -ceq '82312a02fc9ebc9a45360a7fb04d577c31d58468' -and $m.baseline_commit -ceq '0389978f1ff5d7eeacb99c497f9ef8cbfd6be08a') 'Wrong source'
foreach ($field in @('durable_lifecycle_proved', 'root_constructor_added', 'original_start_proved', 'all_writer_cessation_proved', 'server_admitted', 'performance_claim', 'clean_tree_build_attested')) {
    Assert-True ($m.$field -eq $false) ('Scope must remain closed: ' + $field)
}
$paths = @(
    'tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_collector.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_collector_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_observed_reservation.rs',
    'tools/long-run-supervisor-074/src/diagnostic_observed_reservation_tests.rs',
    'crates/xtask/tests/support/performance_contract_074_original_collector.rs',
    'docs/testing/performance/0.74/diagnostic-observed-reservation-contract.toml',
    'docs/testing/performance/0.74/diagnostic-original-collector-design.md'
)
$frozenPaths = @(
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs',
    'tools/long-run-supervisor-074/src/diagnostic_early_attempt_storage.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_intent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_intent_projection.rs',
    'tools/long-run-supervisor-074/src/diagnostic_unit.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector.rs',
    'tools/long-run-supervisor-074/src/diagnostic_starting_intent.rs',
    'Cargo.lock',
    'docs/COMPAT.md',
    'docs/testing/gated-test-registry.toml',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'docs/testing/performance/0.74/local-runs/original-collector-20261016/manifest.json'
)
Assert-True ($m.source_files.Count -eq 9 -and @($m.source_files.path | Sort-Object -Unique).Count -eq 9) 'Wrong source inventory'
Assert-True ($m.frozen_files.Count -eq 13 -and @($m.frozen_files.path | Sort-Object -Unique).Count -eq 13) 'Wrong frozen inventory'
Push-Location $repo
try {
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.source_commit)
    Assert-True ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Source tree differs'
    & git merge-base --is-ancestor $m.baseline_commit $m.source_commit
    Assert-True ($LASTEXITCODE -eq 0) 'Wrong source ancestry'
    foreach ($source in $m.source_files) {
        Assert-True ($paths -ccontains $source.path) 'Unexpected source'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit, $source.path)
        Assert-True ($LASTEXITCODE -eq 0 -and $blob -ceq $source.git_blob) 'Source blob differs'
        & git diff --exit-code $m.source_commit -- $source.path
        Assert-True ($LASTEXITCODE -eq 0) ('Current source differs: ' + $source.path)
    }
    $allowed = $paths + 'docs/testing/performance/0.74/diagnostic-observed-reservation-plan-20261016.md'
    $changed = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Assert-True ($LASTEXITCODE -eq 0 -and $changed.Count -eq 10) 'Wrong changed inventory'
    foreach ($path in $changed) { Assert-True ($allowed -ccontains $path) ('Out-of-scope source change: ' + $path) }
    foreach ($file in $m.frozen_files) {
        Assert-True ($frozenPaths -ccontains $file.path) 'Unexpected frozen input'
        $blob = & git rev-parse ('{0}:{1}' -f $m.baseline_commit, $file.path)
        Assert-True ($LASTEXITCODE -eq 0 -and $blob -ceq $file.git_blob) 'Frozen baseline differs'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit, $file.path)
        Assert-True ($LASTEXITCODE -eq 0 -and $blob -ceq $file.git_blob) 'Frozen source differs'
        Assert-True ((Get-FileHash -LiteralPath $file.path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) ('Frozen bytes differ: ' + $file.path)
    }
} finally { Pop-Location }
$names = @()
foreach ($artifact in $m.artifacts) {
    Assert-True ($artifact.file -cmatch '^[A-Za-z0-9][A-Za-z0-9.-]*$' -and $names -cnotcontains $artifact.file) 'Unsafe or duplicate artifact'
    $names += $artifact.file
    $file = Join-Path $packet $artifact.file
    Assert-True ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $artifact.sha256 -and (Get-Item -LiteralPath $file).Length -eq $artifact.bytes) ('Artifact differs: ' + $artifact.file)
}
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object Name -CNE 'manifest.json' | ForEach-Object Name | Sort-Object)
Assert-True (($actual -join "`n") -ceq (($names | Sort-Object) -join "`n")) 'Artifact inventory differs'
Assert-True ($m.captures.Count -eq 31 -and @($m.captures.file | Sort-Object -Unique).Count -eq 31 -and $m.artifacts.Count -eq 33) 'Wrong capture count'
foreach ($capture in $m.captures) {
    Assert-True ($names -ccontains $capture.file -and $capture.exit_code -eq $capture.expected_exit_code) 'Unexpected capture exit'
}
function Log([string]$name) { Get-Content -LiteralPath (Join-Path $packet ($name + '.log')) -Raw }
Assert-True ((Log 'baseline') -match '8 passed; 0 failed') 'Missing baseline'
Assert-True ((Log 'red-clean') -match 'E0599' -and (Log 'red-clean') -match 'into_observed_reservation' -and (Log 'red-clean') -notmatch 'MetadataExt') 'Missing clean compiler red'
Assert-True ((Log 'first-green') -match '6 passed; 0 failed' -and (Log 'first-green') -match 'signed_mutations=6') 'Missing first green'
Assert-True ((Log 'contracts-first') -match '171 passed; 1 failed' -and (Log 'contracts-first') -match 'self.state.as_ref') 'Missing formatting-sensitive static failure'
Assert-True ((Log 'contract-focused') -match '1 passed; 0 failed') 'Missing corrected static gate'
Assert-True ((Log 'contracts') -match '172 passed; 0 failed') 'Missing static suite'
foreach ($version in @('1.94.0', '1.88.0')) {
    $original = Log "original-$version"
    Assert-True ($original -match '82 passed; 0 failed; 1 ignored' -and $original -match 'seed=0x0740c099 signed_mutations=6 caller_state=false durable_ack=false start=false') 'Missing final original chain proof'
    Assert-True ($original -notmatch 'HC074-KERNEL-FIRST-REFUSAL') 'Unexpected original kernel refusal'
    Assert-True ((Log "bytes-$version") -match '30 passed; 0 failed') 'Missing byte regression'
    Assert-True ((Log "kernel-$version") -match '26 passed; 0 failed; 1 ignored') 'Missing kernel regression'
    Assert-True ((Log "doc-$version") -match '10 passed; 0 failed' -and (Log "doc-$version") -match 'compile fail ... ok') 'Missing compile-fail proof'
}
$matrix = Log 'matrix-current'
Assert-True ($matrix -match '1 passed; 0 failed' -and $matrix -match 'sigkills=9 controls=1 fresh_inspections=20 production_enrollment=false start=false') 'Missing unchanged writer process proof'
Assert-True ([regex]::Matches($matrix, 'controller_attempt_crash seed=0x0740c095 stage=[A-Za-z]+ sigkill=(true|false) fresh_inspection=[12] empty=(true|false) unchanged=true').Count -eq 20) 'Wrong process inspection count'
foreach ($platform in @('linux', 'windows')) {
    foreach ($version in @('1.94.0', '1.88.0')) {
        foreach ($kind in @('check', 'clippy')) {
            $entry = @($m.captures | Where-Object file -CEQ "$platform-$version-$kind.log")
            Assert-True ($entry.Count -eq 1 -and $entry[0].exit_code -eq 0) 'Missing final build/lint lane'
        }
    }
}
Assert-True ((Log 'registry') -match '176 discovered gates, all registered') 'Registry differs'
Assert-True ((Log 'require-ship') -match 'ship admission is closed while candidate identity and release qualification are incomplete') 'Missing closed admission'
Write-Output 'Verified 31 captures, nine source objects, thirteen unchanged inputs and all artifact hashes.'
Write-Output 'Both compilers: 82 original-input, 30 byte, 26 kernel tests and ten compile-fail examples; 172 static contracts and eight build/lint lanes.'
Write-Output 'Existing writer regression: nine SIGKILLs, one control, twenty inspections. Unsigned local consistency only; no durable Reserved/Starting, root construction or server admission.'
