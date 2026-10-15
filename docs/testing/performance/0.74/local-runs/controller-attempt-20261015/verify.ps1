$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Assert-True($condition, [string]$message) {
    if (-not $condition) { throw $message }
}
Assert-True ($m.format_version -eq 1) 'Unknown manifest version'
Assert-True ($m.source_commit -cmatch '^[a-f0-9]{40}$') 'Unsafe source identity'
Assert-True ($m.baseline_commit -cmatch '^[a-f0-9]{40}$') 'Unsafe baseline identity'
Assert-True (-not $m.server_admitted -and -not $m.joined_sigkill_proved -and -not $m.performance_claim) 'Scope must stay closed'
$sourcePaths = @(
    'tools/long-run-supervisor-074/src/diagnostic_controller_fence_preflight.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_fence_preflight_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_attempt_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_early_attempt_storage.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs',
    'crates/xtask/tests/performance_contract_074.rs',
    'docs/COMPAT.md',
    'docs/testing/performance/0.74/diagnostic-controller-attempt-contract.toml'
)
Assert-True ($m.source_files.Count -eq $sourcePaths.Count) 'Wrong source inventory'
Assert-True (@($m.source_files.path | Sort-Object -Unique).Count -eq $sourcePaths.Count) 'Duplicate source path'
Push-Location $repo
try {
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.source_commit)
    Assert-True ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Source tree differs'
    foreach ($source in $m.source_files) {
        Assert-True ($sourcePaths -ccontains $source.path) 'Unexpected source path'
        Assert-True ($source.git_blob -cmatch '^[a-f0-9]{40}$') 'Unsafe source blob'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit, $source.path)
        Assert-True ($LASTEXITCODE -eq 0 -and $blob -ceq $source.git_blob) ('Source differs: ' + $source.path)
    }
    $changed = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Assert-True ($LASTEXITCODE -eq 0) 'Cannot inspect historical source changes'
    $allowed = $sourcePaths + 'docs/testing/performance/0.74/diagnostic-controller-attempt-plan-20261015.md'
    foreach ($path in $changed) { Assert-True ($allowed -ccontains $path) ('Unreviewed source change: ' + $path) }
} finally { Pop-Location }
$frozenPaths = @('Cargo.lock', 'docs/testing/performance/0.74/qualification-manifest.toml', 'docs/testing/performance/0.74/local-runs/controller-fence-preflight-20261015/manifest.json')
Assert-True ($m.frozen_files.Count -eq 3) 'Wrong frozen inventory'
Assert-True (@($m.frozen_files.path | Sort-Object -Unique).Count -eq 3) 'Duplicate frozen path'
foreach ($frozen in $m.frozen_files) {
    Assert-True ($frozenPaths -ccontains $frozen.path) 'Unexpected frozen path'
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $repo $frozen.path)).Hash.ToLowerInvariant()
    Assert-True ($hash -ceq $frozen.sha256) ('Frozen input differs: ' + $frozen.path)
}
$names = @()
foreach ($artifact in $m.artifacts) {
    Assert-True ($artifact.file -cmatch '^[A-Za-z0-9][A-Za-z0-9.-]*$') 'Unsafe artifact name'
    Assert-True ($names -cnotcontains $artifact.file) 'Duplicate artifact'
    $names += $artifact.file
    $file = Join-Path $packet $artifact.file
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLowerInvariant()
    Assert-True ($hash -ceq $artifact.sha256) ('Artifact differs: ' + $artifact.file)
    Assert-True ((Get-Item -LiteralPath $file).Length -eq $artifact.bytes) ('Size differs: ' + $artifact.file)
}
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object { $_.Name -ne 'manifest.json' } | ForEach-Object Name | Sort-Object)
Assert-True (($actual -join "`n") -ceq (($names | Sort-Object) -join "`n")) 'Artifact inventory differs'
foreach ($capture in $m.captures) {
    Assert-True ($names -ccontains $capture.file) 'Uninventoried capture'
    Assert-True ($capture.exit_code -eq $capture.expected_exit_code) ('Unexpected command status: ' + $capture.file)
}
function Log([string]$name) { Get-Content -LiteralPath (Join-Path $packet ($name + '.log')) -Raw }
Assert-True ((Log 'baseline') -match '8 passed; 0 failed') 'Baseline missing'
Assert-True ((Log 'missing-method') -match 'E0599' -and (Log 'missing-method') -match 'retain_request') 'Test-first negative missing'
Assert-True ((Log 'test-first-patch-check') -match 'exact baseline-only index; worktree/index unchanged') 'Reconstructable baseline probe missing'
Assert-True ((Log 'focused-development-failed') -match '7 passed; 1 failed') 'Fixture negative missing'
Assert-True ((Log 'linux-1.94.0-clippy-before-clean') -match 'corrupt metadata|corrupt incremental' -and (Log 'linux-1.94.0-clippy-before-clean') -match 'panicked') 'Cache/compiler negative missing'
Assert-True ((Log 'cache-inventory') -match '12977255 regular file 2' -and (Log 'cache-inventory') -match '00 00 00 00') 'Independent metadata inspection missing'
Assert-True ((Log 'cache-clean') -match 'Removed 19616 files, 12.1GiB') 'Scoped cache cleanup missing'
Assert-True ((Log 'trust-final') -match '67 passed; 0 failed') 'Trust regression missing'
foreach ($name in @('focused-final-1', 'focused-final-2', 'focused-final-3', 'focused-msrv')) {
    Assert-True ((Log $name) -match '10 passed; 0 failed') ('Final focused suite missing: ' + $name)
    Assert-True ((Log $name) -match 'logical_boundaries=8 sigkill_proof=false') 'Logical/SIGKILL boundary blurred'
}
foreach ($name in @('doctest-current', 'doctest-msrv')) {
    $raw = Log $name
    Assert-True ($raw -match '7 passed; 0 failed') ('API examples missing: ' + $name)
    Assert-True ($raw -match 'E0616' -and $raw -match 'E0382') 'Expected ordinary API errors missing'
    foreach ($code in @('E0599', 'E0277', 'E0061')) { Assert-True ($raw -match $code) ('Missing API diagnostic: ' + $code) }
    Assert-True ($raw -notmatch 'E0432|E0433|E0463') 'Unresolved import is not an API proof'
    Assert-True ($raw -notmatch 'internal compiler error|panicked at') 'Compiler panic is not API proof'
}
foreach ($platform in @('linux', 'windows')) {
    foreach ($version in @('1.88.0', '1.94.0')) {
        foreach ($kind in @('check', 'clippy')) {
            $name = "$platform-$version-$kind"
            $entry = @($m.captures | Where-Object file -CEQ ($name + '.log'))
            Assert-True ($entry.Count -eq 1 -and $entry[0].exit_code -eq 0) ('Build lane missing: ' + $name)
            Assert-True ((Log $name) -notmatch 'corrupt incremental|corrupt metadata|internal compiler error|panicked at') ('Invalid build lane: ' + $name)
        }
    }
}
Assert-True ((Log 'contracts') -match '167 passed; 0 failed') 'Static contracts missing'
Assert-True ((Log 'registry-tests') -match '6 passed; 0 failed') 'Registry tests missing'
Assert-True ((Log 'registry') -match '176') 'Gate inventory changed'
Assert-True ((Log 'intent-regression') -match '124 passed; 0 failed; 9 ignored') 'Legacy intent regression missing'
Assert-True ((Log 'empty-fence') -match '11 passed; 0 failed') 'Empty-fence regression missing'
Assert-True ((Log 'legacy-process-regression') -match '9 passed; 0 failed') 'Legacy owned-process regression missing'
Assert-True ((Log 'require-ship') -match 'not|missing|fail|closed|unresolved') 'Expected closed ship check missing'
Write-Output ('Verified {0} command captures, source tree/nine blobs, scoped changes and frozen inputs.' -f $m.captures.Count)
Write-Output 'Verified 67 trust tests, three 10-case focused repeats and 10 MSRV cases; 7 ordinary compile-fail examples on each compiler.'
Write-Output 'Verified legacy intent/empty-fence regression, eight build/lint lanes, 167 contracts and the unchanged 176-gate registry.'
Write-Output 'Missing API, fixture failure and corrupt-cache/compiler failure remain retained; cleanup was scoped to three rebuildable package caches.'
Write-Output 'Unsigned consistency only. Joined SIGKILL/lost-ack, production enrollment, original-start/all-writer cessation and server admission remain open.'
