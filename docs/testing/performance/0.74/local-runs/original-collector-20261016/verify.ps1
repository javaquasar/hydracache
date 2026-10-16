$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Assert-True($ok, [string]$message) { if (-not $ok) { throw $message } }
Assert-True ($m.format_version -eq 1 -and $m.source_commit -ceq 'f76cd63eb719888f4e3ff53a81c0b17924be0cf6') 'Wrong source'
foreach ($field in @('root_constructor_added', 'durable_lifecycle_proved', 'original_start_proved', 'all_writer_cessation_proved', 'server_admitted', 'performance_claim', 'clean_tree_build_attested')) {
    Assert-True ($m.$field -eq $false) ('Scope must remain closed: ' + $field)
}
$paths = @(
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_intent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_collector.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_attempt_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_collector_tests.rs',
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_kernel_refusal.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_intent.rs',
    'crates/xtask/tests/support/performance_contract_074_original_collector.rs',
    'docs/testing/performance/0.74/diagnostic-original-collector-contract.toml',
    'docs/testing/performance/0.74/diagnostic-kernel-refusal-attribution-contract.toml',
    'docs/testing/performance/0.74/diagnostic-original-collector-design.md'
)
Assert-True ($m.source_files.Count -eq 16 -and @($m.source_files.path | Sort-Object -Unique).Count -eq 16) 'Wrong source inventory'
Push-Location $repo
try {
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.source_commit)
    Assert-True ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Source tree differs'
    foreach ($source in $m.source_files) {
        Assert-True ($paths -ccontains $source.path) 'Unexpected source'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit, $source.path)
        Assert-True ($LASTEXITCODE -eq 0 -and $blob -ceq $source.git_blob) 'Source blob differs'
        & git diff --exit-code $m.source_commit -- $source.path
        Assert-True ($LASTEXITCODE -eq 0) ('Current source differs: ' + $source.path)
    }
    $allowed = $paths + 'docs/testing/performance/0.74/diagnostic-original-collector-plan-20261016.md'
    foreach ($path in @(& git diff --name-only '0e37735f7761895071683ec7bead0fc5f37b7e07' $m.source_commit)) {
        Assert-True ($allowed -ccontains $path) ('Unreviewed mutation: ' + $path)
    }
} finally { Pop-Location }
$frozenPaths = @('Cargo.lock', 'docs/COMPAT.md', 'docs/testing/gated-test-registry.toml',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'docs/testing/performance/0.74/local-runs/collector-intent-20261016/manifest.json')
Assert-True ($m.frozen_files.Count -eq 5 -and @($m.frozen_files.path | Sort-Object -Unique).Count -eq 5) 'Wrong frozen inventory'
foreach ($file in $m.frozen_files) {
    Assert-True ($frozenPaths -ccontains $file.path) 'Unexpected frozen input'
    Assert-True ((Get-FileHash -LiteralPath (Join-Path $repo $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) ('Frozen input differs: ' + $file.path)
}
$names = @()
foreach ($artifact in $m.artifacts) {
    Assert-True ($artifact.file -cmatch '^[A-Za-z0-9][A-Za-z0-9.-]*$' -and $names -cnotcontains $artifact.file) 'Unsafe or duplicate artifact'
    $names += $artifact.file
    $file = Join-Path $packet $artifact.file
    Assert-True ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $artifact.sha256 -and (Get-Item -LiteralPath $file).Length -eq $artifact.bytes) ('Artifact differs: ' + $artifact.file)
}
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object Name -CNE 'manifest.json' | ForEach-Object Name | Sort-Object)
Assert-True (($actual -join "`n") -ceq (($names | Sort-Object) -join "`n")) 'Artifact inventory differs'
Assert-True ($m.captures.Count -eq 26 -and @($m.captures.file | Sort-Object -Unique).Count -eq 26) 'Wrong capture count'
foreach ($capture in $m.captures) {
    Assert-True ($names -ccontains $capture.file -and $capture.exit_code -eq $capture.expected_exit_code) 'Unexpected capture exit'
}
function Log([string]$name) { Get-Content -LiteralPath (Join-Path $packet ($name + '.log')) -Raw }
Assert-True ((Log 'baseline') -match '12 passed; 0 failed; 1 ignored') 'Missing baseline'
Assert-True ((Log 'missing-api') -match 'E0599' -and (Log 'missing-api') -match 'bind_collector_requirements') 'Missing compiler red'
Assert-True ((Log 'first-green') -match '7 passed; 0 failed' -and (Log 'first-green') -match 'signed_mutations=10') 'Missing intermediate proof'
foreach ($version in @('1.94.0', '1.88.0')) {
    $original = Log "original-$version"
    Assert-True ($original -match '76 passed; 0 failed; 1 ignored' -and $original -match 'seed=0x0740c098 signed_mutations=11 peer_time_assertions=false output_creation=false') 'Missing final original-input proof'
    Assert-True ($original -notmatch 'HC074-KERNEL-FIRST-REFUSAL') 'Unexpected original fixture refusal'
    Assert-True ((Log "bytes-$version") -match '30 passed; 0 failed') 'Missing byte-only regression'
    Assert-True ((Log "kernel-$version") -match '26 passed; 0 failed; 1 ignored') 'Missing kernel regression'
    Assert-True ((Log "doc-$version") -match '4 passed; 0 failed' -and (Log "doc-$version") -match 'compile fail ... ok') 'Missing actual compile-fail examples'
}
$matrix = Log 'matrix-current'
Assert-True ($matrix -match '1 passed; 0 failed; 0 ignored' -and $matrix -match 'sigkills=9 controls=1 fresh_inspections=20 production_enrollment=false start=false') 'Missing existing joined SIGKILL regression'
Assert-True ([regex]::Matches($matrix, 'controller_attempt_crash seed=0x0740c095 stage=[A-Za-z]+ sigkill=(true|false) fresh_inspection=[12] empty=(true|false) unchanged=true').Count -eq 20) 'Wrong fresh inspection inventory'
Assert-True ((Log 'contracts') -match '171 passed; 0 failed') 'Missing static suite'
foreach ($platform in @('linux', 'windows')) {
    foreach ($version in @('1.94.0', '1.88.0')) {
        foreach ($kind in @('check', 'clippy')) {
            $file = "$platform-$version-$kind.log"
            $entry = @($m.captures | Where-Object file -CEQ $file)
            Assert-True ($entry.Count -eq 1 -and $entry[0].exit_code -eq 0) ('Missing final lane: ' + $file)
        }
    }
}
foreach ($name in @('fmt', 'registry', 'doc-check', 'performance-contract')) {
    $entry = @($m.captures | Where-Object file -CEQ ($name + '.log'))
    Assert-True ($entry.Count -eq 1 -and $entry[0].exit_code -eq 0) ('Missing gate: ' + $name)
}
Assert-True ((Log 'registry') -match '176 discovered gates, all registered') 'Registry differs'
Assert-True ((Log 'require-ship') -match 'ship admission is closed while candidate identity and release qualification are incomplete') 'Closed admission missing'
Write-Output 'Verified 26 captures, source/frozen/artifact hashes; both 76-test original-input, 30-test byte and 26-test kernel groups, four compile-fail examples per compiler, 171 contracts and eight build/lint lanes.'
Write-Output 'Verified unchanged writer SIGKILL regression: nine kills, one control, twenty inspections. Unsigned consistency only; no durable lifecycle, root constructor, original-start or writer-cessation admission.'
