$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Assert-True($condition, [string]$message) { if (-not $condition) { throw $message } }
Assert-True ($m.format_version -eq 1 -and $m.baseline_commit -ceq '914aa749369a78a57ae8aebb8e95c8a878366fbf') 'Wrong packet baseline'
Assert-True ($m.source_commit -cmatch '^[a-f0-9]{40}$') 'Unsafe source identity'
Assert-True ($m.historical_cause_proved -eq $false -and $m.server_admitted -eq $false -and $m.performance_claim -eq $false) 'Scope must remain closed'
$paths = @(
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_refusal_trace.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_fence_preflight_tests.rs',
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_kernel_refusal.rs',
    'docs/testing/performance/0.74/diagnostic-kernel-refusal-attribution-contract.toml'
)
Assert-True ($m.source_files.Count -eq $paths.Count -and @($m.source_files.path | Sort-Object -Unique).Count -eq $paths.Count) 'Wrong source inventory'
Push-Location $repo
try {
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.source_commit)
    Assert-True ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Source tree differs'
    foreach ($source in $m.source_files) {
        Assert-True ($paths -ccontains $source.path) 'Unexpected source path'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit, $source.path)
        Assert-True ($LASTEXITCODE -eq 0 -and $blob -ceq $source.git_blob) ('Source blob differs: ' + $source.path)
    }
    $allowed = $paths + 'docs/testing/performance/0.74/diagnostic-kernel-refusal-attribution-plan-20261016.md'
    foreach ($path in @(& git diff --name-only $m.baseline_commit $m.source_commit)) {
        Assert-True ($allowed -ccontains $path) ('Unreviewed source change: ' + $path)
    }
    foreach ($path in @('Cargo.lock', 'docs/COMPAT.md', 'docs/testing/gated-test-registry.toml',
        'docs/testing/performance/0.74/qualification-manifest.toml',
        'tools/long-run-supervisor-074/src/diagnostic_kernel_ancillary.rs',
        'tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs',
        'tools/long-run-supervisor-074/src/diagnostic_early_attempt_storage.rs',
        'tools/long-run-supervisor-074/src/server.rs')) {
        $before = & git rev-parse ('{0}:{1}' -f $m.baseline_commit, $path)
        Assert-True ($LASTEXITCODE -eq 0) 'Missing baseline object'
        $after = & git rev-parse ('{0}:{1}' -f $m.source_commit, $path)
        Assert-True ($LASTEXITCODE -eq 0 -and $before -ceq $after) ('Out-of-scope change: ' + $path)
    }
} finally { Pop-Location }
$frozenPaths = @('Cargo.lock', 'docs/COMPAT.md', 'docs/testing/performance/0.74/qualification-manifest.toml',
    'docs/testing/performance/0.74/local-runs/controller-attempt-crash-20261016/manifest.json')
