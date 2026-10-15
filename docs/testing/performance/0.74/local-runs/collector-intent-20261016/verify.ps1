$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Assert-True($ok, [string]$message) { if (-not $ok) { throw $message } }
Assert-True ($m.format_version -eq 1 -and $m.source_commit -ceq '9303bb7e3746fdeddaf2171515a66718ddc9fc15') 'Wrong source'
foreach ($field in @('server_admitted', 'performance_claim', 'original_start_proved', 'all_writer_cessation_proved', 'clean_tree_build_attested')) {
    Assert-True ($m.$field -eq $false) ('Scope must remain closed: ' + $field)
}
$paths = @(
    'tools/long-run-supervisor-074/src/diagnostic_intent_projection.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_intent.rs',
    'tools/long-run-supervisor-074/tests/diagnostic_intent_projection.rs',
    'tools/long-run-supervisor-074/tests/support/diagnostic_collector_intent.rs',
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_intent.rs',
    'docs/testing/performance/0.74/diagnostic-collector-intent-contract.toml',
    'docs/COMPAT.md'
)
Assert-True ($m.source_files.Count -eq 8 -and @($m.source_files.path | Sort-Object -Unique).Count -eq 8) 'Wrong source inventory'
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
    $allowed = $paths + 'docs/testing/performance/0.74/diagnostic-collector-intent-design.md'
    foreach ($path in @(& git diff --name-only '46f721e9ad98d5404f3682819b11be550c36f566' $m.source_commit)) {
        Assert-True ($allowed -ccontains $path) ('Unreviewed source mutation: ' + $path)
    }
} finally { Pop-Location }
$frozenPaths = @('Cargo.lock', 'docs/testing/gated-test-registry.toml',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'docs/testing/performance/0.74/local-runs/collector-intent-prerequisite-20261016/manifest.json',
    'docs/testing/performance/0.74/local-runs/kernel-refusal-attribution-20261016/manifest.json')
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
Assert-True ((Log 'baseline') -match '16 passed; 0 failed') 'Missing baseline'
Assert-True ((Log 'missing-api') -match 'E0432' -and (Log 'missing-api') -match 'could not find `collector`') 'Missing actual compiler red'
Assert-True ((Log 'first-green') -match '29 passed; 0 failed') 'Missing intermediate first green'
foreach ($name in @('repeat-1', 'repeat-2', 'repeat-3', 'repeat-msrv')) {
    $raw = Log $name
    Assert-True ($raw -match '30 passed; 0 failed' -and $raw -match 'fields=10 order_mutations=2 durable_replay_prevention=false' -and $raw -match 'seed=0x0740c097 state_mutations=64 independent_threads=4 production_start=false') 'Missing final repeated proof'
    Assert-True ($raw -notmatch 'corrupt incremental|WSL.*ERROR') 'Unexpected final environment warning'
}
foreach ($version in @('current', 'msrv')) {
    Assert-True ((Log "typed-$version") -match '2 passed; 0 failed') 'Missing typed projection regression'
    Assert-True ((Log "doc-$version") -match '5 passed; 0 failed' -and (Log "doc-$version") -match 'compile fail ... ok') 'Missing executed compile-fail examples'
}
Assert-True ((Log 'contracts') -match '170 passed; 0 failed') 'Missing static suite'
Assert-True ((Log 'static-focused') -match 'legacy_unchanged=true requirements_only=true execution=false') 'Missing independent legacy boundary'
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
Assert-True ((Log 'require-ship') -match 'ship admission is closed while candidate identity and release qualification are incomplete') 'Closed release admission missing'
Write-Output 'Verified 26 captures and packet hashes; four 30-test repetitions, two typed and five compile-fail tests on each compiler, 170 static contracts and eight build/lint lanes.'
Write-Output 'Verified historical/current source and frozen inputs. Unsigned consistency only: signed requirements do not prove original start, writer cessation or server admission.'
