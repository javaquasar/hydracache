$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Assert-True($condition, [string]$message) {
    if (-not $condition) { throw $message }
}
Assert-True ($m.format_version -eq 1) 'Unknown packet version'
Assert-True ($m.baseline_commit -ceq '2616276735c16e527640cab98350c1815b7bd1c6') 'Wrong baseline'
Assert-True ($m.source_commit -cmatch '^[a-f0-9]{40}$') 'Unsafe source identity'
Assert-True ($m.joined_local_sigkill_proved -and -not $m.server_admitted -and -not $m.original_start -and -not $m.performance_claim) 'Scope differs'
Assert-True ($m.kernel_invalid_root_cause_established -eq $false) 'Original-input negative must not be retroactively explained'
$expectedSources = @(
    'tools/long-run-supervisor-074/src/diagnostic_controller_attempt_crash_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_attempt_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_fence_preflight_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_trust_input_tests.rs',
    'crates/xtask/tests/performance_contract_074.rs',
    'docs/testing/gated-test-registry.toml',
    'docs/GATES.md',
    'docs/testing/performance/0.74/diagnostic-controller-attempt-contract.toml',
    'docs/testing/performance/0.74/diagnostic-controller-attempt-crash-contract.toml'
)
Assert-True ($m.source_files.Count -eq $expectedSources.Count) 'Wrong source inventory'
Assert-True (@($m.source_files.path | Sort-Object -Unique).Count -eq $expectedSources.Count) 'Duplicate source'
Push-Location $repo
try {
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.source_commit)
    Assert-True ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Source tree differs'
    foreach ($source in $m.source_files) {
        Assert-True ($expectedSources -ccontains $source.path) 'Unexpected source'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit, $source.path)
        Assert-True ($LASTEXITCODE -eq 0 -and $blob -ceq $source.git_blob) ('Source blob differs: ' + $source.path)
    }
    $allowed = $expectedSources + 'docs/testing/performance/0.74/diagnostic-controller-attempt-crash-plan-20261016.md'
    $changed = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Assert-True ($LASTEXITCODE -eq 0) 'Cannot inspect historical source scope'
    foreach ($path in $changed) { Assert-True ($allowed -ccontains $path) ('Unreviewed source change: ' + $path) }
    foreach ($path in @(
        'tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs',
        'tools/long-run-supervisor-074/src/diagnostic_early_attempt_storage.rs',
        'tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs',
        'tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs',
        'tools/long-run-supervisor-074/src/server.rs', 'docs/COMPAT.md', 'Cargo.lock'
    )) {
        $oldBlob = & git rev-parse ('{0}:{1}' -f $m.baseline_commit, $path)
        Assert-True ($LASTEXITCODE -eq 0) 'Cannot inspect baseline runtime'
        $newBlob = & git rev-parse ('{0}:{1}' -f $m.source_commit, $path)
        Assert-True ($LASTEXITCODE -eq 0 -and $oldBlob -ceq $newBlob) ('Runtime changed: ' + $path)
    }
    $registryDiff = @(& git diff --unified=0 $m.baseline_commit $m.source_commit -- docs/testing/gated-test-registry.toml)
    Assert-True (@($registryDiff | Where-Object { $_ -cmatch '^-[^-]' }).Count -eq 0) 'Existing registry entries changed'
    $oldRegistry = (& git show ($m.baseline_commit + ':docs/testing/gated-test-registry.toml')) -join "`n"
    $newRegistry = (& git show ($m.source_commit + ':docs/testing/gated-test-registry.toml')) -join "`n"
    Assert-True ([regex]::Matches($oldRegistry, '(?m)^\[\[gate\]\]$').Count -eq 229) 'Wrong baseline explicit registry'
    Assert-True ([regex]::Matches($newRegistry, '(?m)^\[\[gate\]\]$').Count -eq 230) 'Wrong extended explicit registry'
} finally { Pop-Location }
$frozenPaths = @('Cargo.lock', 'docs/testing/performance/0.74/qualification-manifest.toml', 'docs/testing/performance/0.74/local-runs/controller-attempt-20261015/manifest.json')
Assert-True ($m.frozen_files.Count -eq 3 -and @($m.frozen_files.path | Sort-Object -Unique).Count -eq 3) 'Wrong frozen inventory'
foreach ($frozen in $m.frozen_files) {
    Assert-True ($frozenPaths -ccontains $frozen.path) 'Unexpected frozen file'
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $repo $frozen.path)).Hash.ToLowerInvariant()
    Assert-True ($hash -ceq $frozen.sha256) ('Frozen input differs: ' + $frozen.path)
}
$names = @()
foreach ($artifact in $m.artifacts) {
    Assert-True ($artifact.file -cmatch '^[A-Za-z0-9][A-Za-z0-9.-]*$' -and $names -cnotcontains $artifact.file) 'Unsafe or duplicate artifact'
    $names += $artifact.file
    $file = Join-Path $packet $artifact.file
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLowerInvariant()
    Assert-True ($hash -ceq $artifact.sha256 -and (Get-Item -LiteralPath $file).Length -eq $artifact.bytes) ('Artifact differs: ' + $artifact.file)
}
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object Name -CNE 'manifest.json' | ForEach-Object Name | Sort-Object)
Assert-True (($actual -join "`n") -ceq (($names | Sort-Object) -join "`n")) 'Artifact inventory differs'
Assert-True (@($m.captures.file | Sort-Object -Unique).Count -eq $m.captures.Count) 'Duplicate capture'
foreach ($capture in $m.captures) {
    Assert-True ($names -ccontains $capture.file -and $capture.exit_code -eq $capture.expected_exit_code) ('Unexpected capture: ' + $capture.file)
}
function Log([string]$name) { Get-Content -LiteralPath (Join-Path $packet ($name + '.log')) -Raw }
Assert-True ((Log 'baseline') -match '10 passed; 0 failed') 'Missing baseline'
Assert-True ((Log 'missing-harness') -match 'E0425' -and (Log 'missing-harness') -match 'run_controller_crash_matrix') 'Missing test-first negative'
Assert-True ((Log 'nested-origin-failed') -match 'Err.*Invalid' -and (Log 'nested-origin-failed') -match '0 passed; 1 failed') 'Missing origin negative'
Assert-True ((Log 'contracts-linux-git-failed') -match '164 passed; 4 failed') 'Missing Git-dependent contract negative'
Assert-True ((Log 'wsl-git-diagnostic') -match 'fatal: not a git repository:.*C:/Workspace') 'Missing cross-platform Git diagnosis'
Assert-True ((Log 'patch-check') -match 'exact baseline-only index; worktree/index unchanged') 'Probe patch was not checked'
Assert-True ((Log 'trust-construction-failed') -match '68 passed; 1 failed; 1 ignored' -and (Log 'trust-construction-failed') -match 'Input\(Kernel\(Invalid\)\)') 'Missing unexplained original-input negative'
Assert-True ((Log 'trust-isolated') -match '1 passed; 0 failed') 'Missing isolated same-source observation'
$stages = @('BeforeCreate', 'Created', 'PrefixWritten', 'BytesWritten', 'Sealed', 'FileSynced', 'DirectorySynced', 'Acknowledging', 'AfterReturn')
foreach ($name in @('matrix-final-1', 'matrix-final-2', 'matrix-final-3', 'matrix-msrv')) {
    $raw = Log $name
    Assert-True ($raw -match '1 passed; 0 failed; 0 ignored') ('Missing executed matrix: ' + $name)
    Assert-True ($raw -match 'seed=0x0740c095 sigkills=9 controls=1 fresh_inspections=20 production_enrollment=false start=false') 'Wrong matrix summary'
    $records = [regex]::Matches($raw, 'controller_attempt_crash seed=0x0740c095 stage=([A-Za-z]+) sigkill=(true|false) fresh_inspection=([12]) empty=(true|false) unchanged=true')
    Assert-True ($records.Count -eq 20) 'Wrong number of fresh observations'
    foreach ($stage in $stages) {
        foreach ($inspection in @(1, 2)) {
            $empty = if ($stage -ceq 'BeforeCreate') { 'true' } else { 'false' }
            $expected = "stage=$stage sigkill=true fresh_inspection=$inspection empty=$empty unchanged=true"
            Assert-True ([regex]::Matches($raw, [regex]::Escape($expected)).Count -eq 1) ('Missing exact checkpoint: ' + $expected)
        }
    }
    foreach ($inspection in @(1, 2)) {
        $expected = "stage=AfterReturn sigkill=false fresh_inspection=$inspection empty=false unchanged=true"
        Assert-True ([regex]::Matches($raw, [regex]::Escape($expected)).Count -eq 1) 'Missing uninterrupted control'
    }
}
Assert-True ((Log 'trust-final') -match '69 passed; 0 failed; 1 ignored') 'Missing original-input regression'
Assert-True ((Log 'empty-fence') -match '11 passed; 0 failed') 'Missing fence regression'
Assert-True ((Log 'contracts') -match '168 passed; 0 failed') 'Missing static contracts'
Assert-True ((Log 'registry-tests') -match '6 passed; 0 failed') 'Missing registry tests'
Assert-True ((Log 'registry') -match '176 discovered gates, all registered') 'Missing automatic gate inventory'
Assert-True ((Log 'fmt-windows-argv-failed') -match 'os error 206') 'Missing Windows formatter argument limit'
foreach ($platform in @('linux', 'windows')) {
    foreach ($version in @('1.88.0', '1.94.0')) {
        foreach ($kind in @('check', 'clippy')) {
            $name = "$platform-$version-$kind-final"
            $capture = @($m.captures | Where-Object file -CEQ ($name + '.log'))
            Assert-True ($capture.Count -eq 1 -and $capture[0].exit_code -eq 0) ('Missing build lane: ' + $name)
            Assert-True ((Log $name) -notmatch 'corrupt metadata|internal compiler error|panicked at') ('Invalid build lane: ' + $name)
        }
    }
}
foreach ($name in @('fmt', 'fmt-explicit', 'doc-check', 'performance-contract', 'old-packet', 'verifier-syntax')) {
    $capture = @($m.captures | Where-Object file -CEQ ($name + '.log'))
    Assert-True ($capture.Count -eq 1 -and $capture[0].exit_code -eq 0) ('Missing final gate: ' + $name)
}
Assert-True ((Log 'doc-check') -match 'doc-check: OK') 'Missing documentation result'
Assert-True ((Log 'require-ship') -match 'ship admission is closed while candidate identity and release qualification are incomplete') 'Missing expected closed-admission reason'
Write-Output 'Verified three current-Rust and one MSRV matrix: each 9 owned SIGKILL cases, 1 control and 20 fresh inspections.'
Write-Output 'Verified source scope, unchanged runtime/schema, 69 trust tests, 11 fence tests and 168 contracts.'
Write-Output 'Verified 176 automatic gates still covered and exactly one added manual entry, 229 to 230 explicit registrations.'
Write-Output ('Verified {0} captures and all artifact hashes; test-first and origin negatives remain retained.' -f $m.captures.Count)
Write-Output 'Unsigned consistency only; no execution attestation, power-loss/reboot proof, production enrollment, start or server admission.'