Assert-True ($m.frozen_files.Count -eq 4 -and @($m.frozen_files.path | Sort-Object -Unique).Count -eq 4) 'Wrong frozen inventory'
foreach ($frozen in $m.frozen_files) {
    Assert-True ($frozenPaths -ccontains $frozen.path) 'Unexpected frozen file'
    Assert-True ((Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $repo $frozen.path)).Hash.ToLowerInvariant() -ceq $frozen.sha256) ('Frozen input differs: ' + $frozen.path)
}
$names = @()
foreach ($artifact in $m.artifacts) {
    Assert-True ($artifact.file -cmatch '^[A-Za-z0-9][A-Za-z0-9.-]*$' -and $names -cnotcontains $artifact.file) 'Unsafe or duplicate artifact'
    $names += $artifact.file
    $file = Join-Path $packet $artifact.file
    Assert-True ((Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLowerInvariant() -ceq $artifact.sha256 -and (Get-Item -LiteralPath $file).Length -eq $artifact.bytes) ('Artifact differs: ' + $artifact.file)
}
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object Name -CNE 'manifest.json' | ForEach-Object Name | Sort-Object)
Assert-True (($actual -join "`n") -ceq (($names | Sort-Object) -join "`n")) 'Artifact inventory differs'
Assert-True (@($m.captures.file | Sort-Object -Unique).Count -eq $m.captures.Count) 'Duplicate capture'
foreach ($capture in $m.captures) {
    Assert-True ($names -ccontains $capture.file -and $capture.exit_code -eq $capture.expected_exit_code) ('Unexpected capture: ' + $capture.file)
}
function Log([string]$name) { Get-Content -LiteralPath (Join-Path $packet ($name + '.log')) -Raw }
Assert-True ((Log 'kernel-baseline') -match '19 passed; 0 failed; 1 ignored') 'Missing kernel baseline'
Assert-True ((Log 'missing-capture') -match 'E0433' -and (Log 'missing-capture') -match 'refusal_trace') 'Missing test-first negative'
Assert-True ((Log 'patch-check') -match 'exact baseline; real index/worktree unchanged') 'Unverified probe patch'
Assert-True ((Log 'equivalence-layout-failed') -match 'unexpected guard, ordering, budget or API change') 'Missing layout negative'
Assert-True ((Log 'equivalence-count-failed') -match 'left: \(32, 3\)' -and (Log 'equivalence-count-failed') -match 'right: \(33, 3\)') 'Missing count correction'
Assert-True ((Log 'clippy-lazy-failed') -match 'unnecessary_lazy_evaluations') 'Missing non-test lazy-constructor negative'
foreach ($version in @('1.94.0', '1.88.0')) {
    Assert-True ((Log "kernel-$version") -match '25 passed; 0 failed; 1 ignored') 'Missing kernel regression'
    Assert-True ((Log "kernel-$version") -match 'seed=0x0740c096.*historical_cause_proved=false') 'Missing classifier boundary'
}
foreach ($name in @('trust-1', 'trust-2', 'trust-3', 'trust-msrv')) {
    Assert-True ((Log $name) -match '69 passed; 0 failed; 1 ignored') ('Missing bounded original-input repetition: ' + $name)
    Assert-True ((Log $name) -notmatch 'HC074-KERNEL-FIRST-REFUSAL') ('Unexpected original fixture refusal: ' + $name)
}
foreach ($name in @('matrix-current', 'matrix-msrv')) {
    $raw = Log $name
    Assert-True ($raw -match '1 passed; 0 failed; 0 ignored') 'Missing executed joined process matrix'
    Assert-True ($raw -match 'sigkills=9 controls=1 fresh_inspections=20 production_enrollment=false start=false') 'Wrong joined matrix summary'
    Assert-True ([regex]::Matches($raw, 'controller_attempt_crash seed=0x0740c095 stage=[A-Za-z]+ sigkill=(true|false) fresh_inspection=[12] empty=(true|false) unchanged=true').Count -eq 20) 'Missing unchanged fresh observations'
}
Assert-True ((Log 'contracts') -match '169 passed; 0 failed') 'Missing static gates'
foreach ($name in @('sender-current', 'sender-msrv')) {
    $raw = Log $name
    Assert-True ($raw -match '1 passed; 0 failed; 0 ignored' -and $raw -match 'actual_sigkill=1 exited_sender=refused parent_sender=still_live') 'Missing actual sender-death check'
}
Assert-True ((Log 'equivalence-final') -match 'sites=32 lazy=3 guards_unchanged=true historical_cause_proved=false') 'Missing final source equivalence'
foreach ($platform in @('linux', 'windows')) {
    foreach ($version in @('1.94.0', '1.88.0')) {
        foreach ($kind in @('check', 'clippy')) {
            $name = "$platform-$version-$kind"
            $entry = @($m.captures | Where-Object file -CEQ ($name + '.log'))
            Assert-True ($entry.Count -eq 1 -and $entry[0].exit_code -eq 0) ('Missing final build lane: ' + $name)
        }
    }
}
foreach ($name in @('fmt', 'fmt-explicit', 'registry', 'doc-check', 'performance-contract', 'old-packet')) {
    $entry = @($m.captures | Where-Object file -CEQ ($name + '.log'))
    Assert-True ($entry.Count -eq 1 -and $entry[0].exit_code -eq 0) ('Missing local final gate: ' + $name)
}
Assert-True ((Log 'registry') -match '176 discovered gates, all registered') 'Automatic registry differs'
Assert-True ((Log 'doc-check') -match 'doc-check: OK') 'Documentation gate missing'
Assert-True ((Log 'require-ship') -match 'ship admission is closed while candidate identity and release qualification are incomplete') 'Expected admission refusal missing'
Write-Output 'Verified test-only classifier, source objects and frozen inputs; 25 kernel tests on each compiler and four 69-case original-input repetitions.'
Write-Output 'Verified two joined SIGKILL matrices, 169 contracts, eight build/lint lanes and the unchanged automatic registry.'
Write-Output ('Verified {0} command captures and all artifact hashes; development negatives remain retained.' -f $m.captures.Count)
Write-Output 'Unsigned consistency only; historical Kernel(Invalid) remains unexplained, with no server, original-start or performance admission.'
