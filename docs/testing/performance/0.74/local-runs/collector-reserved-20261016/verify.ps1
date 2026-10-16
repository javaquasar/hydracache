$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
function Assert-True($ok, [string]$why) { if (-not $ok) { throw $why } }
Assert-True ($m.format_version -eq 1 -and $m.source_commit -ceq '6c49e3f9b4ec0241114f08e023affc29c5917bb2' -and $m.baseline_commit -ceq '13dac632d91bb029821b9f3a2678e13694222130') 'Wrong source'
Assert-True ($m.initial_collector_reserved_proved -eq $true) 'Missing initial retention scope'
foreach ($field in @('starting_proved','root_constructor_added','original_start_proved','all_writer_cessation_proved','server_admitted','performance_claim','clean_tree_build_attested','power_loss_or_reboot_proved')) {
    Assert-True ($m.$field -eq $false) ('Scope must stay closed: ' + $field)
}
$paths = @(
    'crates/xtask/tests/performance_contract_074.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_intent.rs',
    'crates/xtask/tests/support/performance_contract_074_collector_reserved.rs',
    'docs/COMPAT.md',
    'docs/testing/gated-test-registry.toml',
    'docs/testing/performance/0.74/diagnostic-collector-reserved-contract.toml',
    'docs/testing/performance/0.74/diagnostic-collector-reserved-design.md',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved_crash_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_reserved_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_early_attempt_storage.rs',
    'tools/long-run-supervisor-074/src/diagnostic_observed_reservation.rs',
    'tools/long-run-supervisor-074/src/diagnostic_observed_reservation_tests.rs'
)
$frozenPaths = @(
    'Cargo.lock',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'docs/testing/performance/0.74/local-runs/observed-reservation-20261016/manifest.json',
    'tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs',
    'tools/long-run-supervisor-074/src/diagnostic_reservation_publication_storage.rs',
    'tools/long-run-supervisor-074/src/diagnostic_starting_intent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs',
    'tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_intent.rs',
    'tools/long-run-supervisor-074/src/diagnostic_intent_projection.rs',
    'tools/long-run-supervisor-074/src/diagnostic_controller_attempt_crash_tests.rs',
    'tools/long-run-supervisor-074/src/diagnostic_unit.rs',
    'tools/long-run-supervisor-074/src/server.rs'
)
Assert-True ($m.source_files.Count -eq 14 -and @($m.source_files.path | Sort-Object -Unique).Count -eq 14) 'Wrong source inventory'
Assert-True ($m.frozen_files.Count -eq 15 -and @($m.frozen_files.path | Sort-Object -Unique).Count -eq 15) 'Wrong unchanged inventory'
Push-Location $repo
try {
    $tree = & git rev-parse ('{0}^{{tree}}' -f $m.source_commit)
    Assert-True ($LASTEXITCODE -eq 0 -and $tree -ceq $m.source_tree) 'Source tree differs'
    & git merge-base --is-ancestor $m.baseline_commit $m.source_commit
    Assert-True ($LASTEXITCODE -eq 0) 'Wrong source ancestry'
    foreach ($s in $m.source_files) {
        Assert-True ($paths -ccontains $s.path) 'Unexpected source'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit,$s.path)
        Assert-True ($LASTEXITCODE -eq 0 -and $blob -ceq $s.git_blob) 'Source blob differs'
        & git diff --exit-code $m.source_commit -- $s.path
        Assert-True ($LASTEXITCODE -eq 0) ('Current source differs: ' + $s.path)
    }
    $allowed = $paths + 'docs/testing/performance/0.74/diagnostic-collector-reserved-plan-20261016.md'
    $changed = @(& git diff --name-only $m.baseline_commit $m.source_commit)
    Assert-True ($LASTEXITCODE -eq 0 -and $changed.Count -eq 15) 'Wrong changed inventory'
    foreach ($p in $changed) { Assert-True ($allowed -ccontains $p) ('Out-of-scope source change: ' + $p) }
    foreach ($s in $m.frozen_files) {
        Assert-True ($frozenPaths -ccontains $s.path) 'Unexpected unchanged input'
        $blob = & git rev-parse ('{0}:{1}' -f $m.baseline_commit,$s.path)
        Assert-True ($LASTEXITCODE -eq 0 -and $blob -ceq $s.git_blob) 'Unchanged baseline differs'
        $blob = & git rev-parse ('{0}:{1}' -f $m.source_commit,$s.path)
        Assert-True ($LASTEXITCODE -eq 0 -and $blob -ceq $s.git_blob) 'Unchanged source differs'
        Assert-True ((Get-FileHash -LiteralPath $s.path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $s.sha256) ('Unchanged bytes differ: ' + $s.path)
    }
} finally { Pop-Location }
$names = @()
foreach ($a in $m.artifacts) {
    Assert-True ($a.file -cmatch '^[A-Za-z0-9][A-Za-z0-9.-]*$' -and $names -cnotcontains $a.file) 'Unsafe or duplicate artifact'
    $names += $a.file
    $p = Join-Path $packet $a.file
    Assert-True ((Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a.sha256 -and (Get-Item -LiteralPath $p).Length -eq $a.bytes) ('Artifact differs: ' + $a.file)
}
$actual = @(Get-ChildItem -LiteralPath $packet -File | Where-Object Name -CNE 'manifest.json' | ForEach-Object Name | Sort-Object)
Assert-True (($actual -join "`n") -ceq (($names | Sort-Object) -join "`n")) 'Artifact inventory differs'
Assert-True ($m.captures.Count -eq 31 -and @($m.captures.file | Sort-Object -Unique).Count -eq 31 -and $m.artifacts.Count -eq 33) 'Wrong capture count'
foreach ($c in $m.captures) {
    Assert-True ($names -ccontains $c.file -and $c.exit_code -eq $c.expected_exit_code) 'Unexpected capture exit'
}
function Log([string]$name) { Get-Content -LiteralPath (Join-Path $packet ($name + '.log')) -Raw }
Assert-True ((Log 'baseline') -match '6 passed; 0 failed') 'Missing baseline'
Assert-True ((Log 'red') -match 'E0599' -and (Log 'red') -match 'publish_reserved') 'Missing compiler red'
Assert-True ((Log 'first-positive') -match '1 passed; 0 failed' -and (Log 'first-positive') -match 'boundary=Acknowledging start=false') 'Missing first positive'
Assert-True ((Log 'focused-current') -match '10 passed; 0 failed; 1 ignored' -and (Log 'focused-current') -match 'logical_faults=9' -and (Log 'focused-current') -match 'paired_byte_mutations=20') 'Missing focused boundary proof'
foreach ($v in @('1.94.0','1.88.0')) {
    $o = Log "original-$v"
    Assert-True ($o -match '92 passed; 0 failed; 2 ignored' -and $o -match 'seed=0x0740c09a logical_faults=9 root_start=false' -and $o -match 'paired_byte_mutations=20 signature_authentication_by_inspector=false') 'Missing final original-input regression'
    Assert-True ($o -notmatch 'HC074-KERNEL-FIRST-REFUSAL') 'Unexpected original kernel refusal'
    Assert-True ((Log "kernel-$v") -match '26 passed; 0 failed; 1 ignored') 'Missing kernel regression'
    Assert-True ((Log "bytes-$v") -match '30 passed; 0 failed') 'Missing projection regression'
    Assert-True ((Log "doc-$v") -match '15 passed; 0 failed' -and (Log "doc-$v") -match 'compile fail ... ok') 'Missing compile-fail proof'
    Assert-True ((Log "doc-$v") -notmatch 'error\[E(0432|0433|0603)\]') 'Compile-fail canary failed on an import or visibility error'
}
Assert-True ((Log 'contracts') -match '173 passed; 0 failed') 'Missing static suite'
$matrix = Log 'matrix-current'
Assert-True ($matrix -match '1 passed; 0 failed') 'Missing process proof'
$rows = [regex]::Matches($matrix,'collector_reserved_crash seed=0x0740c09b repeat=([123]) stage=([A-Za-z]+) sigkill=(true|false) fresh_inspection=([12]) empty=false unchanged=true')
Assert-True ($rows.Count -eq 66) 'Wrong fresh inspection count'
$keys = @($rows | ForEach-Object { '{0}|{1}|{2}|{3}' -f $_.Groups[1].Value,$_.Groups[2].Value,$_.Groups[3].Value,$_.Groups[4].Value })
Assert-True (@($keys | Sort-Object -Unique).Count -eq 66) 'Duplicate inspection case'
foreach ($repeat in 1..3) {
    Assert-True ($matrix.Contains("collector_reserved_crash_summary seed=0x0740c09b repeat=$repeat sigkills=10 controls=1 fresh_inspections=22 production_enrollment=false start=false")) 'Missing repeat summary'
    foreach ($stage in @('BeforeCreate','Created','CreateDirectorySynced','PrefixWritten','BytesWritten','Sealed','FileSynced','DirectorySynced','Acknowledging','AfterReturn')) {
        foreach ($i in 1..2) { Assert-True ($keys -ccontains "$repeat|$stage|true|$i") 'Missing killed stage inspection' }
    }
    foreach ($i in 1..2) { Assert-True ($keys -ccontains "$repeat|AfterReturn|false|$i") 'Missing control inspection' }
}
foreach ($p in @('linux','windows')) { foreach ($v in @('1.94.0','1.88.0')) { foreach ($kind in @('check','clippy')) {
    $entry = @($m.captures | Where-Object file -CEQ "$p-$v-$kind.log")
    Assert-True ($entry.Count -eq 1 -and $entry[0].exit_code -eq 0) 'Missing final build/lint lane'
} } }
Assert-True ((Log 'registry') -match '176 discovered gates, all registered') 'Automatic discovery changed'
Assert-True ((Log 'require-ship') -match 'ship admission is closed while candidate identity and release qualification are incomplete') 'Missing closed ship admission'
Write-Output 'Verified 31 captures, fourteen source objects, fifteen unchanged inputs and all artifact hashes.'
Write-Output 'Three actual process repeats: thirty SIGKILLs, three controls and sixty-six fresh inspections; BeforeCreate remains refused.'
Write-Output 'Both compilers: 92 original-input, 30 projection, 26 kernel tests and fifteen compile-fail examples; 173 static contracts and eight build/lint lanes.'
Write-Output 'Unsigned local consistency only. Initial collector Reserved retention, not Starting, root outputs, authenticated start or server admission.'
