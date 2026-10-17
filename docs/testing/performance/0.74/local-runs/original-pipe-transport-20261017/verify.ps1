$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$repo = (& git -C $packet rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
function Require($value, [string]$why) { if (-not $value) { throw $why } }
function Count([string]$raw, [string]$pattern, [int]$want) {
    Require ([regex]::Matches($raw, $pattern, [Text.RegularExpressions.RegexOptions]::Multiline).Count -eq $want) "Count mismatch: $pattern"
}
function Faults([string]$raw) {
    foreach ($case in @('entry-fixture','entry-build','entry-refused','message','build','output','trust','fixture','expiry')) {
        Count $raw ("HC074-PIPE-TRANSPORT-REFUSAL case=$case exact=true sticky=true held=true unchanged=true send=false") 1
    }
    Count $raw 'HC074-PIPE-TRANSPORT-REFUSAL case=' 9
    Require $raw.Contains('1 passed; 0 failed;') 'Incomplete fault family'
    $clock = [regex]::Match($raw, 'HC074-PIPE-TRANSPORT-EXPIRY event=Event \{[^\r\n]*site: "progress-age"[^\r\n]*clock: \[(\d+), (\d+), (\d+), 0, 0, (\d+)\][^\r\n]*renewed=false')
    Require $clock.Success 'Original expiry event missing'
    $received = [UInt64]$clock.Groups[1].Value
    $previous = [UInt64]$clock.Groups[2].Value
    $now = [UInt64]$clock.Groups[3].Value
    $limit = [UInt64]$clock.Groups[4].Value
    Require ($limit -eq 10000000000 -and $received -gt $previous -and $now -ge $received -and $now - $received -ge $limit) 'Expiry origin renewed or incorrect'
}
$m = Get-Content -LiteralPath (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
Require ($m.format_version -eq 1 -and $m.baseline_commit -ceq 'fa680f3d4220bb0e49bd730c990ff07b9edfe8fa') 'Unrecognized evidence baseline'
foreach ($key in @('message_preparation_complete','original_guards_unchanged','deadlines_unchanged','admission_closed')) {
    Require (($m.scope.$key -is [bool]) -and $m.scope.$key) "Missing boundary $key"
}
foreach ($key in @('message_sent','authenticated_start','watchdog_armed','writer_handoff','all_writer_cessation','completion','later_cells','production_enrollment','installed_release_changed','supervisor_stopped','clock_changed','product_workload','qualification_changed','performance_claim','server_ready')) {
    Require (($m.scope.$key -is [bool]) -and -not $m.scope.$key) "Invalid admission $key"
}
$actual = @(Get-ChildItem -LiteralPath $packet -File -Recurse | ForEach-Object { $_.FullName.Substring($packet.Length + 1).Replace('\','/') } | Where-Object { $_ -ne 'manifest.json' } | Sort-Object)
$expected = @($m.artifacts | ForEach-Object { $_.path } | Sort-Object)
Require (($actual -join "`n") -ceq ($expected -join "`n")) 'Artifact inventory differs'
foreach ($a in $m.artifacts) {
    Require (-not [IO.Path]::IsPathRooted($a.path) -and $a.path -notmatch '(^|/)\.\.(/|$)') 'Unsafe artifact path'
    $p = Join-Path $packet $a.path
    Require ((Get-Item -LiteralPath $p).Length -eq $a.bytes) "Artifact size: $($a.path)"
    Require ((Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $a.sha256) "Artifact hash: $($a.path)"
}
foreach ($s in $m.source_blobs) {
    $actual = (& git -C $repo hash-object $s.path).Trim()
    Require ($LASTEXITCODE -eq 0 -and $actual -ceq $s.git_blob) "Active source: $($s.path)"
    $actual = (& git -C $repo rev-parse ($m.implementation_commit + ':' + $s.path)).Trim()
    Require ($LASTEXITCODE -eq 0 -and $actual -ceq $s.git_blob) "Committed source: $($s.path)"
}
$frozenRaw = (& git -C $repo show 'fa680f3d:docs/testing/performance/0.74/local-runs/original-collector-sinks-join-20261017/unchanged-runtime.json') -join "`n"
Require ($LASTEXITCODE -eq 0) 'Frozen leaf inventory unavailable'
foreach ($s in ($frozenRaw | ConvertFrom-Json)) {
    $actual = (& git -C $repo hash-object $s.path).Trim()
    Require ($LASTEXITCODE -eq 0 -and $actual -ceq $s.git_blob) "Original leaf changed: $($s.path)"
}
foreach ($p in @('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml','tools/long-run-supervisor-074/src/systemd_unit.rs','tools/long-run-supervisor-074/src/systemd_spawn.rs','tools/long-run-supervisor-074/src/diagnostic_unit.rs','tools/long-run-supervisor-074/src/diagnostic_collector_intent.rs','tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs','tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes.rs','tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs','tools/long-run-supervisor-074/src/diagnostic_original_collector_sink_storage.rs','tools/long-run-supervisor-074/src/diagnostic_original_output_join_crash_tests.rs','tools/long-run-supervisor-074/src/diagnostic_original_output_join_fault_tests.rs')) {
    $expected = (& git -C $repo rev-parse ('fa680f3d:' + $p)).Trim()
    Require ($LASTEXITCODE -eq 0) "Frozen source unavailable: $p"
    $actual = (& git -C $repo hash-object $p).Trim()
    Require ($LASTEXITCODE -eq 0 -and $actual -ceq $expected) "Frozen source changed: $p"
}
foreach ($lane in @('194','188')) {
    $binaries = @($m.binaries | Where-Object { $_.lane -ceq $lane })
    Require ($binaries.Count -eq 1) 'Binary lane missing or duplicated'
    $b = $binaries[0]
    $run = [IO.File]::ReadAllText((Join-Path $packet "run-$lane.log"))
    Count $run ("HC074-PIPE-TRANSPORT-BINARY lane=$lane bytes=" + $b.bytes + ' sha256=' + $b.sha256) 1
    $healthy = [IO.File]::ReadAllText((Join-Path $packet "healthy-$lane.log"))
    Require $healthy.Contains('1 passed; 0 failed;') 'Incomplete healthy family'
    foreach ($sample in 0..2) {
        Count $healthy ("HC074-PIPE-TRANSPORT-CONTROL sample=$sample original=true held=true unchanged=true send=false") 1
        foreach ($phase in @('prepare','validate')) {
            Count $healthy ("HC074-PIPE-TRANSPORT-COST sample=$sample phase=$phase [^\r\n]*authentication_calls: 34,") 1
        }
    }
    Count $healthy 'HC074-PIPE-TRANSPORT-CONTROL sample=' 3
    $faults = [IO.File]::ReadAllText((Join-Path $packet "refusals-$lane.log"))
    Faults $faults
    $missing = [regex]::Match($faults, 'HC074-PIPE-TRANSPORT-REFUSAL case=[^\r\n]+').Value
    $rejected = $false
    try { Faults ($faults.Replace($missing,'')) } catch { $rejected = $true }
    Require $rejected 'Missing case canary survived'
    $local = [IO.File]::ReadAllText((Join-Path $packet "local-$lane.log"))
    Require $local.Contains("HC074-PIPE-LOCAL lane=$lane completed=true") 'Local checks incomplete'
    $final = [IO.File]::ReadAllText((Join-Path $packet "final-$lane.log"))
    Require $final.Contains("HC074-PIPE-FINAL lane=$lane completed=true") 'Final source checks incomplete'
}
Require ([IO.File]::ReadAllText((Join-Path $packet 'contracts.log')).Contains('190 passed; 0 failed;')) 'Static contracts incomplete'
Require ([IO.File]::ReadAllText((Join-Path $packet 'registry.log')).Contains('all registered')) 'Registry incomplete'
Require ([IO.File]::ReadAllText((Join-Path $packet 'ship-guard-expected-red.log')).Contains('0.74 ship admission is closed while candidate identity and release qualification are incomplete')) 'Ship refusal missing'
Require ([IO.File]::ReadAllText((Join-Path $packet 'prior-proof.log')).Contains('Prior material proof verified at exact immutable source fa680f3d4220bb0e49bd730c990ff07b9edfe8fa')) 'Prior source proof missing'
Require ([IO.File]::ReadAllText((Join-Path $packet 'postflight.log')).Contains('test_processes=absent supervisor_unchanged=true installed_receipt_unchanged=true active_contexts=absent clock_changed=false product=false')) 'Independent host postflight missing'
& git -C $repo diff --exit-code $m.baseline_commit -- docs/testing/performance/0.74/qualification-manifest.toml
Require ($LASTEXITCODE -eq 0) 'Qualification inputs changed'
Write-Output 'Original unsent pipe message verified: 6 healthy chains, 18 exact refusal cases; original deadlines/guards intact; send, watchdog and production admission CLOSED'
